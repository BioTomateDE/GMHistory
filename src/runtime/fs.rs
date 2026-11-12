use crate::DownloadRuntimeArgs;
use crate::error::*;
use crate::runtime::Platform;
use crate::util::mkdir;
use crate::version::Version;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileStructure {
    nested: bool,
    platform_first: bool,
}

impl FileStructure {
    pub const FLAT_VERSION_FIRST: Self = Self {
        nested: false,
        platform_first: false,
    };
    pub const NESTED_VERSION_FIRST: Self = Self {
        nested: true,
        platform_first: false,
    };
    pub const FLAT_PLATFORM_FIRST: Self = Self {
        nested: false,
        platform_first: true,
    };
    pub const NESTED_PLATFORM_FIRST: Self = Self {
        nested: true,
        platform_first: true,
    };

    pub const fn from_args(args: &DownloadRuntimeArgs) -> Self {
        Self {
            nested: args.nested,
            platform_first: args.platform_first,
        }
    }

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
}

impl fmt::Display for FileStructure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let nestness = if self.nested { "Nested" } else { "Flat" };
        let firstness = if self.platform_first { "platform" } else { "version" };
        write!(f, "{nestness} file structure with {firstness} first")
    }
}
