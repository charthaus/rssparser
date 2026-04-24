#[derive(Debug, Default, Clone)]
pub struct Feed {
    pub title: Option<String>,
    pub link: Option<String>,
    pub links: Vec<Link>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub generator: Option<String>,
    pub updated: Option<String>,
    pub id: Option<String>,
    pub image: Option<Image>,
    pub icon: Option<String>,
    pub logo: Option<String>,
    pub authors: Vec<Person>,
    pub categories: Vec<Category>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Default, Clone)]
pub struct Entry {
    pub title: Option<String>,
    pub link: Option<String>,
    pub links: Vec<Link>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub id: Option<String>,
    pub authors: Vec<Person>,
    pub categories: Vec<Category>,
    pub enclosures: Vec<Enclosure>,
    pub media: Vec<MediaContent>,
}

#[derive(Debug, Default, Clone)]
pub struct Link {
    pub href: String,
    pub rel: Option<String>,
    pub type_: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Person {
    pub name: Option<String>,
    pub email: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Category {
    pub term: String,
    pub scheme: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Enclosure {
    pub url: String,
    pub length: Option<u64>,
    pub type_: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct MediaContent {
    pub url: String,
    pub type_: Option<String>,
    pub medium: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<u32>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Image {
    pub url: String,
    pub title: Option<String>,
    pub link: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
