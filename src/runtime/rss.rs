#![allow(dead_code, reason = "some fields may not be used")]
use std::str::FromStr;

use crate::{error::*, runtime::Platform, version::Version};
use serde::Deserialize;

fn parse_xml<'de, T: Deserialize<'de>>(xml: &'de str) -> MyResult<T> {
    let mut de = ::quick_xml::de::Deserializer::from_str(xml);
    ::serde_path_to_error::deserialize(&mut de)
        .map_err(|e| format!("{} at `{}`", e.inner(), e.path()))
        .map_err(|e| Error::new("Failed to parse RSS manifest").ctx(e))
}

impl FromStr for Rss {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_xml(s)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rss {
    pub channel: Channel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    /// `GameMaker Runtime Release Update Feed`
    pub title: String,

    /// `http://store.yoyogames.com/downloads/gm-studio/update-studio.rss`
    /// Contains links to yydelta files.
    /// TODO: this could be used for faster downloading perhaps.
    pub link: String,

    /// `GameMaker Runtime Release Update Feed`
    pub description: String,

    /// `en`
    pub language: String,

    #[serde(default, rename = "item")]
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    /// `Version {version}`
    pub title: String,

    /// RFC 2822, e.g. `Tue, 31 Mar 2026 09:24:32 Z`
    #[serde(rename = "pubDate")]
    pub pub_date: String,

    #[serde(rename = "releaseNotesLink")] // sparkle
    pub release_notes_link: String,

    /// `http://gms.yoyogames.com/release-notes-{version}.json>
    pub comments: String,

    /// Main content
    pub enclosure: Enclosure,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Enclosure {
    #[serde(rename = "@url")]
    pub url: String,

    #[serde(rename = "@version")] // sparkle
    pub version: Version,

    #[serde(rename = "@length")]
    pub length: u64,

    #[serde(rename = "@dsaSignature")] // sparkle
    pub dsa_signature: String,

    #[serde(default, rename = "module")]
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    #[serde(rename = "@name")]
    pub name: Platform,

    #[serde(rename = "@url")]
    pub url: String,

    /// Usually a GameMaker version, but for `mac.mono` it's some other
    /// version instead (4.8.0.520, 5.10.1.57 or 6.10.0.104).
    #[serde(rename = "@version")] // sparkle
    pub version: Version,

    #[serde(rename = "@length")]
    pub length: u64,

    #[serde(rename = "@dsaSignature")] // sparkle
    pub dsa_signature: String,
}
