mod lost;
mod password;
mod platform;
pub mod rss;
mod url;

use self::lost::LOST_RUNTIMES;
use self::password::generate_password;
pub use self::platform::Platform;
use self::rss::Rss;
use self::url::DownloadUrl;
use crate::DownloadRuntimeArgs;
use crate::download::Downloader;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::tempslop::OpenTempFile;
use crate::tempslop::TempDir;
use crate::util::mkdir;
use crate::version::Version;
use reqwest::Response;
use std::path::Path;
use std::path::PathBuf;
use strum::VariantArray;

// TODO: use `dsaSignature` and `length` for validation maybe?
#[derive(Debug, Clone)]
struct DownloadTarget {
    version: Version,
    platform: Platform,
    download_url: DownloadUrl,
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
        mkdir(&dir).ctx("Failed to create subdirectory")
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

    let download_targets: Box<[DownloadTarget]> = get_download_targets(rss, &args.versions, platforms)?;
    let count = download_targets.len();
    println!("Got {count} download targets");

    mkdir(&args.output_dir).ctx("Failed to create root output directory")?;

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
    for (i, download_target) in download_targets.into_iter().enumerate() {
        let DownloadTarget {
            version,
            platform,
            download_url,
        } = download_target;

        if LOST_RUNTIMES.contains(&download_url.uuid) {
            println!(
                "Runtime {version} for {platform} ({}) is known to be missing from the CDN; skipping it",
                download_url.uuid.hyphenated()
            );
            continue;
        }

        let output_dir: PathBuf = file_structure.resolve(&args.output_dir, version, platform);
        if output_dir.exists() {
            continue;
        }

        let url = download_url.to_url();
        println!("({}/{}) Downloading {version} for {platform} from {url}", i + 1, count);
        let open_temp_file: OpenTempFile = downloader
            .download(&url, &args.output_dir)
            .await
            .ctx(|| format!("Failed to download {version} for {platform} from {url}"))?;

        let archive_dir = TempDir::new(&args.output_dir)?;
        let password: String = generate_password(download_url.uuid);
        println!("Extracting {version} ({platform}) with password {password:?}");
        invoke_7zip(open_temp_file.path(), archive_dir.path(), Some(&password))
            .ctx(|| format!("Failed to extract {version} ({platform}) with password {password:?}"))?;
        drop(open_temp_file);

        file_structure.ensure_parent_dir(&args.output_dir, version, platform)?;
        archive_dir.rename(&output_dir)?;
    }

    Ok(())
}

fn get_download_targets(rss: Rss, versions: &[Version], platforms: &[Platform]) -> MyResult<Box<[DownloadTarget]>> {
    let mut targets: Vec<DownloadTarget> = Vec::new();

    for item in rss.channel.items {
        let version = item.enclosure.version;
        // no versions specified -> download all
        if !versions.is_empty() && !versions.contains(&version) {
            continue;
        }

        for module in item.enclosure.modules {
            let platform = module.name;
            if platform == Platform::MacMono || !platforms.contains(&platform) {
                continue;
            }

            let download_url = DownloadUrl::try_from(module.url.as_str())
                .ctx(|| format!("Failed to parse download URL for {version} {platform}"))?;
            let target = DownloadTarget {
                version,
                platform,
                download_url,
            };
            targets.push(target);
        }

        if !platforms.contains(&Platform::Root) {
            continue;
        }
        let target = DownloadTarget {
            version,
            platform: Platform::Root,
            download_url: item.enclosure.url,
        };
        targets.push(target);
    }

    Ok(targets.into_boxed_slice())
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
