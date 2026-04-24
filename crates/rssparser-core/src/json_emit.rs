//! Direct JSON serialization of the Feed model — no intermediate Value tree.
//!
//! Must produce bytes that `json.loads` parses into the same dict shape as
//! the Python `Feed.to_dict()` binding.

use crate::model::*;

pub fn feed_to_json_bytes(feed: &Feed) -> Vec<u8> {
    let mut out = Vec::with_capacity(4096);
    write_feed(&mut out, feed);
    out
}

fn write_feed(out: &mut Vec<u8>, feed: &Feed) {
    out.push(b'{');
    out.extend_from_slice(br#""feed":"#);
    write_feed_info(out, feed);
    out.extend_from_slice(br#","entries":["#);
    for (i, e) in feed.entries.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        write_entry(out, e);
    }
    out.extend_from_slice(b"]}");
}

fn write_feed_info(out: &mut Vec<u8>, f: &Feed) {
    out.push(b'{');
    write_field_opt_str(out, "title", &f.title, true);
    write_field_opt_str(out, "link", &f.link, false);
    out.push(b',');
    write_key(out, "links");
    write_links(out, &f.links);
    write_field_opt_str(out, "description", &f.description, false);
    write_field_opt_str(out, "language", &f.language, false);
    write_field_opt_str(out, "generator", &f.generator, false);
    write_field_opt_str(out, "updated", &f.updated, false);
    write_field_opt_str(out, "id", &f.id, false);
    out.push(b',');
    write_key(out, "image");
    write_image(out, f.image.as_ref());
    write_field_opt_str(out, "icon", &f.icon, false);
    write_field_opt_str(out, "logo", &f.logo, false);
    out.push(b',');
    write_key(out, "authors");
    write_persons(out, &f.authors);
    out.push(b',');
    write_key(out, "categories");
    write_categories(out, &f.categories);
    out.push(b'}');
}

fn write_entry(out: &mut Vec<u8>, e: &Entry) {
    out.push(b'{');
    write_field_opt_str(out, "title", &e.title, true);
    write_field_opt_str(out, "link", &e.link, false);
    out.push(b',');
    write_key(out, "links");
    write_links(out, &e.links);
    write_field_opt_str(out, "description", &e.description, false);
    write_field_opt_str(out, "content", &e.content, false);
    write_field_opt_str(out, "published", &e.published, false);
    write_field_opt_str(out, "updated", &e.updated, false);
    write_field_opt_str(out, "id", &e.id, false);
    out.push(b',');
    write_key(out, "authors");
    write_persons(out, &e.authors);
    out.push(b',');
    write_key(out, "categories");
    write_categories(out, &e.categories);
    out.push(b',');
    write_key(out, "enclosures");
    write_enclosures(out, &e.enclosures);
    out.push(b',');
    write_key(out, "media");
    write_media(out, &e.media);
    out.push(b'}');
}

fn write_key(out: &mut Vec<u8>, key: &str) {
    out.push(b'"');
    out.extend_from_slice(key.as_bytes());
    out.extend_from_slice(br#"":"#);
}

fn write_field_opt_str(out: &mut Vec<u8>, key: &str, value: &Option<String>, first: bool) {
    if !first {
        out.push(b',');
    }
    write_key(out, key);
    match value {
        Some(s) => write_escaped_string(out, s),
        None => out.extend_from_slice(b"null"),
    }
}

fn write_opt_str(out: &mut Vec<u8>, value: &Option<String>) {
    match value {
        Some(s) => write_escaped_string(out, s),
        None => out.extend_from_slice(b"null"),
    }
}

fn write_opt_u64(out: &mut Vec<u8>, value: &Option<u64>) {
    match value {
        Some(n) => write_u64(out, *n),
        None => out.extend_from_slice(b"null"),
    }
}

fn write_opt_u32(out: &mut Vec<u8>, value: &Option<u32>) {
    match value {
        Some(n) => write_u64(out, *n as u64),
        None => out.extend_from_slice(b"null"),
    }
}

fn write_u64(out: &mut Vec<u8>, mut n: u64) {
    if n == 0 {
        out.push(b'0');
        return;
    }
    let mut tmp = [0u8; 20];
    let mut i = 20;
    while n > 0 {
        i -= 1;
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    out.extend_from_slice(&tmp[i..]);
}

fn write_links(out: &mut Vec<u8>, links: &[Link]) {
    out.push(b'[');
    for (i, l) in links.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        out.push(b'{');
        write_key(out, "href");
        write_escaped_string(out, &l.href);
        out.push(b',');
        write_key(out, "rel");
        write_opt_str(out, &l.rel);
        out.push(b',');
        write_key(out, "type");
        write_opt_str(out, &l.type_);
        out.push(b',');
        write_key(out, "title");
        write_opt_str(out, &l.title);
        out.push(b'}');
    }
    out.push(b']');
}

fn write_persons(out: &mut Vec<u8>, people: &[Person]) {
    out.push(b'[');
    for (i, p) in people.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        out.push(b'{');
        write_key(out, "name");
        write_opt_str(out, &p.name);
        out.push(b',');
        write_key(out, "email");
        write_opt_str(out, &p.email);
        out.push(b',');
        write_key(out, "link");
        write_opt_str(out, &p.link);
        out.push(b'}');
    }
    out.push(b']');
}

fn write_categories(out: &mut Vec<u8>, cats: &[Category]) {
    out.push(b'[');
    for (i, c) in cats.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        out.push(b'{');
        write_key(out, "term");
        write_escaped_string(out, &c.term);
        out.push(b',');
        write_key(out, "scheme");
        write_opt_str(out, &c.scheme);
        out.push(b',');
        write_key(out, "label");
        write_opt_str(out, &c.label);
        out.push(b'}');
    }
    out.push(b']');
}

