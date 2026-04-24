use std::fmt;

#[derive(Debug)]
pub enum ParseError {
    Xml(String),
    NotAFeed,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Xml(m) => write!(f, "XML parse error: {m}"),
            ParseError::NotAFeed => write!(f, "input is not a recognized feed"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<quick_xml::Error> for ParseError {
    fn from(e: quick_xml::Error) -> Self {
        ParseError::Xml(e.to_string())
    }
}
