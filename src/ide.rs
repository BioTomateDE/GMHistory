use crate::DownloadIdeArgs;
use crate::IdeEvolution;
use crate::asyncslop::join_tasks;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::extract::probe_7zip;
use crate::net::download;
use crate::net::init_client;
use crate::progressbar::DownloadBar;
use crate::tempslop::OpenTempFile;
use crate::tempslop::TempDir;
use crate::util::mkdir;
use crate::version::Version;
use log::Level;
use std::path::Path;
use std::path::PathBuf;
use tokio::task::AbortHandle;
use tokio::task::JoinSet;

/// Hardcoded password for all GM:S 1 upgrade ZIPs.
///
/// Datamined from `GameMaker-Studio.exe` lol
pub const GMS1_PASSWORD: &str = "12#_p@o3w$ir_ADD-_$#";

#[derive(Debug)]
struct DownloadTarget {
    version: Version,
    url: String,
    path: PathBuf,
}

pub async fn download_ides(mut args: DownloadIdeArgs) -> MyResult<()> {
    if args.evolution >= IdeEvolution::Extract {
        probe_7zip()?;
    }

    init_client();
    mkdir(&args.output_dir).ctx("Could not create output directory")?;

    let downloads: Vec<DownloadTarget> = get_download_targets(&mut args); // clears args.versions
    let count = downloads.len();
    if count == 0 {
        return Ok(());
    }
    log::info!("Downloading {count} IDEs");
    crate::progressbar::set_download_count(count);

    let mut tasks = JoinSet::new();
    for download_target in downloads {
        let args = args.clone();
        let _: AbortHandle = tasks.spawn(async move { download_ide(download_target, args).await });
    }
    join_tasks(tasks).await
}

async fn download_ide(download_target: DownloadTarget, args: DownloadIdeArgs) -> MyResult<()> {
    let DownloadTarget { version, url, path: _ } = &download_target;
    let name = if url.contains("archive.org") {
        // good enough for me
        format!("IDE {version} (archive.org)")
    } else {
        format!("IDE {version}")
    };
    let bar = DownloadBar::new(name);
    download_ide_impl(&download_target, args, &bar)
        .await
        .ctx(|| format!("Failed to download IDE {version} from {url}"))?;
    Ok(())
}

async fn download_ide_impl(download_target: &DownloadTarget, args: DownloadIdeArgs, bar: &DownloadBar) -> MyResult<()> {
    let DownloadTarget { version, url, path } = &download_target;
    let temp_download_file: OpenTempFile = download(url, &args.output_dir, bar)
        .await
        .ctx(|| format!("Failed to download IDE {version} from {url}"))?;

    if args.evolution < IdeEvolution::Extract {
        temp_download_file.rename(path)?;
        return Ok(());
    }

    // Unfortunately the progress bar will have to stay at
    // 100% for a few seconds while 7-Zip is extracting.
    let archive_dir = TempDir::new(&args.output_dir)?;
    invoke_7zip(temp_download_file.path(), archive_dir.path(), None).await?;
    drop(temp_download_file);

    if args.evolution == IdeEvolution::Upgrade && version.0 == 1 {
        let another_tempdir = TempDir::new(&args.output_dir)?;
        let upgrade_zip = gms1_upgrade_zip_path(*version, archive_dir.path());
        invoke_7zip(&upgrade_zip, another_tempdir.path(), Some(GMS1_PASSWORD)).await?;
    } else {
        archive_dir.rename(path)?;
    }
    Ok(())
}

// TODO: allow version ranges
// TODO: holy fuck this is so bad
fn get_download_targets(args: &mut DownloadIdeArgs) -> Vec<DownloadTarget> {
    if args.versions.is_empty() {
        let mut existing_versions = Vec::new();
        let mut versions: Vec<DownloadTarget> = Vec::new();
        for &version in Version::ALL_IDES {
            if known_missing_version(version) {
                continue;
            }
            let url = get_url(version);
            let path = version_get_path(version, &url, args);
            if path.exists() {
                existing_versions.push(version);
            } else {
                versions.push(DownloadTarget { version, url, path });
            }
        }
        report_bad_versions(&existing_versions, "already downloaded", Level::Info);
        return versions;
    }

    let mut provided_versions = core::mem::take(&mut args.versions);
    let total_count = provided_versions.len();
    provided_versions.dedup();
    let duplicate_count = total_count - provided_versions.len();
    if duplicate_count != 0 {
        warn!("{duplicate_count} versions were provided more than once");
    }

    let mut versions: Vec<DownloadTarget> = Vec::new();
    let mut invalid_versions = Vec::new();
    let mut missing_versions = Vec::new();
    let mut existing_versions = Vec::new();

    for version in provided_versions {
        if !Version::ALL_IDES.contains(&version) {
            invalid_versions.push(version);
            continue;
        }
        if known_missing_version(version) {
            missing_versions.push(version);
            continue;
        }

        let url: String = get_url(version);
        let path = version_get_path(version, &url, args);
        if path.exists() {
            existing_versions.push(version);
        } else {
            versions.push(DownloadTarget { version, url, path });
        }
    }

    report_bad_versions(&invalid_versions, "invalid", Level::Warn);
    report_bad_versions(&missing_versions, "missing", Level::Warn);
    report_bad_versions(&existing_versions, "already downloaded", Level::Info);

    versions
}

