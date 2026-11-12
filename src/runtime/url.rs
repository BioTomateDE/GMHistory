use crate::error::*;
use serde::Deserialize;
use std::str::FromStr;
use uuid::Uuid;

pub const DOMAIN: &str = "https://gms.yoyogames.com/";
const DOMAIN_VERY_BAD_NOT_SECURE: &str = "http://gms.yoyogames.com/";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "&str")]
pub struct DownloadUrl {
    pub uuid: Uuid,
}

impl DownloadUrl {
    pub fn to_url(&self) -> String {
        format!("{DOMAIN}{}.zip", self.uuid.hyphenated())
    }
}

impl TryFrom<&str> for DownloadUrl {
    type Error = Error;
    fn try_from(value: &str) -> MyResult<Self> {
        value.parse()
    }
}

impl FromStr for DownloadUrl {
    type Err = Error;
    fn from_str(string: &str) -> MyResult<Self> {
        let path: &str = string
            .strip_prefix(DOMAIN)
            .or_else(|| string.strip_prefix(DOMAIN_VERY_BAD_NOT_SECURE))
            .ctx(|| format!("URL {string:?} does not start with {DOMAIN:?}"))?;

        let uuid: &str = path
            .strip_suffix(".zip")
            .ctx(|| format!("URL {string:?} does not end with `.zip`"))?;
        let uuid = Uuid::parse_str(uuid).ctx(|| format!("Invalid UUID in URL {string:?}"))?;
        Ok(Self { uuid })
    }
}
