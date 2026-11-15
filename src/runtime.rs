pub mod fs;
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
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::net::download;
use crate::net::fetch_text;
use crate::net::init_client;
use crate::progressbar::DownloadBar;
use crate::tempslop::OpenTempFile;
use crate::tempslop::TempDir;
use crate::util::mkdir;
use crate::version::Version;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use strum::VariantArray;
use tokio::task::AbortHandle;
use tokio::task::JoinSet;

// TODO: use `dsaSignature` and `length` for validation maybe?
#[derive(Debug, Clone)]
struct DownloadTarget {
    version: Version,
    platform: Platform,
    download_url: DownloadUrl,
}

pub async fn download_runtimes(mut args: DownloadRuntimeArgs) -> MyResult<()> {
    if args.list_platforms {
        for plat in Platform::VARIANTS {
            println!("{plat}");
        }
        eprintln!();
        eprintln!("Note: `ROOT` isn't a real platform; it refers to the download URL in the RSS enclosure tag itself");
        // honestly i havent looked in to what this thing actually stores/represents
        return Ok(());
    }

    if args.platforms.contains(&Platform::MacMono) {
        warn!("This tool does not support downloading mac.mono; see the GitHub README for download links");
    }
    validate_versions(&args.versions);
    init_client();

    let rss: Rss = fetch_manifest().await?;

    let download_targets: Box<[DownloadTarget]> = get_download_targets(rss, &args.versions, &args.platforms)?;
    args.versions.clear();
    let count = download_targets.len();
    crate::progressbar::set_download_count(count);
    log::info!("Got {count} download targets");

    mkdir(&args.output_dir).ctx("Failed to create root output directory")?;
    yap_existing_structure(&args.output_dir, args.file_structure)?;

    let mut tasks = JoinSet::new();
    let args = Arc::new(args);
    for download_target in download_targets {
        let args = Arc::clone(&args);
        let _: AbortHandle = tasks.spawn(async move { download_runtime(download_target, args).await });
    }

    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::log!(log::Level::Error, "{e}"),
            Err(e) => bail!(e, "Task panicked"),
        }
    }

    log::info!("All done!");
    Ok(())
}

async fn download_runtime(download_target: DownloadTarget, args: Arc<DownloadRuntimeArgs>) -> MyResult<()> {
    let DownloadTarget { version, platform, .. } = download_target;
    let name = format!("{version} {platform}");
    let bar = DownloadBar::new(name);
    download_runtime_impl(download_target, args, &bar)
        .await
        .ctx(|| format!("Failed to download runtime {version} for {platform}"))?;
    Ok(())
}

async fn download_runtime_impl(
    download_target: DownloadTarget,
    args: Arc<DownloadRuntimeArgs>,
    bar: &DownloadBar,
) -> MyResult<()> {
    let DownloadTarget {
        version,
        platform,
        download_url,
    } = download_target;

    let output_dir: PathBuf = args.file_structure.resolve(&args.output_dir, version, platform);
    if output_dir.exists() {
        return Ok(());
    }

    let url = download_url.to_url();
    let open_temp_file: OpenTempFile = download(&url, &args.output_dir, bar)
        .await
        .ctx(|| format!("Failed to download {version} for {platform} from {url}"))?;

    let archive_dir = TempDir::new(&args.output_dir)?;
    let password: String = generate_password(download_url.uuid);
    log::debug!("Extracting {version} ({platform}) with password {password:?}");
    invoke_7zip(open_temp_file.path(), archive_dir.path(), Some(&password))
        .await
        .ctx(|| format!("Failed to extract {version} ({platform}) with password {password:?}"))?;
    drop(open_temp_file);

    args.file_structure
        .ensure_parent_dir(&args.output_dir, version, platform)?;
    archive_dir.rename(&output_dir)?;
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

            if LOST_RUNTIMES.contains(&download_url.uuid) {
                continue;
            }

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
    log::info!("Fetching manifest Zeus-Runtime.rss");
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
            bail!("Non UTF-8 subdirectory name");
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
        return Some(FileStructure {
            nested: true,
            platform_first: false,
        });
    }
    if is_platform(name) {
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
            platform_first: false,
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
