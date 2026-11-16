use crate::CollectRunnerArgs;
use crate::IdeEvolution;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::extract::probe_7zip;
use crate::ide::evolve;
use crate::ide::upgrade_gms1;
use crate::runtime::Platform;
use crate::runtime::fs::DirStructure;
use crate::runtime::fs::WalkTheDawg;
use crate::tempslop::TempDir;
use crate::util::mkdir;
use crate::version::Version;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;

struct IdeTarget {
    path: PathBuf,
    version: Version,
    evolution: IdeEvolution,
}

#[expect(clippy::useless_let_if_seq)]
pub async fn collect_runners(args: CollectRunnerArgs) -> MyResult<()> {
    probe_7zip()?;

    let mut nothing_ever_happens = true;
    if args.runtimes_dir.try_exists().unwrap_or(true) {
        collect_runtime_runners(&args).await?;
        nothing_ever_happens = false;
    } else {
        log::warn!("Runtimes directory does not exist: `{}`", args.runtimes_dir.display());
    }

    if args.ides_dir.try_exists().unwrap_or(true) {
        collect_ide_runners(&args).await?;
        nothing_ever_happens = false;
    } else {
        log::warn!("IDEs directory does not exist: `{}`", args.ides_dir.display());
    }

    if nothing_ever_happens {
        bail!("Neither the runtimes directory nor the IDEs directory exists");
    }
    log::info!("All done!");
    Ok(())
}

async fn collect_ide_runners(args: &CollectRunnerArgs) -> MyResult<()> {
    let ctx = || format!("Failed to read IDEs directory `{}`", args.ides_dir.display());

    let mut targets: Vec<IdeTarget> = Vec::new();

    for entry in std::fs::read_dir(&args.ides_dir).ctx(ctx)? {
        let entry = entry.ctx(ctx)?;
        let path = entry.path();
        let filename = path.file_name().unwrap();
        let filename = filename
            .to_str()
            .ctx(|| format!("Non UTF-8 filename `{}`", filename.display()))?;
        if filename.starts_with('.') {
            continue;
        }

        let stuff = ide_version_from_filename(&path, filename, entry.metadata())
            .ctx(|| format!("Failed to detect version from IDE path `{}`", path.display()))?;
        targets.push(stuff);
    }

    log::info!("Extracting runners from {} IDEs", targets.len());
    // TODO: parallel slop

    for target in targets {
        let IdeTarget {
            path,
            version,
            evolution,
        } = target;
        if evolution < IdeEvolution::Extract {
            let upgrade = evolution >= IdeEvolution::Upgrade;
            let tempdir = evolve(version, &path, &args.output_dir, upgrade).await?;
            copy_runner(tempdir.path(), &args.output_dir, version).await?;
        } else if evolution == IdeEvolution::Extract && version.0 == 1 {
            let tempdir = upgrade_gms1(version, &path, &args.output_dir).await?;
            copy_runner(tempdir.path(), &args.output_dir, version).await?;
        } else {
            copy_runner(&path, &args.output_dir, version).await?;
        }
    }

    Ok(())
}

async fn copy_runner(ide_dir: &Path, output_dir: &Path, version: Version) -> MyResult<()> {
    // 3 versions also have `Windows8/Native/executable/WinMetroRunner.exe` but it's the same as the x86 one
    const PATHS: &[(&str, &str)] = &[
        ("Windows8/Native/x86/WinMetroRunner.exe", "windows8-x86"),
        ("Windows8/Native/arm/WinMetroRunner.exe", "windows8-arm"),
        ("Tizen/Device/TizenRunner.exe", "tizen-arm"),
        ("Tizen/Emulator/TizenRunner.exe", "tizen-x86"),
    ];
    copy(
        ide_dir.join("Runner.exe"),
        output_dir.join(format!("windows-x64_{version}")),
    )
    .await?;
    for (path, plat_arch) in PATHS {
        copy_if_exists(ide_dir.join(path), output_dir.join(format!("{plat_arch}_{version}"))).await?;
    }
    Ok(())
}

