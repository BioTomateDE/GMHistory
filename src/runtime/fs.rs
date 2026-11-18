use crate::error::*;
use crate::runtime::Platform;
use crate::util::listdir;
use crate::util::mkdir;
use crate::version::Version;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::Args)]
pub struct DirStructure {
    /// Create a nested directory structure (version/platform)
    /// instead of a flat one joined with an underscore.
    #[arg(long)]
    pub nested: bool,

    /// Put the platform before the version in output directory names.
    ///
    /// Flat: `{platform}_{version}` instead of `{version}_{platform}`.
    /// With `--nested`: `{platform}/{version}` instead of `{version}/{platform}`.
    #[arg(long)]
    pub platform_first: bool,
}

impl DirStructure {
    pub fn resolve(self, root_dir: &Path, version: Version, platform: Platform) -> PathBuf {
        match (self.nested, self.platform_first) {
            (false, false) => root_dir.join(format!("{version}_{platform}")),
            (false, true) => root_dir.join(format!("{platform}_{version}")),
            (true, false) => root_dir.join(version.to_string()).join(platform.to_string()),
            (true, true) => root_dir.join(platform.to_string()).join(version.to_string()),
        }
    }

    pub fn ensure_parent_dir(self, root_dir: &Path, version: Version, platform: Platform) -> MyResult<()> {
        if !self.nested {
            return Ok(());
        }
        let dir = if self.platform_first {
            root_dir.join(platform.to_string())
        } else {
            root_dir.join(version.to_string())
        };
        mkdir(&dir).ctx("Failed to create subdirectory")
    }

    pub fn walk(self, root_dir: &Path) -> MyResult<Vec<RuntimeDir>> {
        walk(self, root_dir).ctx(|| format!("Failed to walk runtimes directory `{}`", root_dir.display()))
    }
}

impl fmt::Display for DirStructure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let nestness = if self.nested { "nested" } else { "flat" };
        let firstness = if self.platform_first { "platform" } else { "version" };
        write!(f, "{{{nestness} filesystem structure with {firstness} first}}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDir {
    pub version: Version,
    pub platform: Platform,
    pub path: PathBuf,
}

fn walk(struc: DirStructure, root_dir: &Path) -> MyResult<Vec<RuntimeDir>> {
    let mut entries = Vec::new();
    for entry in listdir(root_dir)? {
        let filename = entry.filename();
        if struc.nested {
            let mut platform = None;
            let mut version = None;
            if struc.platform_first {
                let plat = Platform::from_str(filename).ctx("Failed to parse filename as platform")?;
                platform = Some(plat);
            } else {
                let ver = Version::try_from(filename).ctx("Failed to parse filename as version")?;
                version = Some(ver);
            }
            get_nested_thingy(&entry.path, &mut entries, version, platform)
                .ctx(|| format!("Failed to walk runtimes subdirectory {entry}"))?;
            continue;
        }

        let (a, b) = filename.split_once('_').ctx("No underscore in flat filename")?;
        let runtime_dir = if struc.platform_first {
            RuntimeDir {
                version: parse_version(b)?,
                platform: parse_platform(a)?,
                path: entry.path,
            }
        } else {
            RuntimeDir {
                version: parse_version(a)?,
                platform: parse_platform(b)?,
                path: entry.path,
            }
        };
        entries.push(runtime_dir);
    }
    Ok(entries)
}

fn get_nested_thingy(
    dir_path: &Path,
    entries: &mut Vec<RuntimeDir>,
    version: Option<Version>,
    platform: Option<Platform>,
) -> MyResult<()> {
    for entry in listdir(dir_path)? {
        if !entry.is_dir() {
            continue;
        }
        let filename = entry.filename();
        let dawg = if let Some(version) = version {
            let platform = parse_platform(filename)?;
            RuntimeDir {
                version,
                platform,
                path: entry.path,
            }
        } else if let Some(platform) = platform {
            let version = parse_version(filename)?;
            RuntimeDir {
                version,
                platform,
                path: entry.path,
            }
        } else {
            unreachable!("<<<LLVM failed to optimize this away>>>")
        };
        entries.push(dawg);
    }
    Ok(())
}

fn parse_platform(filename: &str) -> MyResult<Platform> {
    Platform::try_from(filename).ctx("Failed to parse filename as platform")
}

fn parse_version(filename: &str) -> MyResult<Version> {
    Version::try_from(filename).ctx("Failed to parse filename as version")
}
