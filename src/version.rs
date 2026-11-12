use crate::error::*;
use core::fmt;
use serde::Deserialize;

/// A GameMaker version.
///
/// The first number is the major; either 1, 2, 2022, 2023 or 2024.
/// For GM:S 1, the last number is always stored as zero and not printed.
///
/// Note: This can also be a `mac.mono` version in the RSS thingy because of woke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "&str")]
pub struct Version(pub u16, pub u16, pub u16, pub u16);

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
    let part: &str = parts.next().ctx("Not enough version parts")?;
    part.parse().map_err(|_| err!("Non-numeric version part {part:?}"))
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

impl Version {
    pub const ALL: &[Self] = ALL_VERSIONS;
}
const ALL_VERSIONS: &[Version] = &[
    Version(1, 0, 23, 0),
    Version(1, 0, 31, 0),
    Version(1, 0, 52, 0),
    Version(1, 0, 83, 0),
    Version(1, 0, 95, 0),
    Version(1, 0, 97, 0),
    Version(1, 0, 98, 0),
    Version(1, 0, 114, 0),
    Version(1, 0, 125, 0),
    Version(1, 0, 128, 0),
    Version(1, 0, 129, 0),
    Version(1, 0, 131, 0),
    Version(1, 0, 142, 0),
    Version(1, 0, 144, 0),
    Version(1, 0, 153, 0),
    Version(1, 0, 183, 0),
    Version(1, 0, 187, 0),
    Version(1, 0, 191, 0),
    Version(1, 0, 198, 0),
    Version(1, 0, 202, 0),
    Version(1, 0, 210, 0),
    Version(1, 0, 217, 0),
    Version(1, 0, 230, 0),
    Version(1, 0, 231, 0),
    Version(1, 0, 240, 0),
    Version(1, 0, 249, 0),
    Version(1, 0, 253, 0),
    Version(1, 0, 263, 0),
    Version(1, 0, 282, 0),
    Version(1, 0, 292, 0),
    Version(1, 0, 298, 0),
    Version(1, 0, 299, 0),
    Version(1, 0, 303, 0),
    Version(1, 0, 305, 0),
    Version(1, 0, 316, 0),
    Version(1, 0, 327, 0),
    Version(1, 0, 342, 0),
    Version(1, 1, 412, 0),
    Version(1, 1, 416, 0),
    Version(1, 1, 450, 0),
    Version(1, 1, 469, 0),
    Version(1, 1, 513, 0),
    Version(1, 1, 522, 0),
    Version(1, 1, 533, 0),
    Version(1, 1, 560, 0),
    Version(1, 1, 609, 0),
    Version(1, 1, 615, 0),
    Version(1, 1, 622, 0),
    Version(1, 1, 666, 0),
    Version(1, 1, 676, 0),
    Version(1, 1, 690, 0),
    Version(1, 1, 694, 0),
    Version(1, 1, 711, 0),
    Version(1, 1, 734, 0),
    Version(1, 1, 750, 0),
    Version(1, 1, 754, 0),
    Version(1, 1, 867, 0),
    Version(1, 1, 872, 0),
    Version(1, 1, 913, 0),
    Version(1, 1, 917, 0),
    Version(1, 1, 929, 0),
    Version(1, 1, 964, 0),
    Version(1, 1, 1013, 0),
    Version(1, 1, 1044, 0),
    Version(1, 1, 1058, 0),
    Version(1, 1, 1076, 0),
    Version(1, 1, 1089, 0),
    Version(1, 2, 1113, 0),
    Version(1, 2, 1130, 0),
    Version(1, 2, 1135, 0),
    Version(1, 2, 1146, 0),
    Version(1, 2, 1150, 0),
    Version(1, 2, 1161, 0),
    Version(1, 2, 1214, 0),
    Version(1, 2, 1220, 0),
    Version(1, 2, 1224, 0),
    Version(1, 2, 1262, 0),
    Version(1, 2, 1264, 0),
    Version(1, 2, 1279, 0),
    Version(1, 3, 1307, 0),
    Version(1, 3, 1312, 0),
    Version(1, 3, 1336, 0),
    Version(1, 3, 1344, 0),
    Version(1, 3, 1347, 0),
    Version(1, 3, 1354, 0),
    Version(1, 3, 1373, 0),
    Version(1, 3, 1379, 0),
    Version(1, 3, 1386, 0),
    Version(1, 4, 1398, 0),
    Version(1, 4, 1451, 0),
    Version(1, 4, 1474, 0),
    Version(1, 4, 1499, 0),
    Version(1, 4, 1514, 0),
    Version(1, 4, 1522, 0),
    Version(1, 4, 1542, 0),
    Version(1, 4, 1556, 0),
    Version(1, 4, 1567, 0),
    Version(1, 4, 1598, 0),
    Version(1, 4, 1629, 0),
    Version(1, 4, 1657, 0),
    Version(1, 4, 1675, 0),
    Version(1, 4, 1683, 0),
    Version(1, 4, 1690, 0),
    Version(1, 4, 1711, 0),
    Version(1, 4, 1749, 0),
    Version(1, 4, 1757, 0),
    Version(1, 4, 1760, 0),
    Version(1, 4, 1763, 0),
    Version(1, 4, 1767, 0),
    Version(1, 4, 1772, 0),
    Version(1, 4, 1773, 0),
    Version(1, 4, 1788, 0),
    Version(1, 4, 1804, 0),
    Version(1, 4, 9999, 0),
    Version(2, 0, 1, 8),
    Version(2, 0, 1, 9),
    Version(2, 0, 1, 10),
    Version(2, 0, 1, 12),
    Version(2, 0, 1, 16),
    Version(2, 0, 1, 22),
    Version(2, 0, 1, 32),
    Version(2, 0, 1, 33),
    Version(2, 0, 2, 44),
    Version(2, 0, 3, 56),
    Version(2, 0, 4, 64),
    Version(2, 0, 4, 65),
    Version(2, 0, 5, 72),
    Version(2, 0, 5, 74),
    Version(2, 0, 5, 76),
    Version(2, 0, 5, 77),
    Version(2, 0, 6, 136),
    Version(2, 0, 6, 146),
    Version(2, 0, 7, 171),
    Version(2, 1, 0, 204),
    Version(2, 1, 0, 207),
    Version(2, 1, 0, 212),
    Version(2, 1, 1, 241),
    Version(2, 1, 2, 257),
    Version(2, 1, 3, 271),
    Version(2, 1, 3, 273),
    Version(2, 1, 4, 285),
    Version(2, 1, 4, 288),
    Version(2, 1, 4, 295),
    Version(2, 1, 5, 322),
    Version(2, 2, 0, 343),
    Version(2, 2, 1, 375),
    Version(2, 2, 2, 413),
    Version(2, 2, 3, 433),
    Version(2, 2, 3, 436),
    Version(2, 2, 4, 458),
    Version(2, 2, 4, 459),
    Version(2, 2, 4, 464),
    Version(2, 2, 4, 474),
    Version(2, 2, 5, 481),
    Version(2, 3, 0, 529),
    Version(2, 3, 1, 536),
    Version(2, 3, 1, 542),
    Version(2, 3, 2, 556),
    Version(2, 3, 2, 558),
    Version(2, 3, 2, 560),
    Version(2, 3, 3, 570),
    Version(2, 3, 3, 574),
    Version(2, 3, 4, 577),
    Version(2, 3, 4, 580),
    Version(2, 3, 4, 583),
    Version(2, 3, 5, 589),
    Version(2, 3, 6, 595),
    Version(2, 3, 7, 603),
    Version(2, 3, 7, 606),
    Version(2022, 1, 0, 609),
    Version(2022, 1, 1, 610),
    Version(2022, 2, 0, 614),
    Version(2022, 2, 1, 618),
    Version(2022, 3, 0, 624),
    Version(2022, 3, 0, 625),
    Version(2022, 5, 0, 8),
    Version(2022, 5, 1, 14),
    Version(2022, 5, 1, 16),
    Version(2022, 6, 0, 23),
    Version(2022, 6, 1, 26),
    Version(2022, 8, 0, 34),
    Version(2022, 8, 1, 37),
    Version(2022, 9, 0, 49),
    Version(2022, 9, 1, 51),
    Version(2022, 11, 0, 54),
    Version(2022, 11, 1, 56),
    Version(2023, 1, 0, 58),
    Version(2023, 1, 1, 62),
    Version(2023, 2, 0, 71),
    Version(2023, 2, 1, 75),
    Version(2023, 4, 0, 84),
    Version(2023, 6, 0, 89),
    Version(2023, 6, 0, 92),
    Version(2023, 8, 0, 98),
    Version(2023, 8, 1, 102),
    Version(2023, 8, 2, 106),
    Version(2023, 8, 2, 108),
    Version(2023, 11, 0, 121),
    Version(2023, 11, 1, 129),
    Version(2024, 2, 0, 132),
    Version(2024, 4, 0, 137),
    Version(2024, 4, 1, 152),
    Version(2024, 6, 0, 157),
    Version(2024, 6, 1, 160),
    Version(2024, 6, 2, 162), // this one specifically needs the 6 zero padded: 2024.06.2.162
    Version(2024, 8, 0, 169),
    Version(2024, 8, 1, 171),
    Version(2024, 11, 0, 179),
    Version(2024, 13, 0, 190),
    Version(2024, 13, 1, 193),
    Version(2024, 14, 0, 207),
    Version(2024, 14, 1, 210),
    Version(2024, 14, 2, 212),
    Version(2024, 14, 2, 213),
    Version(2024, 14, 3, 217),
    Version(2024, 14, 4, 221),
    Version(2024, 14, 4, 222),
];