fn write_enclosures(out: &mut Vec<u8>, encs: &[Enclosure]) {
    out.push(b'[');
    for (i, e) in encs.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        out.push(b'{');
        write_key(out, "url");
        write_escaped_string(out, &e.url);
        out.push(b',');
        write_key(out, "length");
        write_opt_u64(out, &e.length);
        out.push(b',');
        write_key(out, "type");
        write_opt_str(out, &e.type_);
        out.push(b'}');
    }
    out.push(b']');
}

fn write_media(out: &mut Vec<u8>, media: &[MediaContent]) {
    out.push(b'[');
    for (i, m) in media.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        out.push(b'{');
        write_key(out, "url");
        write_escaped_string(out, &m.url);
        out.push(b',');
        write_key(out, "type");
        write_opt_str(out, &m.type_);
        out.push(b',');
        write_key(out, "medium");
        write_opt_str(out, &m.medium);
        out.push(b',');
        write_key(out, "width");
        write_opt_u32(out, &m.width);
        out.push(b',');
        write_key(out, "height");
        write_opt_u32(out, &m.height);
        out.push(b',');
        write_key(out, "duration");
        write_opt_u32(out, &m.duration);
        out.push(b',');
        write_key(out, "title");
        write_opt_str(out, &m.title);
        out.push(b',');
        write_key(out, "description");
        write_opt_str(out, &m.description);
        out.push(b',');
        write_key(out, "thumbnail");
        write_opt_str(out, &m.thumbnail);
        out.push(b'}');
    }
    out.push(b']');
}

fn write_image(out: &mut Vec<u8>, img: Option<&Image>) {
    match img {
        None => out.extend_from_slice(b"null"),
        Some(i) => {
            out.push(b'{');
            write_key(out, "url");
            write_escaped_string(out, &i.url);
            out.push(b',');
            write_key(out, "title");
            write_opt_str(out, &i.title);
            out.push(b',');
            write_key(out, "link");
            write_opt_str(out, &i.link);
            out.push(b',');
            write_key(out, "width");
            write_opt_u32(out, &i.width);
            out.push(b',');
            write_key(out, "height");
            write_opt_u32(out, &i.height);
            out.push(b'}');
        }
    }
}

fn write_escaped_string(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    let bytes = s.as_bytes();
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let escape: &[u8] = match b {
            b'"' => br#"\""#,
            b'\\' => br#"\\"#,
            b'\n' => br#"\n"#,
            b'\r' => br#"\r"#,
            b'\t' => br#"\t"#,
            b'\x08' => br#"\b"#,
            b'\x0c' => br#"\f"#,
            0..=0x1f => {
                // Flush safe run, then emit \u00XX.
                if start < i {
                    out.extend_from_slice(&bytes[start..i]);
                }
                out.extend_from_slice(br#"\u00"#);
                out.push(hex(b >> 4));
                out.push(hex(b & 0x0f));
                start = i + 1;
                continue;
            }
            _ => continue,
        };
        if start < i {
            out.extend_from_slice(&bytes[start..i]);
        }
        out.extend_from_slice(escape);
        start = i + 1;
    }
    if start < bytes.len() {
        out.extend_from_slice(&bytes[start..]);
    }
    out.push(b'"');
}

fn hex(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        10..=15 => b'a' + (nibble - 10),
        _ => unreachable!(),
    }
}
