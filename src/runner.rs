use crate::CollectRunnerArgs;
use crate::IdeEvolution;
use crate::asyncslop::join_tasks;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::extract::probe_7zip;
use crate::ide::upgrade_gms1;
use crate::runtime::Platform;
use crate::runtime::fs::DirStructure;
use crate::runtime::fs::RuntimeDir;
use crate::tempslop::TempDir;
use crate::util::FsEntry;
use crate::util::copy_file;
use crate::util::listdir;
use crate::util::mkdir;
use crate::version::Version;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::AbortHandle;
use tokio::task::JoinSet;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdeTarget {
    path: PathBuf,
    version: Version,
    evolution: IdeEvolution,
}

#[expect(clippy::useless_let_if_seq)]
pub async fn collect_runners(args: CollectRunnerArgs) -> MyResult<()> {
    probe_7zip()?;
    let mut nothing_ever_happens = true;

    if args.ides_dir.try_exists().unwrap_or(true) {
        collect_ide_runners(&args).await?;
        nothing_ever_happens = false;
    } else {
        log::warn!("IDEs directory does not exist: `{}`", args.ides_dir.display());
    }

    if args.runtimes_dir.try_exists().unwrap_or(true) {
        collect_runtime_runners(&args).await?;
        nothing_ever_happens = false;
    } else {
        log::warn!("Runtimes directory does not exist: `{}`", args.runtimes_dir.display());
    }

    if nothing_ever_happens {
        bail!("Neither the runtimes directory nor the IDEs directory exists");
    }
    Ok(())
}

async fn collect_ide_runners(args: &CollectRunnerArgs) -> MyResult<()> {
    let mut targets: Vec<IdeTarget> = Vec::new();

    for entry in listdir(&args.ides_dir)? {
        let target: IdeTarget =
            ide_target_from_filename(&entry).ctx(|| format!("Failed to detect version from IDE {entry}"))?;
        if target.version.0 == 1 {
            targets.push(target);
        }
    }

    log::info!("Extracting runners from {} IDEs", targets.len());

    for target in targets {
        let ver = target.version;
        collect_ide_runner(target, args)
            .await
            .ctx(|| format!("Failed to collect runner from IDE {ver}"))?;
    }

    Ok(())
}

async fn collect_ide_runner(target: IdeTarget, args: &CollectRunnerArgs) -> MyResult<()> {
    let IdeTarget {
        path,
        version,
        evolution,
    } = target;
    if evolution < IdeEvolution::Extract {
        let archive_dir = TempDir::new_in(&args.output_dir)?;
        invoke_7zip(&path, archive_dir.path(), None).await?;
        let tempdir = if evolution >= IdeEvolution::Upgrade && version.0 == 1 {
            upgrade_gms1(version, archive_dir.path(), &args.output_dir).await?
        } else {
            archive_dir
        };
        copy_runner(tempdir.path(), &args.output_dir, version).await?;
    } else if evolution == IdeEvolution::Extract && version.0 == 1 {
        let tempdir = upgrade_gms1(version, &path, &args.output_dir).await?;
        copy_runner(tempdir.path(), &args.output_dir, version).await?;
    } else {
        copy_runner(&path, &args.output_dir, version).await?;
    }
    Ok(())
}

async fn copy_runner(ide_dir: &Path, output_dir: &Path, version: Version) -> MyResult<()> {
    // 3 versions also have `Windows8/Native/executable/WinMetroRunner.exe` but it's the same as the x86 one
    const PATHS: &[(&str, &str)] = &[
        // ("Windows8/Native/x86/WinMetroRunner.exe", "windows8-x86"),
        // ("Windows8/Native/arm/WinMetroRunner.exe", "windows8-arm"),
        ("Tizen/Device/TizenRunner.exe", "tizen-arm"),
        ("Tizen/Emulator/TizenRunner.exe", "tizen-x86"),
    ];
    copy_file(
        ide_dir.join("Runner.exe"),
        output_dir.join(format!("windows-x64_{version}")),
    )
    .await?;
    for (path, plat_arch) in PATHS {
        copy_if_exists(ide_dir.join(path), output_dir.join(format!("{plat_arch}_{version}"))).await?;
    }
    Ok(())
}