fn ide_version_from_filename(path: &Path, filename: &str, metadata: std::io::Result<Metadata>) -> MyResult<IdeTarget> {
    /// Original installer filenames start with one of these prefixes, depending on the GameMaker version.
    const PREFIXES: &[&str] = &[
        "GMStudio-Installer-",
        "GameMakerStudio-Installer-",
        "GameMaker-Installer-",
    ];

    let metadata: Metadata = metadata.ctx(|| format!("Failed to fetch metadata for `{}`", path.display()))?;
    if metadata.is_file() {
        let Some(stem) = filename.strip_suffix(".exe") else {
            bail!("Filename does not end with `.exe`: {filename:?}");
        };
        let (stem, evolution) = match strip_any_prefix(stem, PREFIXES) {
            Some(stripped) => (stripped, IdeEvolution::OriginalInstallers),
            None => (stem, IdeEvolution::RenamedInstallers),
        };
        let version = Version::try_from(stem).ctx(|| format!("Failed to parse version from filename {filename:?}"))?;
        Ok(IdeTarget {
            path: path.to_path_buf(),
            version,
            evolution,
        })
    } else if metadata.is_dir() {
        let version =
            Version::try_from(filename).ctx(|| format!("Failed to parse version from filename {filename:?}"))?;
        let upgradezip_path = path.join("$LOCALAPPDATA/GameMaker-Studio/UpgradeZip");
        let has_upgradezip: bool = upgradezip_path
            .try_exists()
            .ctx(|| format!("Failed to stat `{}`", upgradezip_path.display()))?;
        let evolution = if has_upgradezip {
            IdeEvolution::Extract
        } else {
            IdeEvolution::Upgrade
        };
        Ok(IdeTarget {
            path: path.to_path_buf(),
            version,
            evolution,
        })
    } else {
        bail!(
            "Directory entry is neither a file nor a directory: `{}`",
            path.display()
        );
    }
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
    let entries: Vec<WalkTheDawg> = struc
        .walk(&args.runtimes_dir)
        .ctx("Failed to walk runtimes directory")?;

    let total_count = entries.len();
    log::info!("Collecting runners from {total_count} runtimes");

    mkdir(&args.output_dir).ctx("Failed to create output directory")?;

    // TODO: parallel
    for entry in entries {
        collect_runtime_runner(&entry, &args.output_dir)
            .await
            .ctx(|| format!("Failed to collect runner from {entry:?}"))?;
    }
    Ok(())
}

async fn collect_runtime_runner(entry: &WalkTheDawg, output_dir: &Path) -> MyResult<()> {
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
            let tempdir = TempDir::new(output_dir)?;
            invoke_7zip(&zip_path, tempdir.path(), None)
                .await
                .ctx("Failed to extract Linux runner.zip")?;
            copy(tempdir.path().join("runner"), output_path).await?;
        }
        Platform::Ps4 => {
            copy(entry.path.join("ps4/PS4Runner.elf"), output_path).await?;
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
                copy(src, dest).await?;
            }
        }
        Platform::XboxOne => {
            copy(entry.path.join("xboxone/XboxOneRunner.exe"), output_path).await?;
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

async fn copy(source_path: impl AsRef<Path>, dest_path: impl AsRef<Path>) -> MyResult<()> {
    let source_path = source_path.as_ref();
    let dest_path = dest_path.as_ref();
    if dest_path.exists() {
        return Ok(());
    }
    let _: u64 = tokio::fs::copy(source_path, dest_path).await.ctx(|| {
        format!(
            "Failed to copy runner file `{}` to `{}`",
            source_path.display(),
            dest_path.display()
        )
    })?;
    Ok(())
}

async fn copy_if_exists(source_path: impl AsRef<Path>, dest_path: impl AsRef<Path>) -> MyResult<()> {
    let source_path = source_path.as_ref();
    if source_path.try_exists().unwrap_or(true) {
        copy(source_path, dest_path).await?;
    }
    Ok(())
}
