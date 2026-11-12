// `http://store.yoyogames.com/downloads/gm-studio/update-studio.rss`
// contains links to yydelta files.
// This could be used for faster downloading perhaps.
use crate::error::*;
use crate::runtime::Platform;
use crate::runtime::url::DownloadUrl;
use crate::version::Version;
use serde::Deserialize;

fn parse_xml<'de, T: Deserialize<'de>>(xml: &'de str) -> MyResult<T> {
    let mut de = ::quick_xml::de::Deserializer::from_str(xml);
    ::serde_path_to_error::deserialize(&mut de)
        .map_err(|e| format!("{} at `{}`", e.inner(), e.path()))
        .map_err(|e| Error::new("Failed to parse RSS manifest").ctx(e))
}

impl core::str::FromStr for Rss {
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
    #[serde(default, rename = "item")]
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub enclosure: Enclosure,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Enclosure {
    // TODO: make sure to use this too???
    #[serde(rename = "@url")]
    pub url: DownloadUrl,

    #[serde(rename = "@version")] // sparkle
    pub version: Version,

    #[serde(default, rename = "module")]
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    #[serde(rename = "@name")]
    pub name: Platform,

    #[serde(rename = "@url")]
    pub url: DownloadUrl,

    /// Usually a GameMaker version, but for `mac.mono` it's some other
    /// version instead (4.8.0.520, 5.10.1.57 or 6.10.0.104).
    #[serde(rename = "@version")] // sparkle
    pub _version: Version,
}
