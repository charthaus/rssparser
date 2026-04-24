use quick_xml::events::attributes::Attributes;

pub fn local_name(qname: &[u8]) -> &[u8] {
    match qname.iter().position(|&b| b == b':') {
        Some(i) => &qname[i + 1..],
        None => qname,
    }
}

pub fn namespace_prefix(qname: &[u8]) -> Option<&[u8]> {
    qname.iter().position(|&b| b == b':').map(|i| &qname[..i])
}

pub fn attr_value(attrs: Attributes<'_>, target_local: &[u8]) -> Option<String> {
    for a in attrs.flatten() {
        if local_name(a.key.as_ref()) == target_local {
            return a.unescape_value().ok().map(|v| v.into_owned());
        }
    }
    None
}

pub fn parse_u32(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

pub fn parse_u64(s: &str) -> Option<u64> {
    s.trim().parse().ok()
}
