use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::date;
use crate::error::ParseError;
use crate::model::*;
use crate::xml_util::*;

const NS_CONTENT: &[u8] = b"content";
const NS_DC: &[u8] = b"dc";
const NS_ATOM: &[u8] = b"atom";
const NS_MEDIA: &[u8] = b"media";
const NS_ITUNES: &[u8] = b"itunes";

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
    let mut stack: Vec<ElementKind> = Vec::new();
    let mut current_item: Option<Entry> = None;
    let mut current_image: Option<Image> = None;
    let mut current_author: Option<Person> = None;
    let mut text_buf = String::new();
    let mut cdata_active = false;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let raw_name = e.name();
                let prefix = namespace_prefix(raw_name.as_ref()).map(|p| p.to_vec());
                let local = local_name(raw_name.as_ref()).to_vec();
                let kind = classify(&prefix, &local, &stack);
                text_buf.clear();
                cdata_active = false;

                match kind {
                    ElementKind::Item => {
                        current_item = Some(Entry::default());
                    }
                    ElementKind::ImageBlock => {
                        current_image = Some(Image::default());
                    }
                    ElementKind::ItemAuthor | ElementKind::FeedManagingEditor => {
                        current_author = Some(Person::default());
                    }
                    ElementKind::Enclosure => {
                        if let Some(item) = current_item.as_mut() {
                            let mut enc = Enclosure::default();
                            for a in e.attributes().flatten() {
                                match local_name(a.key.as_ref()) {
                                    b"url" => {
                                        enc.url =
                                            a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
                                    }
                                    b"length" => {
                                        enc.length = a
                                            .unescape_value()
                                            .ok()
                                            .and_then(|v| parse_u64(&v));
                                    }
                                    b"type" => {
                                        enc.type_ =
                                            a.unescape_value().ok().map(|v| v.into_owned());
                                    }
                                    _ => {}
                                }
                            }
                            if !enc.url.is_empty() {
                                item.enclosures.push(enc);
                            }
                        }
                    }
                    ElementKind::AtomLink => {
                        let link = extract_atom_link(&e);
                        if let Some(l) = link {
                            if let Some(item) = current_item.as_mut() {
                                item.links.push(l);
                            } else {
                                feed.links.push(l);
                            }
                        }
                    }
                    ElementKind::MediaContent => {
                        if let Some(item) = current_item.as_mut() {
                            let mc = extract_media_content(&e);
                            if !mc.url.is_empty() {
                                item.media.push(mc);
                            }
                        }
                    }
                    ElementKind::MediaThumbnail => {
                        if let Some(item) = current_item.as_mut() {
                            if let Some(url) = e
                                .attributes()
                                .flatten()
                                .find(|a| local_name(a.key.as_ref()) == b"url")
                                .and_then(|a| a.unescape_value().ok())
                            {
                                if let Some(last) = item.media.last_mut() {
                                    last.thumbnail = Some(url.into_owned());
                                } else {
                                    let mut mc = MediaContent::default();
                                    mc.thumbnail = Some(url.into_owned());
                                    item.media.push(mc);
                                }
                            }
                        }
                    }
                    ElementKind::Category => {
                        let scheme = attr_value(e.attributes(), b"domain");
                        text_buf.clear();
                        stack.push(ElementKind::CategoryPending(scheme));
                        continue;
                    }
                    ElementKind::GuidStart => {
                        let is_permalink =
                            attr_value(e.attributes(), b"isPermaLink").as_deref() != Some("false");
                        stack.push(ElementKind::GuidPending(is_permalink));
                        continue;
                    }
                    _ => {}
                }
                stack.push(kind);
            }
            Event::Empty(e) => {
                let raw_name = e.name();
                let prefix = namespace_prefix(raw_name.as_ref()).map(|p| p.to_vec());
                let local = local_name(raw_name.as_ref()).to_vec();
                let kind = classify(&prefix, &local, &stack);

                match kind {
                    ElementKind::Enclosure => {
                        if let Some(item) = current_item.as_mut() {
                            let mut enc = Enclosure::default();
                            for a in e.attributes().flatten() {
                                match local_name(a.key.as_ref()) {
                                    b"url" => {
                                        enc.url = a
                                            .unescape_value()
                                            .map(|v| v.into_owned())
                                            .unwrap_or_default();
                                    }
                                    b"length" => {
                                        enc.length = a
                                            .unescape_value()
                                            .ok()
                                            .and_then(|v| parse_u64(&v));
                                    }
                                    b"type" => {
                                        enc.type_ =
                                            a.unescape_value().ok().map(|v| v.into_owned());
                                    }
                                    _ => {}
                                }
                            }
                            if !enc.url.is_empty() {
                                item.enclosures.push(enc);
                            }
                        }
                    }
                    ElementKind::AtomLink => {
                        if let Some(l) = extract_atom_link(&e) {
                            if let Some(item) = current_item.as_mut() {
                                item.links.push(l);
                            } else {
                                feed.links.push(l);
                            }
                        }
                    }
                    ElementKind::MediaContent => {
                        if let Some(item) = current_item.as_mut() {
                            let mc = extract_media_content(&e);
                            if !mc.url.is_empty() {
                                item.media.push(mc);
                            }
                        }
                    }
                    ElementKind::MediaThumbnail => {
                        if let Some(item) = current_item.as_mut() {
                            if let Some(url) = e
                                .attributes()
                                .flatten()
                                .find(|a| local_name(a.key.as_ref()) == b"url")
                                .and_then(|a| a.unescape_value().ok())
                            {
                                if let Some(last) = item.media.last_mut() {
                                    last.thumbnail = Some(url.into_owned());
                                } else {
                                    let mut mc = MediaContent::default();
                                    mc.thumbnail = Some(url.into_owned());
                                    item.media.push(mc);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) => {
                if !t.is_empty() {
                    if let Ok(unescaped) = t.unescape() {
                        text_buf.push_str(&unescaped);
                    }
                }
            }
            Event::CData(t) => {
                cdata_active = true;
                text_buf.push_str(&String::from_utf8_lossy(t.as_ref()));
            }
            Event::End(_) => {
                let kind = stack.pop().unwrap_or(ElementKind::Unknown);
                let text = std::mem::take(&mut text_buf);
                handle_end(
                    kind,
                    text,
                    cdata_active,
                    &mut feed,
                    &mut current_item,
                    &mut current_image,
                    &mut current_author,
                );
                cdata_active = false;
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
    Channel,
    Item,
    FeedTitle,
    FeedLink,
    FeedDescription,
    FeedLanguage,
    FeedGenerator,
    FeedPubDate,
    FeedLastBuildDate,
    FeedCopyright,
    FeedManagingEditor,
    FeedWebmaster,
    PersonName,
    PersonEmail,
    PersonUri,
    ImageBlock,
    ImageUrl,
    ImageTitle,
    ImageLink,
    ImageWidth,
    ImageHeight,
    ItemTitle,
    ItemLink,
    ItemDescription,
    ItemAuthor,
    ItemPubDate,
    ItemComments,
    ItemSource,
    ContentEncoded,
    DcCreator,
    DcDate,
    DcSubject,
    Enclosure,
    AtomLink,
    MediaContent,
    MediaThumbnail,
    MediaTitle,
    MediaDescription,
    Category,
    CategoryPending(Option<String>),
    GuidStart,
    GuidPending(bool),
    ItunesAuthor,
    ItunesSummary,
    Unknown,
}

fn classify(prefix: &Option<Vec<u8>>, local: &[u8], stack: &[ElementKind]) -> ElementKind {
    let in_item = stack
        .iter()
        .any(|k| matches!(k, ElementKind::Item));
    let in_image = stack
        .iter()
        .any(|k| matches!(k, ElementKind::ImageBlock));
    let in_channel = stack
        .iter()
        .any(|k| matches!(k, ElementKind::Channel));
    let in_author = stack
        .iter()
        .any(|k| matches!(k, ElementKind::ItemAuthor | ElementKind::FeedManagingEditor));

    if in_author {
        match local {
            b"name" => return ElementKind::PersonName,
            b"email" => return ElementKind::PersonEmail,
            b"uri" => return ElementKind::PersonUri,
            _ => {}
        }
    }

    if prefix.as_deref() == Some(NS_ATOM) && local == b"link" {
        return ElementKind::AtomLink;
    }
    if prefix.as_deref() == Some(NS_CONTENT) && local == b"encoded" {
        return ElementKind::ContentEncoded;
    }
    if prefix.as_deref() == Some(NS_DC) {
        return match local {
            b"creator" => ElementKind::DcCreator,
            b"date" => ElementKind::DcDate,
            b"subject" => ElementKind::DcSubject,
            _ => ElementKind::Unknown,
        };
    }
    if prefix.as_deref() == Some(NS_MEDIA) {
        return match local {
            b"content" => ElementKind::MediaContent,
            b"thumbnail" => ElementKind::MediaThumbnail,
            b"title" => ElementKind::MediaTitle,
            b"description" => ElementKind::MediaDescription,
            b"group" => ElementKind::Unknown,
            _ => ElementKind::Unknown,
        };
    }
    if prefix.as_deref() == Some(NS_ITUNES) {
        return match local {
            b"author" => ElementKind::ItunesAuthor,
            b"summary" => ElementKind::ItunesSummary,
            _ => ElementKind::Unknown,
        };
    }

    // Unprefixed (or RSS-default) elements.
    match local {
        b"channel" => ElementKind::Channel,
        b"item" => ElementKind::Item,
        b"image" if in_channel && !in_item => ElementKind::ImageBlock,
        b"enclosure" if in_item => ElementKind::Enclosure,
        b"category" => ElementKind::Category,
        b"guid" if in_item => ElementKind::GuidStart,
        b"title" if in_image => ElementKind::ImageTitle,
        b"url" if in_image => ElementKind::ImageUrl,
        b"link" if in_image => ElementKind::ImageLink,
        b"width" if in_image => ElementKind::ImageWidth,
        b"height" if in_image => ElementKind::ImageHeight,
        b"title" if in_item => ElementKind::ItemTitle,
        b"link" if in_item => ElementKind::ItemLink,
        b"description" if in_item => ElementKind::ItemDescription,
        b"author" if in_item => ElementKind::ItemAuthor,
        b"pubDate" if in_item => ElementKind::ItemPubDate,
        b"comments" if in_item => ElementKind::ItemComments,
        b"source" if in_item => ElementKind::ItemSource,
        b"title" if in_channel => ElementKind::FeedTitle,
        b"link" if in_channel => ElementKind::FeedLink,
        b"description" if in_channel => ElementKind::FeedDescription,
        b"language" if in_channel => ElementKind::FeedLanguage,
        b"generator" if in_channel => ElementKind::FeedGenerator,
        b"pubDate" if in_channel => ElementKind::FeedPubDate,
        b"lastBuildDate" if in_channel => ElementKind::FeedLastBuildDate,
        b"copyright" if in_channel => ElementKind::FeedCopyright,
        b"managingEditor" if in_channel => ElementKind::FeedManagingEditor,
        b"webMaster" if in_channel => ElementKind::FeedWebmaster,
        _ => ElementKind::Unknown,
    }
}

fn handle_end(
    kind: ElementKind,
    text: String,
    _cdata: bool,
    feed: &mut Feed,
    current_item: &mut Option<Entry>,
    current_image: &mut Option<Image>,
    current_author: &mut Option<Person>,
) {
    let text = text.trim().to_string();
    match kind {
        ElementKind::PersonName => {
            if let Some(p) = current_author.as_mut() {
                if !text.is_empty() {
                    p.name = Some(text);
                }
            }
        }
        ElementKind::PersonEmail => {
            if let Some(p) = current_author.as_mut() {
                if !text.is_empty() {
                    p.email = Some(text);
                }
            }
        }
        ElementKind::PersonUri => {
            if let Some(p) = current_author.as_mut() {
                if !text.is_empty() {
                    p.link = Some(text);
                }
            }
        }
        ElementKind::Item => {
            if let Some(item) = current_item.take() {
                feed.entries.push(item);
            }
        }
        ElementKind::ImageBlock => {
            if let Some(img) = current_image.take() {
                if !img.url.is_empty() {
                    feed.image = Some(img);
                }
            }
        }
        ElementKind::FeedTitle => set_if_empty(&mut feed.title, text),
        ElementKind::FeedLink => set_if_empty(&mut feed.link, text),
        ElementKind::FeedDescription => set_if_empty(&mut feed.description, text),
        ElementKind::FeedLanguage => set_if_empty(&mut feed.language, text),
        ElementKind::FeedGenerator => set_if_empty(&mut feed.generator, text),
        ElementKind::FeedPubDate | ElementKind::FeedLastBuildDate => {
            set_if_empty(&mut feed.updated, date::normalize(&text).unwrap_or(text));
        }
        ElementKind::FeedCopyright | ElementKind::FeedManagingEditor | ElementKind::FeedWebmaster => {}
        ElementKind::ImageUrl => {
            if let Some(img) = current_image.as_mut() {
                img.url = text;
            }
        }
        ElementKind::ImageTitle => {
            if let Some(img) = current_image.as_mut() {
                img.title = Some(text);
            }
        }
        ElementKind::ImageLink => {
            if let Some(img) = current_image.as_mut() {
                img.link = Some(text);
            }
        }
        ElementKind::ImageWidth => {
            if let Some(img) = current_image.as_mut() {
                img.width = parse_u32(&text);
            }
        }
        ElementKind::ImageHeight => {
            if let Some(img) = current_image.as_mut() {
                img.height = parse_u32(&text);
            }
        }
        ElementKind::ItemTitle => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.title, text);
            }
        }
        ElementKind::ItemLink => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.link, text);
            }
        }
        ElementKind::ItemDescription => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.description, text);
            }
        }
        ElementKind::ItemAuthor => {
            let mut person = current_author.take().unwrap_or_default();
            if person.name.is_none() && person.email.is_none() && !text.is_empty() {
                person = parse_author(&text);
            }
            if person.name.is_some() || person.email.is_some() {
                if let Some(item) = current_item.as_mut() {
                    item.authors.push(person);
                }
            }
        }
        ElementKind::ItemPubDate => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.published, date::normalize(&text).unwrap_or(text));
            }
        }
        ElementKind::ItemComments => {}
        ElementKind::ItemSource => {}
        ElementKind::ContentEncoded => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.content, text);
            }
        }
        ElementKind::DcCreator => {
            if let Some(item) = current_item.as_mut() {
                item.authors.push(Person {
                    name: Some(text),
                    email: None,
                    link: None,
                });
            } else {
                feed.authors.push(Person {
                    name: Some(text),
                    email: None,
                    link: None,
                });
            }
        }
        ElementKind::DcDate => {
            let normalized = date::normalize(&text).unwrap_or(text);
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.published, normalized);
            } else {
                set_if_empty(&mut feed.updated, normalized);
            }
        }
        ElementKind::DcSubject => {
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
        ElementKind::MediaTitle => {
            if let Some(item) = current_item.as_mut() {
                if let Some(last) = item.media.last_mut() {
                    last.title = Some(text);
                }
            }
        }
        ElementKind::MediaDescription => {
            if let Some(item) = current_item.as_mut() {
                if let Some(last) = item.media.last_mut() {
                    last.description = Some(text);
                }
            }
        }
        ElementKind::CategoryPending(scheme) => {
            let cat = Category {
                term: text,
                scheme,
                label: None,
            };
            if let Some(item) = current_item.as_mut() {
                item.categories.push(cat);
            } else {
                feed.categories.push(cat);
            }
        }
        ElementKind::GuidPending(is_permalink) => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.id, text.clone());
                if is_permalink && item.link.is_none() && text.starts_with("http") {
                    item.link = Some(text);
                }
            }
        }
        ElementKind::ItunesAuthor => {
            if let Some(item) = current_item.as_mut() {
                if item.authors.is_empty() {
                    item.authors.push(Person {
                        name: Some(text),
                        email: None,
                        link: None,
                    });
                }
            } else if feed.authors.is_empty() {
                feed.authors.push(Person {
                    name: Some(text),
                    email: None,
                    link: None,
                });
            }
        }
        ElementKind::ItunesSummary => {
            if let Some(item) = current_item.as_mut() {
                set_if_empty(&mut item.description, text);
            } else {
                set_if_empty(&mut feed.description, text);
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

fn parse_author(raw: &str) -> Person {
    // RSS author: "email@host.com (Name)" or "Name" or plain email.
    let raw = raw.trim();
    if let Some(paren) = raw.find('(') {
        let email = raw[..paren].trim().to_string();
        let name_part = raw[paren + 1..].trim_end_matches(')').trim();
        return Person {
            name: if name_part.is_empty() {
                None
            } else {
                Some(name_part.to_string())
            },
            email: if email.is_empty() { None } else { Some(email) },
            link: None,
        };
    }
    if raw.contains('@') && !raw.contains(' ') {
        return Person {
            name: None,
            email: Some(raw.to_string()),
            link: None,
        };
    }
    Person {
        name: Some(raw.to_string()),
        email: None,
        link: None,
    }
}

fn extract_atom_link(e: &quick_xml::events::BytesStart<'_>) -> Option<Link> {
    let mut link = Link::default();
    for a in e.attributes().flatten() {
        match local_name(a.key.as_ref()) {
            b"href" => {
                link.href = a.unescape_value().ok()?.into_owned();
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
    if link.href.is_empty() {
        None
    } else {
        Some(link)
    }
}

fn extract_media_content(e: &quick_xml::events::BytesStart<'_>) -> MediaContent {
    let mut mc = MediaContent::default();
    for a in e.attributes().flatten() {
        match local_name(a.key.as_ref()) {
            b"url" => {
                mc.url = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
            }
            b"type" => {
                mc.type_ = a.unescape_value().ok().map(|v| v.into_owned());
            }
            b"medium" => {
                mc.medium = a.unescape_value().ok().map(|v| v.into_owned());
            }
            b"width" => {
                mc.width = a.unescape_value().ok().and_then(|v| parse_u32(&v));
            }
            b"height" => {
                mc.height = a.unescape_value().ok().and_then(|v| parse_u32(&v));
            }
            b"duration" => {
                mc.duration = a.unescape_value().ok().and_then(|v| parse_u32(&v));
            }
            _ => {}
        }
    }
    mc
}
