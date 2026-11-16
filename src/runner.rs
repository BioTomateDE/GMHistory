use crate::CollectRunnerArgs;
use crate::error::*;
use crate::extract::invoke_7zip;
use crate::extract::probe_7zip;
use crate::runtime::Platform;
use crate::runtime::fs::DirStructure;
use crate::runtime::fs::WalkTheDawg;
use crate::tempslop::TempDir;
use crate::util::mkdir;
use std::path::Path;

// TODO: support gms1
pub async fn collect_runners(args: CollectRunnerArgs) -> MyResult<()> {
    probe_7zip()?;

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
        dostuff(&entry, &args.output_dir)
            .await
            .ctx(|| format!("Failed to collect runner from {entry:?}"))?;
    }

    Ok(())
}

async fn dostuff(entry: &WalkTheDawg, output_dir: &Path) -> MyResult<()> {
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
            if src.try_exists().unwrap_or(true) {
                copy(src, format!("windows-x86_{}", entry.version)).await?;
            }
            // in between, they had both.
            let src = entry.path.join("windows/x64/Runner.exe");
            if src.try_exists().unwrap_or(true) {
                copy(src, format!("windows-x64_{}", entry.version)).await?;
            }
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
            let src = entry.path.join("xboxone/XboxOneRunner.exe");
            copy(src, output_path).await?;
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
