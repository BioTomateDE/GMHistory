//! This program only downloads Windows IDEs (not macOS).
//!
//! # HTML changelogs
//! * (changelogs 1.0.23) 1.1.412 - 1.1.754: <https://web.archive.org/web/20130123092814/https://store.yoyogames.com/downloads/gm-studio/release-notes-studio.html#:~:text=Version%201.1.412>
//! * 1.1.867 - 1.4.1804: <https://web.archive.org/web/20180829163534/http://store.yoyogames.com/downloads/gm-studio/release-notes-studio.html#:~:text=Version%201.1.867>
//!
//! # 1.4.9999
//! There are multiple revisions of 1.4.9999.
//! For the same download link and page, the changelogs and IDE installer differ slightly for different archive.org snapshots.
//! TODO: Test whether the IDE is actually different (apart from metadata changes)?
//! This program always downloads the oldest available 1.4.9999 revision.
//!
//! | date       | changelog ID   | installer ID |
//! |------------|----------------|----------------|
//! | 2018-08-28 | 20180830234512 | 20180902152118 |
//! | 2018-09-06 | 20180927020445 | 20180902152118 |
//! | 2018-10-04 | 20181225202637 | 20190327151917 |
//!
//! * The **date** is the date specified in the header in the changelog HTML: `/html/body/h1[1]/a[1]`.
//!
//! * The **changelog ID** is the "archive.org timestamp" of the changelog HTML:
//!   `https://web.archive.org/web/{TIMESTAMP}/https://store.yoyogames.com/downloads/gm-studio/release-notes-studio.html`
//!
//! * The **installer ID** is the "archive.org timestamp" of the Windows IDE installer download:
//!   `https://web.archive.org/web/{TIMESTAMP}/http://store.yoyogames.com/downloads/gm-studio/GMStudio-Installer-1.4.9999.exe`
//!
//! This installer timestamp is the same for 2018-08-28 and 2018-09-06;
//! archive.org did unfortunately not record a snapshot between those two 1.4.9999 revisions.
//!
//! **If you have access to more revisions, please let me know!**
//! Contact me via:
//! * Discord: `@biotomate.de` (User ID 1383525574508679228)
//! * Email: `bio@biotomate.dev`
//! * Github/Codeberg Issue/PR
//!
//! Some IDEs are not downloadable via archive.org or the official website anymore:
//! * GM:S 1 has no downloads before 1.1.690
//! * GMS 2 has no downloads before 2.1.4.295
//!
//! If you have access to these IDEs, also let me know!

use crate::download::Downloader;
use crate::error::*;
use crate::tempslop::{OpenTempFile, new_tempdir};
use crate::util::ignore_exists;
use crate::{
    DownloadIdeArgs,
    extract::{invoke_7zip, probe_7zip},
    password::GMS1_PASSWORD,
    version::Version,
};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct DownloadTarget {
    version: Version,
    url: String,
    path: PathBuf,
}
pub async fn download_ides(args: DownloadIdeArgs) -> MyResult<()> {
    if args.extract && !probe_7zip() {
        bail!("You need to install 7-Zip (command 7z not found)");
    }

    if args.upgrade && !args.extract {
        warn!("Option --upgrade has no effect without --extract");
    }

    ignore_exists(std::fs::create_dir(&args.output_dir))
        .ctx("Could not create output directory")?;

    let downloads: Vec<DownloadTarget> = get_download_targets(&args);
    let count = downloads.len();
    println!("Downloading {count} IDEs");

    let downloader = Downloader::new().ctx("Failed to create downloader")?;

    // TODO: concurrency
    for (i, download_target) in downloads.iter().enumerate() {
        let DownloadTarget { version, url, path } = &download_target;
        println!("{}/{count} Downloading IDE {version} from {url}", i + 1);
        let temp_download_file: OpenTempFile = downloader
            .download(url, &args.output_dir)
            .await
            .ctx(|| format!("Failed to download IDE {version} from {url}"))?;

        if !args.extract {
            temp_download_file.rename(path)?;
            continue;
        }

        let archive_dir = new_tempdir(&args.output_dir)?;
        invoke_7zip(temp_download_file.path(), archive_dir.path(), None)?;
        drop(temp_download_file);

        if args.upgrade && version.0 == 1 {
            let another_tempdir = new_tempdir(&args.output_dir)?;
            let upgrade_zip = gms1_upgrade_zip_path(*version, archive_dir.path());
            invoke_7zip(&upgrade_zip, another_tempdir.path(), Some(GMS1_PASSWORD))?;
            continue;
        }
        archive_dir.rename(path)?;
    }

    println!("All done!");
    Ok(())
}

// TODO: allow version ranges
// TODO: holy fuck this is so bad
fn get_download_targets(args: &DownloadIdeArgs) -> Vec<DownloadTarget> {
    if args.versions.is_empty() {
        let mut existing_versions = Vec::new();
        let mut versions: Vec<DownloadTarget> = Vec::new();
        for &version in Version::ALL {
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
        report_bad_versions(&existing_versions, "already downloaded");
        return versions;
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
    let mut existing_versions = Vec::new();

    for version in provided_versions {
        if !Version::ALL.contains(&version) {
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

    report_bad_versions(&invalid_versions, "invalid");
    report_bad_versions(&missing_versions, "missing");
    report_bad_versions(&existing_versions, "already downloaded");

    versions
}

fn version_get_path(version: Version, url: &str, args: &DownloadIdeArgs) -> PathBuf {
    let name: String = if args.extract {
        version.to_string() // directory
    } else if args.keep_name {
        url.rsplit_once('/').unwrap().1.to_owned()
    } else {
        format!("{version}.exe")
    };
    args.output_dir.join(name)
}

fn report_bad_versions(bad_versions: &[Version], wgat: &'static str) {
    use core::fmt::Write;
    let count = bad_versions.len();
    if count == 0 {
        return;
    }
    let s = if count == 1 { "s" } else { "" };
    let mut buffer = format!("Skipping {count} {wgat} version{s}: ");
    for ver in bad_versions {
        let _ = write!(buffer, "{ver}, ");
    }
    _ = buffer.pop();
    _ = buffer.pop();
    warn!("{buffer}");
}

/// Some versions weren't recorded by archive.org :(
/// If you have access to these versions, let me know!
#[allow(clippy::match_like_matches_macro, reason = "matches! is ugly")]
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
        format!(
            "https://web.archive.org/web/0/http://gms.yoyogames.com/GameMakerStudio-Installer-{version}.exe"
        )
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
