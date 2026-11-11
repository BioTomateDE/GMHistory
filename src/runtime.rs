mod lost;
mod password;
mod platform;
pub mod rss;

use self::password::generate_password;
pub use self::platform::Platform;
use self::rss::Rss;
use crate::DownloadRuntimeArgs;
use crate::download::Downloader;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::tempslop::{OpenTempFile, TempDir};
use crate::util::ignore_exists;
use crate::version::Version;
use reqwest::Response;
use std::path::{Path, PathBuf};
use strum::VariantArray;

// TODO: use `dsaSignature` and `length` for validation maybe?
#[derive(Debug, Clone)]
struct DownloadTarget {
    version: Version,
    platform: Platform,
    url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileStructure {
    nested: bool,
    platform_first: bool,
}

impl FileStructure {
    fn resolve(self, root_dir: &Path, version: Version, platform: Platform) -> PathBuf {
        match (self.nested, self.platform_first) {
            (false, false) => root_dir.join(format!("{version}_{platform}")),
            (false, true) => root_dir.join(format!("{platform}_{version}")),
            (true, false) => root_dir.join(version.to_string()).join(platform.to_string()),
            (true, true) => root_dir.join(platform.to_string()).join(version.to_string()),
        }
    }

    fn ensure_parent_dir(self, root_dir: &Path, version: Version, platform: Platform) -> MyResult<()> {
        if !self.nested {
            return Ok(());
        }
        let dir = if self.platform_first {
            root_dir.join(platform.to_string())
        } else {
            root_dir.join(version.to_string())
        };
        ignore_exists(std::fs::create_dir(dir)).ctx("Failed to create subdirectory")
    }
}

pub async fn download_runtimes(args: DownloadRuntimeArgs) -> MyResult<()> {
    let platforms: &[Platform] = if args.platforms.is_empty() {
        Platform::VARIANTS
    } else {
        &args.platforms
    };

    println!("Fetching manifest Zeus-Runtime.rss");
    let downloader = Downloader::new()?;
    let response: Response = downloader
        .send_request("https://gms.yoyogames.com/Zeus-Runtime.rss")
        .await
        .ctx("Failed to fetch Zeus-Runtime.rss manifest")?;
    let text: String = response.text().await.ctx("Failed to get response text")?;
    let rss: Rss = text.parse()?;

    let download_targets: Box<[DownloadTarget]> = get_download_targets(rss, &args.versions, platforms);
    println!("Got {} download targets", download_targets.len());

    ignore_exists(std::fs::create_dir(&args.output_dir)).ctx("Failed to create root output directory")?;

    let file_structure = FileStructure {
        nested: args.nested,
        platform_first: args.platform_first,
    };
    // over-engineered ux stuff or something
    match detect_existing_structure(&args.output_dir) {
        Ok(Some(struc)) if struc != file_structure => {
            // could be a warning ig
            let error = err!("Existing {struc:?} does not match with provided {file_structure:?}");
            let note = "Continuing would mess up the nice structure. Either fix your options or choose a different output directory.";
            return Err(error.ctx(note));
        }
        Ok(_) => {}
        Err(e) => warn!(e, "Failed to detect existing file structure"),
    }

    // TODO: parallelization
    for download_target in download_targets {
        let DownloadTarget { version, platform, url } = download_target;

        // TODO: skip lost runtimes

        let output_dir: PathBuf = file_structure.resolve(&args.output_dir, version, platform);
        if output_dir.exists() {
            continue;
        }

        let open_temp_file: OpenTempFile = downloader
            .download(&url, &args.output_dir)
            .await
            .ctx(|| format!("Failed to download {version} for {platform:?} from {url}"))?;

        let archive_dir = TempDir::new(&args.output_dir)?;
        let password: String = generate_password(&url);
        println!("Extracting {version} ({platform}) with password {password:?}");
        invoke_7zip(open_temp_file.path(), archive_dir.path(), Some(&password))
            .ctx(|| format!("Failed to extract {version} ({platform}) with password {password:?}"))?;
        drop(open_temp_file);

        file_structure.ensure_parent_dir(&args.output_dir, version, platform)?;
        archive_dir.rename(&output_dir)?;
    }

    Ok(())
}

fn get_filename(filename_or_url: &str) -> &str {
    match filename_or_url.rfind('/') {
        Some(idx) => &filename_or_url[idx + 1..],
        None => filename_or_url,
    }
}

fn get_download_targets(rss: Rss, versions: &[Version], platforms: &[Platform]) -> Box<[DownloadTarget]> {
    let mut targets: Vec<DownloadTarget> = Vec::new();

    for item in rss.channel.items {
        let version = item.enclosure.version;
        // no versions specified -> download all
        if !versions.is_empty() && !versions.contains(&version) {
            continue;
        }
        for module in item.enclosure.modules {
            // `module.version == version` unless it's mac.mono
            if !platforms.contains(&module.name) {
                continue;
            }
            let target = DownloadTarget {
                version,
                platform: module.name,
                url: module.url,
            };
            targets.push(target);
        }
    }

    targets.into_boxed_slice()
}

fn detect_existing_structure(root_dir: &Path) -> MyResult<Option<FileStructure>> {
    for entry in std::fs::read_dir(root_dir).ctx("Failed to read directory")? {
        let entry = entry.ctx("Entry of read_dir is broken")?;

        let ty: std::fs::FileType = entry.file_type().ctx("Failed to get file type of dir entry")?;
        if !ty.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            bail!("Non UTF-8 subdirectory name {name:?}");
        };
        if name.starts_with('.') {
            continue;
        }

        if let Some(struc) = test_structure(name) {
            return Ok(Some(struc));
        }
    }

    Ok(None)
}

fn test_structure(name: &str) -> Option<FileStructure> {
    if name.parse::<Version>().is_ok() {
        return Some(FileStructure {
            nested: true,
            platform_first: false,
        });
    }
    if name.parse::<Platform>().is_ok() {
        return Some(FileStructure {
            nested: true,
            platform_first: true,
        });
    }

    let (a, b) = name.split_once('_')?;
    if is_version(a) && is_platform(b) {
        return Some(FileStructure {
            nested: false,
            platform_first: false,
        });
    }
    if is_platform(a) && is_version(b) {
        return Some(FileStructure {
            nested: false,
            platform_first: true,
        });
    }

    None
}

fn is_version(name: &str) -> bool {
    name.parse::<Version>().is_ok()
}

fn is_platform(name: &str) -> bool {
    name.parse::<Platform>().is_ok()
}
