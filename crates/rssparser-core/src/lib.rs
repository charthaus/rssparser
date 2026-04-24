mod atom;
mod date;
mod error;
mod json_feed;
mod model;
mod preprocess;
mod rdf;
mod rss;
mod xml_util;

pub use error::ParseError;
pub use model::*;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use xml_util::local_name;

pub fn parse(data: &[u8]) -> Result<Feed, ParseError> {
    match first_non_whitespace(data) {
        Some(b'<') | Some(0xFF) | Some(0xFE) | Some(0xEF) => {
            let cleaned = preprocess::prepare(data)?;
            dispatch_xml(&cleaned)
        }
        Some(b'{') | Some(b'[') => json_feed::parse(data),
        _ => Err(ParseError::NotAFeed),
    }
}

fn first_non_whitespace(data: &[u8]) -> Option<u8> {
    // Return first meaningful byte for dispatch. UTF-16/UTF-8 BOMs go through preprocess.
    if data.starts_with(&[0xFF, 0xFE]) || data.starts_with(&[0xFE, 0xFF]) {
        return Some(0xFF);
    }
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return data.iter().skip(3).copied().find(|b| !b.is_ascii_whitespace());
    }
    data.iter().copied().find(|b| !b.is_ascii_whitespace())
}

fn dispatch_xml(data: &[u8]) -> Result<Feed, ParseError> {
    let root = peek_root_tag(data);
    match root.as_deref() {
        Some(b"rss") => rss::parse(data),
        Some(b"feed") => atom::parse(data),
        Some(b"RDF") => rdf::parse(data),
        _ => Err(ParseError::NotAFeed),
    }
}

fn peek_root_tag(data: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::from_reader(data);
    {
        let cfg = reader.config_mut();
        cfg.trim_text(true);
        cfg.check_end_names = false;
        cfg.allow_unmatched_ends = true;
    }
    let mut buf = Vec::new();
    for _ in 0..8 {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(e) | Event::Empty(e) => {
                return Some(local_name(e.name().as_ref()).to_vec());
            }
            Event::Eof => return None,
            _ => {}
        }
        buf.clear();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_rss() {
        let xml = br#"<?xml version="1.0"?><rss version="2.0"><channel><title>X</title></channel></rss>"#;
        assert_eq!(parse(xml).unwrap().title.as_deref(), Some("X"));
    }

    #[test]
    fn dispatches_atom() {
        let xml = br#"<?xml version="1.0"?><feed xmlns="http://www.w3.org/2005/Atom"><title>X</title></feed>"#;
        assert_eq!(parse(xml).unwrap().title.as_deref(), Some("X"));
    }

    #[test]
    fn dispatches_rdf() {
        let xml = br#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns="http://purl.org/rss/1.0/">
  <channel><title>X</title></channel>
</rdf:RDF>"#;
        assert_eq!(parse(xml).unwrap().title.as_deref(), Some("X"));
    }

    #[test]
    fn dispatches_json_feed() {
        let data = br#"{"version":"https://jsonfeed.org/version/1.1","title":"Hello","items":[]}"#;
        assert_eq!(parse(data).unwrap().title.as_deref(), Some("Hello"));
    }

    #[test]
    fn empty_document_errors() {
        let _ = parse(b"").unwrap_err();
    }

    #[test]
    fn non_feed_json_errors() {
        let _ = parse(br#"{"foo":1}"#).unwrap_err();
    }
}
