use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::date;
use crate::error::ParseError;
use crate::model::*;
use crate::xml_util::*;

pub fn parse(data: &[u8]) -> Result<Feed, ParseError> {
    let mut reader = Reader::from_reader(data);
    {
        let cfg = reader.config_mut();
        cfg.trim_text(true);
        cfg.check_end_names = false;
        cfg.allow_unmatched_ends = true;
    }

    let mut feed = Feed::default();
    let mut buf = Vec::new();
    let mut stack: Vec<Kind> = Vec::new();
    let mut current_item: Option<Entry> = None;
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let raw_name = e.name();
                let local = local_name(raw_name.as_ref()).to_vec();
                let prefix = namespace_prefix(raw_name.as_ref()).map(|p| p.to_vec());
                let kind = classify(&prefix, &local, &stack);
                text_buf.clear();

                if matches!(kind, Kind::Item) {
                    let mut entry = Entry::default();
                    // rdf:about attribute becomes id / link
                    for a in e.attributes().flatten() {
                        if local_name(a.key.as_ref()) == b"about" {
                            if let Ok(v) = a.unescape_value() {
                                let s = v.into_owned();
                                entry.id = Some(s.clone());
                                entry.link = Some(s);
                            }
                        }
                    }
                    current_item = Some(entry);
                }
                stack.push(kind);
            }
            Event::Empty(_) => {}
            Event::Text(t) => {
                if let Ok(unescaped) = t.unescape() {
                    text_buf.push_str(&unescaped);
                }
            }
            Event::CData(t) => {
                text_buf.push_str(&String::from_utf8_lossy(t.as_ref()));
            }
            Event::End(_) => {
                let kind = stack.pop().unwrap_or(Kind::Unknown);
                let text = std::mem::take(&mut text_buf);
                handle_end(kind, text.trim().to_string(), &mut feed, &mut current_item);
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(feed)
}

#[derive(Debug, Clone)]
enum Kind {
    Rdf,
    Channel,
    Item,
    Title,
    Link,
    Description,
    Language,
    DcCreator,
    DcDate,
    DcSubject,
    ContentEncoded,
    Unknown,
}

fn classify(prefix: &Option<Vec<u8>>, local: &[u8], stack: &[Kind]) -> Kind {
    if prefix.as_deref() == Some(b"dc") {
        return match local {
            b"creator" => Kind::DcCreator,
            b"date" => Kind::DcDate,
            b"subject" => Kind::DcSubject,
            _ => Kind::Unknown,
        };
    }
    if prefix.as_deref() == Some(b"content") && local == b"encoded" {
        return Kind::ContentEncoded;
    }
    if prefix.as_deref() == Some(b"rdf") && local == b"RDF" {
        return Kind::Rdf;
    }
    // Unprefixed / default-namespace
    let in_item = stack.iter().any(|k| matches!(k, Kind::Item));
    let in_channel = stack.iter().any(|k| matches!(k, Kind::Channel));
    match local {
        b"channel" => Kind::Channel,
        b"item" => Kind::Item,
        b"title" if in_item || in_channel => Kind::Title,
        b"link" if in_item || in_channel => Kind::Link,
        b"description" if in_item || in_channel => Kind::Description,
        b"language" if in_channel => Kind::Language,
        _ => Kind::Unknown,
    }
}

fn handle_end(kind: Kind, text: String, feed: &mut Feed, current_item: &mut Option<Entry>) {
    match kind {
        Kind::Item => {
            if let Some(item) = current_item.take() {
                feed.entries.push(item);
            }
        }
        Kind::Title => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.title, text);
            } else {
                set_if_empty(&mut feed.title, text);
            }
        }
        Kind::Link => {
            if let Some(item) = current_item.as_mut() {
                // RDF items: <link> overrides the rdf:about-derived link.
                item.link = Some(text);
            } else {
                set_if_empty(&mut feed.link, text);
            }
        }
        Kind::Description => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.description, text);
            } else {
                set_if_empty(&mut feed.description, text);
            }
        }
        Kind::Language => {
            set_if_empty(&mut feed.language, text);
        }
        Kind::DcCreator => {
            let person = Person {
                name: Some(text),
                email: None,
                link: None,
            };
            if let Some(item) = current_item.as_mut() {
                item.authors.push(person);
            } else {
                feed.authors.push(person);
            }
        }
        Kind::DcDate => {
            let normalized = date::normalize(&text).unwrap_or(text);
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.published, normalized);
            } else {
                set_if_empty(&mut feed.updated, normalized);
            }
        }
        Kind::DcSubject => {
            let cat = Category {
                term: text,
                scheme: None,
                label: None,
            };
            if let Some(item) = current_item.as_mut() {
                item.categories.push(cat);
            } else {
                feed.categories.push(cat);
            }
        }
        Kind::ContentEncoded => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.content, text);
            }
        }
        _ => {}
    }
}

fn set_if_empty(target: &mut Option<String>, value: String) {
    if target.is_none() && !value.is_empty() {
        *target = Some(value);
    }
}
