use serde_json::Value;

use crate::date;
use crate::error::ParseError;
use crate::model::*;

pub fn parse(data: &[u8]) -> Result<Feed, ParseError> {
    let root: Value = serde_json::from_slice(data)
        .map_err(|e| ParseError::Xml(format!("JSON parse error: {e}")))?;
    let obj = root.as_object().ok_or(ParseError::NotAFeed)?;

    // Validate it's JSON Feed (permissive — just look for a 'version' or 'items').
    let version_ok = obj
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.contains("jsonfeed.org"))
        .unwrap_or(false);
    if !version_ok && !obj.contains_key("items") {
        return Err(ParseError::NotAFeed);
    }

    let mut feed = Feed::default();
    feed.title = str_field(obj.get("title"));
    feed.description = str_field(obj.get("description"));
    feed.link = str_field(obj.get("home_page_url"));
    feed.language = str_field(obj.get("language"));
    feed.icon = str_field(obj.get("favicon"));
    feed.logo = str_field(obj.get("icon"));

    if let Some(feed_url) = str_field(obj.get("feed_url")) {
        feed.links.push(Link {
            href: feed_url,
            rel: Some("self".to_string()),
            type_: Some("application/json".to_string()),
            title: None,
        });
    }

    feed.authors = parse_authors(obj.get("authors"), obj.get("author"));

    if let Some(items) = obj.get("items").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(o) = item.as_object() {
                feed.entries.push(parse_item(o));
            }
        }
    }

    Ok(feed)
}

fn parse_item(o: &serde_json::Map<String, Value>) -> Entry {
    let mut e = Entry::default();
    e.id = str_field(o.get("id"));
    e.link = str_field(o.get("url"));
    e.title = str_field(o.get("title"));
    e.content = str_field(o.get("content_html")).or_else(|| str_field(o.get("content_text")));
    e.description = str_field(o.get("summary"));
    e.published = str_field(o.get("date_published")).and_then(|s| date::normalize(&s));
    e.updated = str_field(o.get("date_modified")).and_then(|s| date::normalize(&s));
    e.authors = parse_authors(o.get("authors"), o.get("author"));

    if let Some(external) = str_field(o.get("external_url")) {
        e.links.push(Link {
            href: external,
            rel: Some("related".to_string()),
            type_: None,
            title: None,
        });
    }

    if let Some(tags) = o.get("tags").and_then(|v| v.as_array()) {
        for t in tags {
            if let Some(s) = t.as_str() {
                e.categories.push(Category {
                    term: s.to_string(),
                    scheme: None,
                    label: None,
                });
            }
        }
    }

    if let Some(attachments) = o.get("attachments").and_then(|v| v.as_array()) {
        for a in attachments {
            if let Some(ao) = a.as_object() {
                let url = str_field(ao.get("url")).unwrap_or_default();
                if !url.is_empty() {
                    e.enclosures.push(Enclosure {
                        url,
                        length: ao.get("size_in_bytes").and_then(|v| v.as_u64()),
                        type_: str_field(ao.get("mime_type")),
                    });
                }
            }
        }
    }

    e
}

fn parse_authors(authors: Option<&Value>, single: Option<&Value>) -> Vec<Person> {
    let mut out = Vec::new();
    if let Some(arr) = authors.and_then(|v| v.as_array()) {
        for a in arr {
            if let Some(o) = a.as_object() {
                out.push(Person {
                    name: str_field(o.get("name")),
                    email: None,
                    link: str_field(o.get("url")),
                });
            }
        }
    }
    if let Some(one) = single {
        if let Some(o) = one.as_object() {
            out.push(Person {
                name: str_field(o.get("name")),
                email: None,
                link: str_field(o.get("url")),
            });
        } else if let Some(s) = one.as_str() {
            out.push(Person {
                name: Some(s.to_string()),
                email: None,
                link: None,
            });
        }
    }
    out
}

fn str_field(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| x.as_str()).map(|s| s.to_string())
}
