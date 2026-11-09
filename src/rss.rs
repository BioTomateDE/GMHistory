#![allow(dead_code)]
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Rss {
    pub channel: Channel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub title: String,

    #[serde(default)]
    pub item: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub title: String,

    #[serde(rename = "pubDate")]
    pub pub_date: String,

    #[serde(rename = "sparkle:releaseNotesLink")]
    pub release_notes_link: String,

    pub comments: String,

    pub enclosure: Enclosure,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Enclosure {
    #[serde(rename = "@url")]
    pub url: String,

    #[serde(rename = "@sparkle:version")]
    pub version: String,

    #[serde(rename = "@length")]
    pub length: u64,

    #[serde(rename = "@sparkle:dsaSignature")]
    pub dsa_signature: String,

    #[serde(rename = "module", default)]
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "@url")]
    pub url: String,

    #[serde(rename = "@sparkle:version")]
    pub version: String,

    #[serde(rename = "@length")]
    pub length: u64,

    #[serde(rename = "@sparkle:dsaSignature")]
    pub dsa_signature: String,
}
