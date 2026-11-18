mod targets;
mod versions;

use self::targets::DOWNLOAD_TARGETS;
use self::versions::ALL_VERSIONS;
use crate::DownloadIdeArgs;
use crate::IdeEvolution;
use crate::asyncslop::join_tasks;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::extract::probe_7zip;
use crate::net::download;
use crate::net::init_client;
use crate::progressbar::DownloadBar;
use crate::progressbar::init_total_bar;
use crate::runner::detect_ide_evolution;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DownloadTarget {
    version: Version,
    content_length: u64,
    url: &'static str,
}

impl DownloadTarget {
    fn filename(self, evolution: IdeEvolution) -> String {
        match evolution {
            IdeEvolution::OriginalInstallers => self.url.rsplit_once('/').unwrap().1.to_owned(),
            IdeEvolution::RenamedInstallers => format!("{}.exe", self.version),
            IdeEvolution::Extract | IdeEvolution::Upgrade => self.version.to_string(), // directory
        }
    }

    fn is_archive(self) -> bool {
        self.url.contains("archive.org") // good enough
    }
}

pub async fn download_ides(args: DownloadIdeArgs) -> MyResult<()> {
    if args.evolution >= IdeEvolution::Extract {
        probe_7zip()?;
    }

    init_client();
    mkdir(&args.output_dir).ctx("Could not create output directory")?;

    let mut downloads: Vec<DownloadTarget> = get_download_targets(&args);
    filter_already_downloaded(&mut downloads, &args.output_dir, args.evolution);
    let count = downloads.len();
    if count == 0 {
        return Ok(());
    }

    if let Some(evo) = detect_ide_evolution(&args.output_dir)?
        && evo != args.evolution
    {
        bail!(
            "Output directory already has {evo:?} downloads; refusing to download {:?} in the same directory",
            args.evolution,
        );
    }

    let total_length: u64 = downloads.iter().map(|d| d.content_length).sum();
    init_total_bar(count, total_length);
    log::info!("Downloading {total_length} total bytes from {count} IDEs");

    let mut tasks = JoinSet::new();
    for download_target in downloads {
        let args = args.clone();
        let _: AbortHandle = tasks.spawn(async move { download_ide(download_target, args).await });
    }
    join_tasks(tasks).await
}

async fn download_ide(download_target: DownloadTarget, args: DownloadIdeArgs) -> MyResult<()> {
    let DownloadTarget {
        version,
        url,
        content_length,
    } = download_target;
    let name = if download_target.is_archive() {
        format!("IDE {version} (archive.org)")
    } else {
        format!("IDE {version}")
    };
    let bar = DownloadBar::new(name, content_length);
    download_ide_impl(download_target, args, &bar)
        .await
        .ctx(|| format!("Failed to download IDE {version} from {url}"))?;
    Ok(())
}

async fn download_ide_impl(download_target: DownloadTarget, args: DownloadIdeArgs, bar: &DownloadBar) -> MyResult<()> {
    let temp_download_file: OpenTempFile = download(download_target.url, &args.output_dir, bar).await?;

    let path = args.output_dir.join(download_target.filename(args.evolution));
    if args.evolution < IdeEvolution::Extract {
        temp_download_file.rename(&path)?;
        return Ok(());
    }

    let archive_dir = TempDir::new(&args.output_dir)?;
    invoke_7zip(temp_download_file.path(), archive_dir.path(), None).await?;

    // Unfortunately the progress bar will have to stay at
    // 100% for a few seconds while 7-Zip is extracting.
    let archive_dir = TempDir::new(&args.output_dir)?;
    invoke_7zip(temp_download_file.path(), archive_dir.path(), None).await?;
    drop(temp_download_file);
    if args.evolution >= IdeEvolution::Upgrade && download_target.version.0 == 1 {
        let tempdir = upgrade_gms1(download_target.version, archive_dir.path(), &args.output_dir).await?;
        tempdir.rename(&path)?;
    } else {
        archive_dir.rename(&path)?;
    }
    Ok(())
}

// TODO: get rid of this function

// TODO: allow version ranges
fn get_download_targets(args: &DownloadIdeArgs) -> Vec<DownloadTarget> {
    if args.versions.is_empty() {
        return DOWNLOAD_TARGETS.to_vec();
    }

    let mut provided_versions = args.versions.clone();
    let total_count = provided_versions.len();
    provided_versions.dedup();
    let duplicate_count = total_count - provided_versions.len();
    if duplicate_count != 0 {
        warn!("{duplicate_count} versions were provided more than once");
    }

    let mut versions: Vec<DownloadTarget> = Vec::new();
    let mut invalid_versions = Vec::new();
    let mut missing_versions = Vec::new();

    for version in provided_versions {
        if let Some(&download_target) = DOWNLOAD_TARGETS.iter().find(|target| target.version == version) {
            versions.push(download_target);
            continue;
        }
        if ALL_VERSIONS.contains(&version) {
            missing_versions.push(version);
        } else {
            invalid_versions.push(version);
        }
        invalid_versions.push(version);
    }

    report_versions(&invalid_versions, "invalid", Level::Warn);
    report_versions(&missing_versions, "missing", Level::Warn);
    versions
}

fn filter_already_downloaded(targets: &mut Vec<DownloadTarget>, output_dir: &Path, evolution: IdeEvolution) {
    let mut existing_versions = Vec::new();
    targets.retain(|target| {
        let path: PathBuf = output_dir.join(target.filename(evolution));
        if path.exists() {
            existing_versions.push(target.version);
            false
        } else {
            true
        }
    });
    report_versions(&existing_versions, "already downloaded", Level::Info);
}

fn report_versions(bad_versions: &[Version], wgat: &'static str, level: Level) {
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

fn gms1_upgrade_zip_path(version: Version, temp_archive_dir: &Path) -> PathBuf {
    let zip_name = if version.2 > 1451 {
        format!("Original-{version}.zip")
    } else {
        "upgrade.zip".to_owned()
    };
    temp_archive_dir
        .join("$LOCALAPPDATA/GameMaker-Studio/UpgradeZip/")
        .join(zip_name)
}

pub async fn upgrade_gms1(version: Version, ide_dir: &Path, output_dir: &Path) -> MyResult<TempDir> {
    // 1.4.1567 will fail here with "invalid password". No idea what the correct password is.
    let upgrade_zip = gms1_upgrade_zip_path(version, ide_dir);
    let archive_dir = TempDir::new(output_dir)?;
    invoke_7zip(&upgrade_zip, archive_dir.path(), Some(GMS1_PASSWORD)).await?;
    Ok(archive_dir)
}
