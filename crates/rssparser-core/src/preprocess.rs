//! Byte-level pre-cleanup to make real-world feeds parseable.
//!
//! Ported from the legacy Python `_clean_feed_bytes`, `_fix_malformed_xml_bytes`,
//! `_detect_xml_encoding`, and `_prepare_xml_bytes` helpers.

use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
use once_cell::sync::Lazy;
use regex::bytes::Regex;

use crate::error::ParseError;

static RE_XML_DECL_ENCODING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(<\?xml[^>]*encoding=["'])([^"']+)(["'][^>]*\?>)"#).unwrap()
});
static RE_DOUBLE_XML_DECL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<\?xml\?xml\s+").unwrap());
static RE_DOUBLE_CLOSE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\?\?>\s*").unwrap());
static RE_UNQUOTED_ATTR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(\s+[\w:]+)=([^\s>"']+)"#).unwrap());
static RE_UTF16_ENCODING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(<\?xml[^>]*encoding=["'])utf-16(-le|-be)?(["'][^>]*\?>)"#).unwrap()
});

/// Detect the charset of the feed bytes, decode to UTF-8, and apply byte-level fixups.
/// Returns the cleaned UTF-8 byte buffer ready to hand to the XML parser.
pub fn prepare(input: &[u8]) -> Result<Vec<u8>, ParseError> {
    if input.is_empty() {
        return Err(ParseError::NotAFeed);
    }

    let cleaned_bytes = clean_feed_bytes(input)?;

    // Decode to UTF-8 using the detected encoding (from BOM or XML decl).
    let (utf8_bytes, actual_encoding_name) = decode_to_utf8(&cleaned_bytes);

    // Fix malformed XML syntax that quick-xml can't recover on its own.
    let needs_fix = preview_needs_fix(&utf8_bytes, actual_encoding_name);
    let out = if needs_fix {
        fix_malformed_bytes(&utf8_bytes, actual_encoding_name)
    } else {
        utf8_bytes
    };

    Ok(out)
}

fn clean_feed_bytes(input: &[u8]) -> Result<Vec<u8>, ParseError> {
    // Skip leading whitespace but preserve BOM position check.
    let stripped = strip_leading_whitespace(input);
    let (after_bom, _) = strip_utf8_bom(stripped);

    let preview: Vec<u8> = after_bom[..after_bom.len().min(2000)]
        .iter()
        .map(|b| b.to_ascii_lowercase())
        .collect();

    if preview.starts_with(b"<?xml")
        || preview.starts_with(b"<rss")
        || preview.starts_with(b"<feed")
        || preview.starts_with(b"<rdf")
    {
        return Ok(stripped.to_vec());
    }

    if preview.starts_with(b"<!doctype html") || preview.starts_with(b"<html") {
        return Err(ParseError::NotAFeed);
    }

    // Scan first 8KB for an XML start pattern.
    let search_limit = input.len().min(8192);
    let chunk: Vec<u8> = input[..search_limit]
        .iter()
        .map(|b| b.to_ascii_lowercase())
        .collect();
    let patterns: &[&[u8]] = &[
        b"<?xml",
        b"<rss",
        b"<feed",
        b"<rdf:rdf",
        b"<?xml-stylesheet",
    ];
    let mut earliest = None;
    for pat in patterns {
        if let Some(idx) = find_bytes(&chunk, pat) {
            earliest = match earliest {
                Some(e) if e <= idx => Some(e),
                _ => Some(idx),
            };
        }
    }
    if let Some(start) = earliest {
        return Ok(input[start..].to_vec());
    }

    if find_bytes(&preview, b"<script>").is_some() || find_bytes(&preview, b"<body>").is_some() {
        return Err(ParseError::NotAFeed);
    }

    Ok(input.to_vec())
}

fn strip_leading_whitespace(input: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < input.len() && input[i].is_ascii_whitespace() {
        i += 1;
    }
    &input[i..]
}

fn strip_utf8_bom(input: &[u8]) -> (&[u8], bool) {
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (&input[3..], true)
    } else {
        (input, false)
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

/// Decode possibly-non-UTF-8 input to UTF-8 bytes.
/// Returns the UTF-8 bytes and the effective encoding name (for downstream fixups).
fn decode_to_utf8(input: &[u8]) -> (Vec<u8>, &'static str) {
    // BOM-based detection wins outright.
    if input.starts_with(&[0xFF, 0xFE]) {
        let (decoded, _, _) = UTF_16LE.decode(&input[2..]);
        return (decoded.into_owned().into_bytes(), "utf-16");
    }
    if input.starts_with(&[0xFE, 0xFF]) {
        let (decoded, _, _) = UTF_16BE.decode(&input[2..]);
        return (decoded.into_owned().into_bytes(), "utf-16");
    }
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (input[3..].to_vec(), "utf-8");
    }

    // If bytes are already valid UTF-8, trust that regardless of what the
    // XML declaration says. Lots of feeds mis-declare iso-8859-1 / utf-16
    // while emitting UTF-8.
    if std::str::from_utf8(input).is_ok() {
        return (input.to_vec(), "utf-8");
    }

    // Bytes aren't valid UTF-8 — trust the declaration for transcoding.
    let header_limit = input.len().min(2000);
    let declared = RE_XML_DECL_ENCODING
        .captures(&input[..header_limit])
        .and_then(|c| c.get(2))
        .map(|m| String::from_utf8_lossy(m.as_bytes()).to_ascii_lowercase());

    if let Some(name) = declared {
        if name.starts_with("utf-16") {
            let enc = if name == "utf-16be" { UTF_16BE } else { UTF_16LE };
            let (decoded, _, _) = enc.decode(input);
            return (decoded.into_owned().into_bytes(), "utf-16");
        }
        if let Some(enc) = Encoding::for_label(name.as_bytes()) {
            if enc == UTF_8 {
                return (input.to_vec(), "utf-8");
            }
            let (decoded, _, _) = enc.decode(input);
            return (decoded.into_owned().into_bytes(), "utf-8");
        }
    }

    (input.to_vec(), "utf-8")
}

fn preview_needs_fix(input: &[u8], actual_encoding: &str) -> bool {
    let preview_len = input.len().min(1000);
    let preview: Vec<u8> = input[..preview_len]
        .iter()
        .map(|b| b.to_ascii_lowercase())
        .collect();
    find_bytes(&preview, b"?xml?xml").is_some()
        || find_bytes(&preview, b"??>").is_some()
        || (find_bytes(&preview, b"rss:").is_some()
            && find_bytes(&input[..input.len().min(2000)].to_ascii_lowercase_vec(), b"xmlns:rss")
                .is_none())
        || (find_bytes(&preview, b"utf-16").is_some() && actual_encoding != "utf-16")
}

trait ToAsciiLowercaseVec {
    fn to_ascii_lowercase_vec(&self) -> Vec<u8>;
}
impl ToAsciiLowercaseVec for [u8] {
    fn to_ascii_lowercase_vec(&self) -> Vec<u8> {
        self.iter().map(|b| b.to_ascii_lowercase()).collect()
    }
}

fn fix_malformed_bytes(input: &[u8], actual_encoding: &str) -> Vec<u8> {
    // Only the first 2 KB can legally contain an XML declaration; run declaration-only
    // patterns there for speed.
    let split = input.len().min(2048);
    let (header, tail) = input.split_at(split);

    let mut header = RE_DOUBLE_XML_DECL
        .replace_all(header, &b"<?xml "[..])
        .into_owned();
    header = RE_DOUBLE_CLOSE
        .replace_all(&header, &b"?>"[..])
        .into_owned();

    if actual_encoding != "utf-16" {
        let replacement_tail = format!(r"${{1}}{}${{3}}", actual_encoding);
        header = RE_UTF16_ENCODING
            .replace_all(&header, replacement_tail.as_bytes())
            .into_owned();
    }

    let mut out = header;
    out.extend_from_slice(tail);

    // Rss:version=2.0 and similar unquoted-attr patterns can appear anywhere.
    RE_UNQUOTED_ATTR
        .replace_all(&out, &br#"$1="$2""#[..])
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_simple_utf8() {
        let out = prepare(b"<?xml version=\"1.0\"?><rss><channel/></rss>").unwrap();
        assert!(out.starts_with(b"<?xml"));
    }

    #[test]
    fn fixes_double_xml_decl() {
        let input = br#"<?xml?xml version="1.0" encoding="UTF-8" ??><rss version="2.0"><channel/></rss>"#;
        let out = prepare(input).unwrap();
        assert!(!out.starts_with(b"<?xml?xml"));
        assert!(find_bytes(&out, b"<?xml ").is_some());
        assert!(find_bytes(&out, b"??>").is_none());
    }

    #[test]
    fn fixes_unquoted_attr() {
        let input = br#"<?xml version="1.0"?><rss rss:version=2.0><channel/></rss>"#;
        let out = prepare(input).unwrap();
        assert!(find_bytes(&out, br#"rss:version="2.0""#).is_some());
    }

    #[test]
    fn handles_utf16_mislabeled_as_utf8() {
        // Declares UTF-16 but bytes are UTF-8 — should be treated as UTF-8.
        let input = b"<?xml version=\"1.0\" encoding=\"UTF-16\"?>\n<rss version=\"2.0\"><channel/></rss>";
        let out = prepare(input).unwrap();
        // Encoding attr rewritten to utf-8.
        assert!(find_bytes(&out, b"encoding=\"utf-8\"").is_some()
            || find_bytes(&out, b"encoding='utf-8'").is_some());
    }

    #[test]
    fn rejects_html() {
        let err = prepare(b"<!DOCTYPE html><html><body/></html>").unwrap_err();
        matches!(err, ParseError::NotAFeed);
    }
}
