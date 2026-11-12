mod ide;
mod runtime;

use crate::error::*;
use core::fmt;
use serde::Deserialize;
use std::num::IntErrorKind;

/// A GameMaker version.
///
/// The first number is the major; either 1, 2, 2022, 2023 or 2024.
/// For GM:S 1, the last number is always stored as zero and not printed.
///
/// Note: This can also be a `mac.mono` version in the RSS thingy because of woke,
///       but that garbage is discarded and never used in methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "&str")]
pub struct Version(pub u16, pub u16, pub u16, pub u16);

impl Version {
    pub const ALL_IDES: &[Self] = self::ide::ALL;
    pub const ALL_RUNTIMES: &[Self] = self::runtime::ALL;
}

impl TryFrom<&str> for Version {
    type Error = Error;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        parse_version(string).ctx(|| format!("Failed to parse version {string:?}"))
    }
}

impl core::str::FromStr for Version {
    type Err = String;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        parse_version(string).map_err(|e| e.message)
    }
}

fn parse_version(string: &str) -> MyResult<Version> {
    let parts = &mut string.split('.');
    let p1: u16 = parse_part(parts)?;
    let p2: u16 = parse_part(parts)?;
    let p3: u16 = parse_part(parts)?;
    let p4 = if p1 == 1 { 0 } else { parse_part(parts)? };
    if parts.next().is_some() {
        bail!("Too many version parts");
    }
    Ok(Version(p1, p2, p3, p4))
}

fn parse_part<'a>(parts: &mut impl Iterator<Item = &'a str>) -> MyResult<u16> {
    let part: &str = parts.next().ctx("Not enough version parts (need 3 or 4)")?;
    match part.parse() {
        Ok(int) => Ok(int),
        Err(e) if *e.kind() == IntErrorKind::PosOverflow => bail!("Version part {part:?} too large"),
        _ => bail!("Non-numeric version part {part:?}"),
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if *self == Self(2024, 6, 2, 162) {
            // fuck you yoyo games
            return write!(f, "2024.06.2.162");
        }

        if self.0 == 1 {
            debug_assert_eq!(self.3, 0, "GM:S 1 only has 3 version parts");
            write!(f, "{}.{}.{}", self.0, self.1, self.2)
        } else {
            write!(f, "{}.{}.{}.{}", self.0, self.1, self.2, self.3)
        }
    }
}
