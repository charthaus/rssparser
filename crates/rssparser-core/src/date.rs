use chrono::{DateTime, FixedOffset, Utc};

pub fn normalize(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(dt) = parse(trimmed) {
        return Some(format_utc(dt));
    }

    // Fallback fixups. Mirror the _RE_* regexes from the legacy parser.
    if let Some(fixed) = fix_iso_variants(trimmed) {
        if let Some(dt) = parse(&fixed) {
            return Some(format_utc(dt));
        }
    }
    if let Some(fixed) = fix_rfc822_variants(trimmed) {
        if let Some(dt) = parse(&fixed) {
            return Some(format_utc(dt));
        }
    }

    // Give up — return the raw string so callers still see something.
    Some(trimmed.to_string())
}

fn parse(s: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_rfc2822(s))
        .ok()
}

fn format_utc(dt: DateTime<FixedOffset>) -> String {
    dt.with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%S+00:00")
        .to_string()
}

fn fix_rfc822_variants(s: &str) -> Option<String> {
    if !has_timezone(s) {
        return Some(format!("{s} +0000"));
    }
    None
}

fn fix_iso_variants(s: &str) -> Option<String> {
    if let Some(fixed) = fix_hour_24(s) {
        return Some(fixed);
    }
    if let Some(fixed) = fix_tz_no_colon(s) {
        return Some(fixed);
    }
    if let Some(fixed) = fix_tz_hour_only(s) {
        return Some(fixed);
    }
    if let Some(fixed) = fix_fraction(s) {
        return Some(fixed);
    }
    if !has_timezone(s) && s.contains('T') {
        return Some(format!("{s}Z"));
    }
    None
}

fn has_timezone(s: &str) -> bool {
    if s.ends_with('Z') || s.ends_with('z') {
        return true;
    }
    let bytes = s.as_bytes();
    for i in (bytes.len().saturating_sub(6)..bytes.len()).rev() {
        if bytes[i] == b'+' || bytes[i] == b'-' {
            if i > 0 && (bytes[i - 1].is_ascii_digit() || bytes[i - 1] == b' ') {
                return true;
            }
        }
    }
    let upper_tail = s
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>();
    upper_tail.len() >= 3
}

fn fix_hour_24(s: &str) -> Option<String> {
    if let Some(idx) = s.find("T24:") {
        let (head, tail) = s.split_at(idx);
        let rest = &tail[4..];
        return Some(format!("{head}T00:{rest}"));
    }
    if let Some(idx) = s.find(" 24:") {
        let (head, tail) = s.split_at(idx);
        let rest = &tail[4..];
        return Some(format!("{head} 00:{rest}"));
    }
    None
}

fn fix_tz_no_colon(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.len() < 5 {
        return None;
    }
    let tail = &bytes[bytes.len() - 5..];
    if (tail[0] == b'+' || tail[0] == b'-')
        && tail[1].is_ascii_digit()
        && tail[2].is_ascii_digit()
        && tail[3].is_ascii_digit()
        && tail[4].is_ascii_digit()
    {
        let head = &s[..s.len() - 5];
        let sign = tail[0] as char;
        let hh = std::str::from_utf8(&tail[1..3]).ok()?;
        let mm = std::str::from_utf8(&tail[3..5]).ok()?;
        return Some(format!("{head}{sign}{hh}:{mm}"));
    }
    None
}

fn fix_tz_hour_only(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.len() < 3 {
        return None;
    }
    let tail = &bytes[bytes.len() - 3..];
    if (tail[0] == b'+' || tail[0] == b'-')
        && tail[1].is_ascii_digit()
        && tail[2].is_ascii_digit()
    {
        let before = bytes.len().checked_sub(4).map(|i| bytes[i]).unwrap_or(0);
        if before.is_ascii_digit() || before == b':' {
            let head = &s[..s.len() - 3];
            let sign = tail[0] as char;
            let hh = std::str::from_utf8(&tail[1..3]).ok()?;
            return Some(format!("{head}{sign}{hh}:00"));
        }
    }
    None
}

fn fix_fraction(s: &str) -> Option<String> {
    let dot = s.find('.')?;
    let after = &s[dot + 1..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 7 {
        return None;
    }
    let tail = &after[digits.len()..];
    Some(format!("{}.{}{}", &s[..dot], &digits[..6], tail))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_roundtrip() {
        assert_eq!(
            normalize("2024-09-26T00:00:00Z").unwrap(),
            "2024-09-26T00:00:00+00:00"
        );
    }

    #[test]
    fn rfc2822_roundtrip() {
        assert_eq!(
            normalize("Thu, 26 Sep 2024 00:00:00 +0000").unwrap(),
            "2024-09-26T00:00:00+00:00"
        );
    }

    #[test]
    fn rfc2822_with_offset_normalizes_to_utc() {
        assert_eq!(
            normalize("Thu, 26 Sep 2024 01:00:00 +0100").unwrap(),
            "2024-09-26T00:00:00+00:00"
        );
    }

    #[test]
    fn iso_no_tz_assumed_utc() {
        assert_eq!(
            normalize("2024-09-26T00:00:00").unwrap(),
            "2024-09-26T00:00:00+00:00"
        );
    }

    #[test]
    fn unparseable_passes_through() {
        assert_eq!(normalize("tomorrow").unwrap(), "tomorrow");
    }
}