fn ide_target_from_filename(entry: &FsEntry) -> MyResult<IdeTarget> {
    /// Original installer filenames start with one of these prefixes, depending on the GameMaker version.
    const PREFIXES: &[&str] = &[
        "GMStudio-Installer-",
        "GameMakerStudio-Installer-",
        "GameMaker-Installer-",
    ];

    let filename: &str = entry.filename();
    if entry.is_file() {
        let Some(stem) = filename.strip_suffix(".exe") else {
            bail!("Filename does not end with `.exe`: {filename:?}");
        };
        let (stem, evolution) = match strip_any_prefix(stem, PREFIXES) {
            Some(stripped) => (stripped, IdeEvolution::OriginalInstallers),
            None => (stem, IdeEvolution::RenamedInstallers),
        };
        let version = Version::try_from(stem).ctx(|| format!("Failed to parse version from filename {filename:?}"))?;
        Ok(IdeTarget {
            path: entry.path.clone(),
            version,
            evolution,
        })
    } else if entry.is_dir() {
        let version =
            Version::try_from(filename).ctx(|| format!("Failed to parse version from filename {filename:?}"))?;
        let upgradezip_path = entry.path.join("$LOCALAPPDATA/GameMaker-Studio/UpgradeZip");
        let has_upgradezip: bool = upgradezip_path
            .try_exists()
            .ctx(|| format!("Failed to stat `{}`", upgradezip_path.display()))?;
        let evolution = if has_upgradezip {
            IdeEvolution::Extract
        } else {
            IdeEvolution::Upgrade
        };
        Ok(IdeTarget {
            path: entry.path.clone(),
            version,
            evolution,
        })
    } else {
        bail!("Directory entry is neither a file nor a directory: {entry}");
    }
}

pub fn detect_ide_evolution(output_dir: &Path) -> MyResult<Option<IdeEvolution>> {
    let entries = listdir(output_dir)?;
    let Some(entry) = entries.first() else { return Ok(None) };
    let stuff = ide_target_from_filename(entry).ctx(|| format!("Failed to detect version from IDE {entry}"))?;
    Ok(Some(stuff.evolution))
}

fn strip_any_prefix<'a>(string: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if let Some(rest) = string.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

async fn collect_runtime_runners(args: &CollectRunnerArgs) -> MyResult<()> {
    let struc: DirStructure = get_runtime_file_structure(&args.runtimes_dir)?;
    log::debug!("Detected {struc}");
    let entries: Vec<RuntimeDir> = struc
        .walk(&args.runtimes_dir)
        .ctx("Failed to walk runtimes directory")?;

    let total_count = entries.len();
    log::info!("Collecting runners from {total_count} runtimes");

    mkdir(&args.output_dir).ctx("Failed to create output directory")?;

    let mut tasks = JoinSet::new();
    let output_dir = Arc::new(args.output_dir.clone());
    for entry in entries {
        let output_dir = Arc::clone(&output_dir);
        let _: AbortHandle = tasks.spawn(async move { collect_runtime_runner(entry, output_dir).await });
    }
    join_tasks(tasks).await
}

async fn collect_runtime_runner(entry: RuntimeDir, output_dir: Arc<PathBuf>) -> MyResult<()> {
    // TODO: preserve original structure maybe??
    let filename = format!("{}_{}", entry.platform, entry.version);
    // TODO: should this append the `.exe` to the output filename for winslop?
    let output_path = output_dir.join(filename);

    match entry.platform {
        Platform::Linux => {
            if output_dir.exists() {
                return Ok(());
            }
            let zip_path = entry.path.join("linux/runner.zip");
            let tempdir = TempDir::new_in(&output_dir)?;
            invoke_7zip(&zip_path, tempdir.path(), None)
                .await
                .ctx("Failed to extract Linux runner.zip")?;
            copy_file(tempdir.path().join("runner"), output_path).await?;
        }
        Platform::Ps4 => {
            copy_file(entry.path.join("ps4/PS4Runner.elf"), output_path).await?;
        }
        Platform::Windows => {
            // old versions only have the i686 runner.
            let src = entry.path.join("windows/Runner.exe");
            copy_if_exists(src, output_dir.join(format!("windows-x86_{}", entry.version))).await?;
            // in between, they had both.
            let src = entry.path.join("windows/x64/Runner.exe");
            copy_if_exists(src, output_dir.join(format!("windows-x64_{}", entry.version))).await?;
            // new versions only have x86-64.
        }
        Platform::WindowsUap => {
            // don't think theres a better way to do this (output filenames wise)
            for arch in ["x86", "x64", "arm"] {
                let src = entry.path.join(format!("windowsuap/{arch}/WinUAPRunner.exe"));
                let dest = output_dir.join(format!("windowsuap-{arch}_{}", entry.version));
                copy_file(src, dest).await?;
            }
        }
        Platform::XboxOne => {
            copy_file(entry.path.join("xboxone/XboxOneRunner.exe"), output_path).await?;
        }
        _ => {}
    }
    Ok(())
}

fn get_runtime_file_structure(path: &Path) -> MyResult<DirStructure> {
    let ctx = || {
        format!(
            "Failed to detect filesystem structure in runtimes directory `{}`",
            path.display(),
        )
    };
    crate::runtime::detect_existing_structure(path).ctx(ctx)?.ctx(ctx)
}

async fn copy_if_exists(source_path: impl AsRef<Path>, dest_path: impl AsRef<Path>) -> MyResult<()> {
    let source_path = source_path.as_ref();
    if source_path.try_exists().unwrap_or(true) {
        copy_file(source_path, dest_path).await?;
    }
    Ok(())
}
