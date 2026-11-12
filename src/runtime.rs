mod fs;
mod lost;
mod password;
mod platform;
pub mod rss;
mod url;

use self::fs::FileStructure;
use self::lost::LOST_RUNTIMES;
use self::password::generate_password;
pub use self::platform::Platform;
use self::rss::Rss;
use self::url::DownloadUrl;
use crate::DownloadRuntimeArgs;
use crate::net::download;
use crate::net::fetch_text;
use crate::net::init_client;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::tempslop::OpenTempFile;
use crate::tempslop::TempDir;
use crate::util::mkdir;
use crate::version::Version;
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

pub async fn download_runtimes(args: DownloadRuntimeArgs) -> MyResult<()> {
    if args.list_platforms {
        for plat in Platform::VARIANTS {
            println!("{plat}");
        }
        return Ok(());
    }

    if args.platforms.contains(&Platform::MacMono) {
        warn!("This tool does not support downloading mac.mono; see the GitHub README for download links");
    }
    validate_versions(&args.versions);
    init_client();

    let rss: Rss = fetch_manifest().await?;

    let download_targets: Box<[DownloadTarget]> = get_download_targets(rss, &args.versions, &args.platforms)?;
    let count = download_targets.len();
    println!("Got {count} download targets");

    mkdir(&args.output_dir).ctx("Failed to create root output directory")?;

    let file_structure = FileStructure::from_args(&args);
    yap_existing_structure(&args.output_dir, file_structure)?;

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
        let open_temp_file: OpenTempFile = download(&url, &args.output_dir)
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
            if platform == Platform::MacMono {
                continue;
            }
            // no platforms specified -> download all
            if !platforms.is_empty() && !platforms.contains(&platform) {
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

        if !platforms.is_empty() && !platforms.contains(&Platform::Root) {
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

async fn fetch_manifest() -> MyResult<Rss> {
    println!("Fetching manifest Zeus-Runtime.rss");
    let text: String = fetch_text("https://gms.yoyogames.com/Zeus-Runtime.rss")
        .await
        .ctx("Failed to fetch Zeus-Runtime.rss manifest")?;
    text.parse::<Rss>().ctx("Failed to parse Zeus-Runtime.rss XML")
}

fn validate_versions(versions: &[Version]) {
    use core::fmt::Write;

    let mut invalids: Vec<Version> = Vec::new();
    for ver in versions {
        if !Version::ALL_RUNTIMES.contains(ver) {
            invalids.push(*ver);
        }
    }
    if invalids.is_empty() {
        return;
    }

    let count = invalids.len();
    if count == 1 {
        warn!(
            "Provided version {} is not known to be a valid GameMaker runtime version",
            invalids[0]
        );
        return;
    }

    let mut buffer = format!("{count} provided versions are not known to be valid GameMaker runtime versions: ");
    for ver in invalids {
        let _ = write!(buffer, "{ver}, ");
    }
    _ = buffer.pop();
    _ = buffer.pop();
    warn!("{buffer}");
}

/// over-engineered ux stuff or something
fn yap_existing_structure(root_dir: &Path, provided_struc: FileStructure) -> MyResult<()> {
    match detect_existing_structure(root_dir) {
        Ok(Some(detected_struc)) if detected_struc != provided_struc => {
            // could be a warning ig
            let error = err!("Existing {detected_struc} does not match with provided {provided_struc}");
            let note = "Continuing would mess up the nice structure. Either fix your options or choose a different output directory.";
            return Err(error.ctx(note));
        }
        Ok(_) => {}
        Err(e) => warn!(e, "Failed to detect existing file structure"),
    }
    Ok(())
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
    if is_version(name) {
        return Some(FileStructure::NESTED_VERSION_FIRST);
    }
    if is_platform(name) {
        return Some(FileStructure::NESTED_PLATFORM_FIRST);
    }

    let (a, b) = name.split_once('_')?;
    if is_version(a) && is_platform(b) {
        return Some(FileStructure::FLAT_VERSION_FIRST);
    }
    if is_platform(a) && is_version(b) {
        return Some(FileStructure::FLAT_PLATFORM_FIRST);
    }

    None
}

fn is_version(name: &str) -> bool {
    name.parse::<Version>().is_ok()
}

fn is_platform(name: &str) -> bool {
    name.parse::<Platform>().is_ok()
}
