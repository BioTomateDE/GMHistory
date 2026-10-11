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

use crate::{
    DownloadIdeArgs, cli_panic,
    error::print_trace,
    extract::{invoke_7zip, probe_7zip},
    password::GMS1_PASSWORD,
    version::Version,
    warn,
};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::blocking::{Client, Response};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Duration,
};

pub fn download_ides(mut args: DownloadIdeArgs) {
    if args.extract && !probe_7zip() {
        cli_panic!("You need to install 7-Zip (command 7z not found)");
    }

    if args.upgrade && !args.extract {
        warn!("Option --upgrade has no effect without --extract");
    }

    match std::fs::create_dir(&args.output_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
        Err(e) => cli_panic!("Could not create output directory: {e}"),
    }

    let versions: &[Version] = if args.versions.is_empty() {
        Version::ALL
    } else {
        args.versions.dedup();
        for ver in &args.versions {
            if !Version::ALL.contains(ver) {
                warn!("WARNING: Version {ver} is not known to be a valid GameMaker IDE version");
            }
        }
        &args.versions
    };

    let total_count = versions.len();
    let versions: Vec<Version> = versions
        .iter()
        .copied()
        .filter(|&v| !known_missing_version(v))
        .collect();
    let count = versions.len();
    println!("Downloading {count}/{total_count} IDEs");

    let mut skip_count: usize = 0;

    let client: Client = reqwest::blocking::ClientBuilder::new()
        .timeout(Duration::from_secs(240))
        .build()
        .expect("failed to build reqwest client");

    let temp_dir_path = args.output_dir.join(".archive.tmp");
    let temp_file_path = args.output_dir.join(".download.tmp");
    remove_temp(&temp_file_path, &temp_dir_path);
    set_ctrlc_handler(temp_file_path.clone(), temp_dir_path.clone());

    for (i, &version) in versions.iter().enumerate() {
        let url: String = get_url(version);

        let name: String = if args.extract {
            version.to_string() // directory
        } else if args.keep_name {
            url.rsplit_once("/").unwrap().1.to_owned()
        } else {
            format!("{version}.exe")
        };
        let target = args.output_dir.join(name);
        if target.exists() {
            skip_count += 1;
            continue;
        }
        if skip_count > 0 {
            println!("Skipped {skip_count} already downloaded versions");
            skip_count = 0;
        }

        println!("{}/{count} Downloading IDE {version} from {url}", i + 1);

        let response = match client.get(&url).send().and_then(Response::error_for_status) {
            Ok(r) => r,
            Err(e) => {
                print_trace(&format!("Failed to download IDE {version} from {url}"), &e);
                continue;
            }
        };

        let Some(content_length) = response.content_length() else {
            warn!("No content length specified for {version}; probably a 404 page");
            continue;
        };

        let mut file = match std::fs::File::create_new(&temp_file_path) {
            Ok(f) => f,
            Err(e) => {
                print_trace("Failed to create tempfile", &e);
                continue;
            }
        };

        // this takes the majority of time
        let pb = ProgressBar::new(content_length);
        pb.set_style(
            ProgressStyle::with_template("{percent:>3}% [{bar:50}] {bytes}/{total_bytes}")
                .unwrap()
                .progress_chars("=> "),
        );
        pb.set_draw_target(ProgressDrawTarget::stdout_with_hz(4));

        if let Err(e) = std::io::copy(&mut pb.wrap_read(response), &mut file) {
            print_trace("Failed to write response to tempfile", &e);
            continue;
        }
        pb.finish_and_clear();

        let source = if args.extract {
            invoke_7zip(&temp_file_path, &temp_dir_path, None);
            if args.upgrade && version.0 == 1 {
                let upgrade_zip = upgrade_zip_path(version, &temp_dir_path);
                invoke_7zip(&upgrade_zip, &target, Some(GMS1_PASSWORD));
                remove_temp(&temp_file_path, &temp_dir_path);
                continue;
            }
            &temp_dir_path
        } else {
            &temp_file_path
        };
        if let Err(e) = std::fs::rename(source, &target) {
            let msg = format!("Failed to rename temp to `{}`", target.display());
            print_trace(&msg, &e);
        }
        remove_temp(&temp_file_path, &temp_dir_path);
    }

    if skip_count > 0 {
        println!("Skipped {skip_count} already downloaded versions");
    }

    remove_temp(&temp_file_path, &temp_dir_path);
    println!("All done!");
}

fn set_ctrlc_handler(temp_file_path: PathBuf, temp_dir_path: PathBuf) {
    // triggers for SIGINT and for SIGTERM (`ctrlc` crate feature `termination`)
    ctrlc::set_handler(move || {
        remove_temp(&temp_file_path, &temp_dir_path);
        std::process::exit(130); // SIGINT
    })
    .expect("failed to set ctrlc handler")
}

fn remove_temp(file_path: &Path, dir_path: &Path) {
    match std::fs::remove_file(file_path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => {
            let msg = format!("Failed to remove tempfile `{}`", file_path.display());
            print_trace(&msg, &e);
        }
    }
    match std::fs::remove_dir_all(dir_path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => {
            let msg = format!("Failed to remove tempdir `{}`", dir_path.display());
            print_trace(&msg, &e);
        }
    }
}

/// Some versions weren't recorded by archive.org :(
/// If you have access to these versions, let me know!
#[allow(clippy::match_like_matches_macro, reason = "matches! is ugly")]
fn known_missing_version(ver: Version) -> bool {
    match ver {
        Version(1, _, ..690, _) => true,
        Version(1, 2, 1262, _) | Version(1, 3, 1312, _) | Version(2, 0, 3, 56) => true,
        _ => false,
    }
}

/// Some installer downloads were captured before they were actually released.
/// This causes the earliest snapshot to be a 404 page instead of a download.
fn override_timestamp(ver: Version) -> Option<u64> {
    Some(match ver {
        Version(1, 1, 1013, _) => 20140721060018,
        Version(1, 1, 1044, _) => 20160325114921,
        Version(1, 1, 1058, _) => 20160325114921,
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

fn upgrade_zip_path(version: Version, temp_archive_path: &Path) -> PathBuf {
    let zip_name = if version.3 > 1451 {
        format!("Original-{version}.zip")
    } else {
        "upgrade.zip".to_owned()
    };
    temp_archive_path
        .join("$LOCALAPPDATA/GameMaker-Studio/UpgradeZip/")
        .join(zip_name)
}
