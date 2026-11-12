use crate::error::*;
use crate::version::Version;
use serde::Deserialize;
use std::str::FromStr;
use uuid::Uuid;

pub const DOMAIN: &str = "https://gms.yoyogames.com/";
const DOMAIN_VERY_BAD_NOT_SECURE: &str = "http://gms.yoyogames.com/";
const MACMONO_PREFIX: &str = "MonoFramework-MDK-";
const MACMONO_SUFFIX: &str = ".macos10.xamarin.universal.pkg";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "&str")]
pub enum DownloadUrl {
    GameMaker(Uuid),
    MacMono(Version),
}

impl DownloadUrl {
    pub fn to_url(&self) -> String {
        match self {
            Self::GameMaker(uuid) => format!("{DOMAIN}{}.zip", uuid.hyphenated()),
            Self::MacMono(version) => format!("{DOMAIN}{MACMONO_PREFIX}{version}{MACMONO_SUFFIX}"),
        }
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

        if let Some(version) = path.strip_circumfix(MACMONO_PREFIX, MACMONO_SUFFIX) {
            let version = Version::try_from(version).ctx("Bad mac mono version")?;
            return Ok(Self::MacMono(version));
        }

        let uuid: &str = path
            .strip_suffix(".zip")
            .ctx(|| format!("URL {string:?} does not end with `.zip`"))?;
        let uuid = Uuid::parse_str(uuid).ctx(|| format!("Invalid UUID in URL {string:?}"))?;
        Ok(Self::GameMaker(uuid))
    }
}
