use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

use crate::date;
use crate::error::ParseError;
use crate::model::*;
use crate::xml_util::*;

pub fn parse(data: &[u8]) -> Result<Feed, ParseError> {
    let mut reader = Reader::from_reader(data);
    {
        let cfg = reader.config_mut();
        cfg.trim_text(false); // Atom content/summary may need whitespace preserved
        cfg.check_end_names = false;
        cfg.allow_unmatched_ends = true;
    }

    let mut feed = Feed::default();
    let mut buf = Vec::new();
    let mut stack: Vec<ElementKind> = Vec::new();
    let mut current_entry: Option<Entry> = None;
    let mut current_author: Option<Person> = None;
    let mut in_contributor = false;
    let mut text_buf = String::new();
    let mut xhtml_buf: Option<(usize, i32)> = None; // (start byte pos, depth counter for nested div)

    loop {
        let pos_before = reader.buffer_position() as usize;
        let event = reader.read_event_into(&mut buf)?;
        match event {
            Event::Start(e) => {
                let raw_name = e.name();
                let local = local_name(raw_name.as_ref()).to_vec();
                let kind = classify(&local, &stack);

                // If we're already inside an xhtml block, count depth and skip normal handling
                if let Some((_, depth)) = xhtml_buf.as_mut() {
                    *depth += 1;
                    stack.push(ElementKind::XhtmlPassthrough);
                    continue;
                }

                text_buf.clear();

                match &kind {
                    ElementKind::Entry => {
                        current_entry = Some(Entry::default());
                    }
                    ElementKind::Author | ElementKind::Contributor => {
                        current_author = Some(Person::default());
                        in_contributor = matches!(kind, ElementKind::Contributor);
                    }
                    ElementKind::Content | ElementKind::Summary => {
                        if let Some(ty) = attr_value(e.attributes(), b"type") {
                            if ty == "xhtml" {
                                let end_of_open = reader.buffer_position() as usize;
                                xhtml_buf = Some((end_of_open, 0));
                            }
                        }
                    }
                    ElementKind::Link => {
                        let link = extract_link(&e);
                        consume_link(link, &mut feed, &mut current_entry);
                    }
                    ElementKind::Category => {
                        let cat = extract_category(&e);
                        if let Some(entry) = current_entry.as_mut() {
                            entry.categories.push(cat);
                        } else {
                            feed.categories.push(cat);
                        }
                    }
                    _ => {}
                }
                stack.push(kind);
            }
            Event::Empty(e) => {
                let raw_name = e.name();
                let local = local_name(raw_name.as_ref()).to_vec();
                let kind = classify(&local, &stack);
                match kind {
                    ElementKind::Link => {
                        let link = extract_link(&e);
                        consume_link(link, &mut feed, &mut current_entry);
                    }
                    ElementKind::Category => {
                        let cat = extract_category(&e);
                        if let Some(entry) = current_entry.as_mut() {
                            entry.categories.push(cat);
                        } else {
                            feed.categories.push(cat);
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) => {
                if xhtml_buf.is_none() {
                    if let Ok(unescaped) = t.unescape() {
                        text_buf.push_str(&unescaped);
                    }
                }
            }
            Event::CData(t) => {
                if xhtml_buf.is_none() {
                    text_buf.push_str(&String::from_utf8_lossy(t.as_ref()));
                }
            }
            Event::End(_) => {
                let kind = stack.pop().unwrap_or(ElementKind::Unknown);

                if matches!(kind, ElementKind::XhtmlPassthrough) {
                    if let Some((_, depth)) = xhtml_buf.as_mut() {
                        *depth -= 1;
                    }
                    continue;
                }

                // Check if this End closes an xhtml content/summary.
                if let Some((start, _)) = xhtml_buf.take() {
                    if matches!(kind, ElementKind::Content | ElementKind::Summary) {
                        let end = pos_before;
                        let raw = &data[start..end];
                        let xhtml = extract_xhtml_inner(raw);
                        let target = current_entry
                            .as_mut()
                            .map(|e| {
                                if matches!(kind, ElementKind::Content) {
                                    &mut e.content
                                } else {
                                    &mut e.description
                                }
                            });
                        if let Some(slot) = target {
                            if slot.is_none() {
                                *slot = Some(xhtml);
                            }
                        }
                        text_buf.clear();
                        continue;
                    } else {
                        // xhtml was active but this end doesn't close content/summary — shouldn't happen
                        xhtml_buf = Some((0, 0));
                    }
                }

                let text = std::mem::take(&mut text_buf);
                handle_end(
                    kind,
                    text,
                    &mut feed,
                    &mut current_entry,
                    &mut current_author,
                    &mut in_contributor,
                );
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(feed)
}

#[derive(Debug, Clone)]
enum ElementKind {
    Feed,
    Entry,
    Title,
    Subtitle,
    Id,
    Updated,
    Published,
    Summary,
    Content,
    Link,
    Author,
    Contributor,
    Category,
    Generator,
    Icon,
    Logo,
    Rights,
    Source,
    Name,
    Email,
    Uri,
    XhtmlPassthrough,
    Unknown,
}

fn classify(local: &[u8], stack: &[ElementKind]) -> ElementKind {
    let in_author = stack
        .iter()
        .any(|k| matches!(k, ElementKind::Author | ElementKind::Contributor));

    if in_author {
        match local {
            b"name" => return ElementKind::Name,
            b"email" => return ElementKind::Email,
            b"uri" => return ElementKind::Uri,
            _ => {}
        }
    }

    match local {
        b"feed" => ElementKind::Feed,
        b"entry" => ElementKind::Entry,
        b"title" => ElementKind::Title,
        b"subtitle" => ElementKind::Subtitle,
        b"id" => ElementKind::Id,
        b"updated" => ElementKind::Updated,
        b"published" => ElementKind::Published,
        b"summary" => ElementKind::Summary,
        b"content" => ElementKind::Content,
        b"link" => ElementKind::Link,
        b"author" => ElementKind::Author,
        b"contributor" => ElementKind::Contributor,
        b"category" => ElementKind::Category,
        b"generator" => ElementKind::Generator,
        b"icon" => ElementKind::Icon,
        b"logo" => ElementKind::Logo,
        b"rights" => ElementKind::Rights,
        b"source" => ElementKind::Source,
        _ => ElementKind::Unknown,
    }
}

fn handle_end(
    kind: ElementKind,
    text: String,
    feed: &mut Feed,
    current_entry: &mut Option<Entry>,
    current_author: &mut Option<Person>,
    in_contributor: &mut bool,
) {
    let text = text.trim().to_string();
    match kind {
        ElementKind::Entry => {
            if let Some(entry) = current_entry.take() {
                feed.entries.push(entry);
            }
        }
        ElementKind::Title => target_text(feed, current_entry, text, |f| &mut f.title, |e| &mut e.title),
        ElementKind::Subtitle => {
            if current_entry.is_none() {
                set_if_empty(&mut feed.description, text);
            }
        }
        ElementKind::Id => target_text(feed, current_entry, text, |f| &mut f.id, |e| &mut e.id),
        ElementKind::Updated => {
            let norm = date::normalize(&text).unwrap_or(text);
            target_text_str(feed, current_entry, norm, |f| &mut f.updated, |e| &mut e.updated);
        }
        ElementKind::Published => {
            if let Some(entry) = current_entry.as_mut() {
                let norm = date::normalize(&text).unwrap_or(text);
                set_if_empty(&mut entry.published, norm);
            }
        }
        ElementKind::Summary => {
            if let Some(entry) = current_entry.as_mut() {
                set_if_empty(&mut entry.description, text);
            }
        }
        ElementKind::Content => {
            if let Some(entry) = current_entry.as_mut() {
                set_if_empty(&mut entry.content, text);
            }
        }
        ElementKind::Generator => {
            set_if_empty(&mut feed.generator, text);
        }
        ElementKind::Icon => {
            set_if_empty(&mut feed.icon, text);
        }
        ElementKind::Logo => {
            set_if_empty(&mut feed.logo, text);
        }
        ElementKind::Rights => {
            // Not a dedicated field in the new model; ignore for now.
        }
        ElementKind::Source => {
            // Atom source references — ignore; rare in practice.
        }
        ElementKind::Author | ElementKind::Contributor => {
            if let Some(person) = current_author.take() {
                if person.name.is_some() || person.email.is_some() || person.link.is_some() {
                    if *in_contributor {
                        // Drop contributors for now — no dedicated slot.
                    } else if let Some(entry) = current_entry.as_mut() {
                        entry.authors.push(person);
                    } else {
                        feed.authors.push(person);
                    }
                }
            }
            *in_contributor = false;
        }
        ElementKind::Name => {
            if let Some(p) = current_author.as_mut() {
                if !text.is_empty() {
                    p.name = Some(text);
                }
            }
        }
        ElementKind::Email => {
            if let Some(p) = current_author.as_mut() {
                if !text.is_empty() {
                    p.email = Some(text);
                }
            }
        }
        ElementKind::Uri => {
            if let Some(p) = current_author.as_mut() {
                if !text.is_empty() {
                    p.link = Some(text);
                }
            }
        }
        _ => {}
    }
}

fn target_text(
    feed: &mut Feed,
    current_entry: &mut Option<Entry>,
    text: String,
    feed_slot: impl FnOnce(&mut Feed) -> &mut Option<String>,
    entry_slot: impl FnOnce(&mut Entry) -> &mut Option<String>,
) {
    if let Some(entry) = current_entry.as_mut() {
        set_if_empty(entry_slot(entry), text);
    } else {
        set_if_empty(feed_slot(feed), text);
    }
}

fn target_text_str(
    feed: &mut Feed,
    current_entry: &mut Option<Entry>,
    text: String,
    feed_slot: impl FnOnce(&mut Feed) -> &mut Option<String>,
    entry_slot: impl FnOnce(&mut Entry) -> &mut Option<String>,
) {
    target_text(feed, current_entry, text, feed_slot, entry_slot)
}

fn set_if_empty(target: &mut Option<String>, value: String) {
    if target.is_none() && !value.is_empty() {
        *target = Some(value);
    }
}

fn extract_link(e: &BytesStart<'_>) -> Link {
    let mut link = Link::default();
    for a in e.attributes().flatten() {
        match local_name(a.key.as_ref()) {
            b"href" => {
                link.href = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
            }
            b"rel" => {
                link.rel = a.unescape_value().ok().map(|v| v.into_owned());
            }
            b"type" => {
                link.type_ = a.unescape_value().ok().map(|v| v.into_owned());
            }
            b"title" => {
                link.title = a.unescape_value().ok().map(|v| v.into_owned());
            }
            _ => {}
        }
    }
    link
}

fn consume_link(link: Link, feed: &mut Feed, current_entry: &mut Option<Entry>) {
    if link.href.is_empty() {
        return;
    }
    let rel = link.rel.as_deref().unwrap_or("alternate");
    if let Some(entry) = current_entry.as_mut() {
        match rel {
            "alternate" => {
                if entry.link.is_none() {
                    entry.link = Some(link.href.clone());
                }
                entry.links.push(link);
            }
            "enclosure" => {
                entry.enclosures.push(Enclosure {
                    url: link.href,
                    length: None,
                    type_: link.type_,
                });
            }
            _ => {
                entry.links.push(link);
            }
        }
    } else {
        if feed.link.is_none() && (rel == "alternate") {
            feed.link = Some(link.href.clone());
        }
        feed.links.push(link);
    }
}

fn extract_category(e: &BytesStart<'_>) -> Category {
    let mut cat = Category::default();
    for a in e.attributes().flatten() {
        match local_name(a.key.as_ref()) {
            b"term" => {
                cat.term = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
            }
            b"scheme" => {
                cat.scheme = a.unescape_value().ok().map(|v| v.into_owned());
            }
            b"label" => {
                cat.label = a.unescape_value().ok().map(|v| v.into_owned());
            }
            _ => {}
        }
    }
    cat
}

/// Extract the inner XHTML by stripping the outer wrapping `<div xmlns="...">...</div>`.
/// Input is the raw bytes between `<content type="xhtml">` and `</content>`.
fn extract_xhtml_inner(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    let trimmed = s.trim();
    // Strip a leading `<div ...>` and trailing `</div>` if present.
    if let Some(after_open) = skip_div_open(trimmed) {
        if let Some(before_close) = strip_trailing_div_close(after_open) {
            return before_close.trim().to_string();
        }
    }
    trimmed.to_string()
}

fn skip_div_open(s: &str) -> Option<&str> {
    let s = s.trim_start();
    if !s.starts_with("<div") {
        return None;
    }
    let rest = &s[4..];
    let close = rest.find('>')?;
    Some(&rest[close + 1..])
}

fn strip_trailing_div_close(s: &str) -> Option<&str> {
    let s = s.trim_end();
    if let Some(idx) = s.rfind("</div>") {
        if s[idx + 6..].trim().is_empty() {
            return Some(&s[..idx]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_xhtml_div_wrapper() {
        let raw = br#"
    <div xmlns="http://www.w3.org/1999/xhtml">
      <p>Hello</p>
    </div>
  "#;
        let out = extract_xhtml_inner(raw);
        assert!(out.contains("<p>Hello</p>"));
        assert!(!out.contains("<div"));
    }

    #[test]
    fn parses_simple_atom_feed() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
    <title>Example</title>
    <id>urn:x</id>
    <updated>2024-11-29T00:00:00Z</updated>
    <link href="https://example.com/" rel="self"/>
    <link href="https://example.com/web" rel="alternate"/>
    <entry>
        <title>Post 1</title>
        <id>urn:x:1</id>
        <updated>2024-11-28T00:00:00Z</updated>
        <published>2024-11-27T00:00:00Z</published>
        <link rel="alternate" href="https://example.com/web/1"/>
        <author><name>Alice</name></author>
        <summary type="text">A summary</summary>
    </entry>
</feed>"#;
        let feed = parse(xml).unwrap();
        assert_eq!(feed.title.as_deref(), Some("Example"));
        assert_eq!(feed.id.as_deref(), Some("urn:x"));
        assert_eq!(feed.link.as_deref(), Some("https://example.com/web"));
        assert_eq!(feed.entries.len(), 1);
        let e = &feed.entries[0];
        assert_eq!(e.title.as_deref(), Some("Post 1"));
        assert_eq!(e.link.as_deref(), Some("https://example.com/web/1"));
        assert_eq!(e.published.as_deref(), Some("2024-11-27T00:00:00+00:00"));
        assert_eq!(e.updated.as_deref(), Some("2024-11-28T00:00:00+00:00"));
        assert_eq!(e.authors.len(), 1);
        assert_eq!(e.authors[0].name.as_deref(), Some("Alice"));
        assert_eq!(e.description.as_deref(), Some("A summary"));
    }
}