fn version_get_path(version: Version, url: &str, args: &DownloadIdeArgs) -> PathBuf {
    let name: String = match args.evolution {
        IdeEvolution::OriginalInstallers => url.rsplit_once('/').unwrap().1.to_owned(),
        IdeEvolution::RenamedInstallers => format!("{version}.exe"),
        IdeEvolution::Extract | IdeEvolution::Upgrade => version.to_string(), // directory
    };
    args.output_dir.join(name)
}

fn report_bad_versions(bad_versions: &[Version], wgat: &'static str, level: Level) {
    use core::fmt::Write;
    const MAX: usize = 12;
    let count = bad_versions.len();
    if count == 0 {
        return;
    }
    let s = if count == 1 { "" } else { "s" };
    let mut string = format!("Skipping {count} {wgat} version{s}: ");
    for ver in bad_versions.iter().take(MAX) {
        _ = write!(string, "{ver}, ");
    }
    _ = string.pop();
    _ = string.pop();
    if count > MAX {
        _ = write!(string, " and {} more", count - MAX);
    }
    log::log!(level, "{string}");
}

/// Some versions weren't recorded by archive.org :(
/// If you have access to these versions, let me know!
const fn known_missing_version(ver: Version) -> bool {
    match ver {
        Version(1, _, ..690, _) => true,
        Version(1, 2, 1262, _) | Version(1, 3, 1312, _) | Version(2, 0, 3, 56) => true,
        _ => false,
    }
}

/// Some installer downloads were captured before they were actually released.
/// This causes the earliest snapshot to be a 404 page instead of a download.
#[expect(clippy::unreadable_literal)]
const fn override_timestamp(ver: Version) -> Option<u64> {
    Some(match ver {
        Version(1, 1, 1013, _) => 20140721060018,
        Version(1, 1, 1044 | 1058, _) => 20160325114921,
        Version(1, 1, 1076, _) => 20140721043634,
        _ => return None,
    })
}

fn get_url(version: Version) -> String {
    if version.0 == 1 {
        // A zero in the archive.org URL expands to the oldest recorded snapshot.
        let timestamp: u64 = override_timestamp(version).unwrap_or(0);
        format!(
            "https://web.archive.org/web/{timestamp}/http://store.yoyogames.com/downloads/gm-studio/GMStudio-Installer-{version}.exe"
        )
    } else if version.0 == 2 && version.3 < 295 {
        format!("https://web.archive.org/web/0/http://gms.yoyogames.com/GameMakerStudio-Installer-{version}.exe")
    } else if version.0 == 2 || (version.0 == 2022 && version.1 < 3) {
        // The earliest post-Studio releases (2022.1.0.609, 2022.1.1.610, 2022.2.0.614, 2022.2.1.618) still have Studio in their download URL.
        format!("https://gms.yoyogames.com/GameMakerStudio-Installer-{version}.exe")
        // macOS: "https://gms.yoyogames.com/GameMakerStudio2-{version}.pkg"
    } else {
        format!("https://gms.yoyogames.com/GameMaker-Installer-{version}.exe")
        // macOS: "https://gms.yoyogames.com/GameMaker-{version}.pkg"
        // HTML overview: <https://gms.yoyogames.com/ReleaseNotes.html>
    }
}

fn gms1_upgrade_zip_path(version: Version, temp_archive_dir: &Path) -> PathBuf {
    let zip_name = if version.3 > 1451 {
        format!("Original-{version}.zip")
    } else {
        "upgrade.zip".to_owned()
    };
    temp_archive_dir
        .join("$LOCALAPPDATA/GameMaker-Studio/UpgradeZip/")
        .join(zip_name)
}
