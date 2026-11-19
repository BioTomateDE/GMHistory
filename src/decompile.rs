use crate::DecompileArgs;
use crate::asyncslop::join_tasks;
use crate::error::*;
use crate::progressbar::BAR_TOTAL;
use crate::progressbar::init_total_bar;
use crate::tempslop::TempDir;
use crate::tempslop::TempFile;
use crate::util::FsEntry;
use crate::util::copy_file;
use crate::util::file_kind;
use crate::util::listdir;
use crate::util::mkdir;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::AbortHandle;
use tokio::task::JoinSet;

const JAVA_CODE: &str = include_str!("Decompile.java");

const GIB: u64 = 1024 * 1024 * 1024;

/// Average memory usage of one Ghidra unit (~2GiB).
const GHIDRA_RAM_USAGE: u64 = 2 * GIB;

/// How much system memory to keep free (1 GiB).
const RAM_BUFFER: u64 = GIB;

#[derive(Debug)]
struct GlobalContext {
    output_dir: PathBuf,
    script_dir: PathBuf,
    ghidra_path: PathBuf,
    limit: Semaphore,
}

pub async fn decompile_runners(args: DecompileArgs) -> MyResult<()> {
    let ghidra_path: PathBuf = probe_ghidra(&args.ghidra_dir)?;

    let temp_script_dir = TempDir::new_for_ghidra()?;
    let temp_script_path = temp_script_dir.path().join("Decompile.java");
    std::fs::write(temp_script_path, JAVA_CODE).ctx("Failed to write java code to tempfile")?;

    let runners_dir: Vec<FsEntry> = listdir(&args.runners_dir)?;
    mkdir(&args.output_dir)?;

    let cpu_count: usize = match std::thread::available_parallelism() {
        Ok(n) if n.get() < 2 => 1,
        Ok(n) => n.get() - 1, // keep one core free
        Err(e) => {
            warn!(e, "Failed to get number of CPU cores; falling back to one");
            1
        }
    };
    let available_ram: u64 = ::system_memory::available();
    let available_ram = available_ram
        .checked_sub(RAM_BUFFER)
        .ctx("You don't have enough system memory!!!")?;
    let ram_limit = (available_ram / GHIDRA_RAM_USAGE) as usize;
    if ram_limit < 1 {
        bail!("I know times are tough with all the AI slop but you need more RAM for ts");
    }
    let limit = ram_limit.min(cpu_count);

    log::info!("Decompiling {} runners ({limit} concurrently)", runners_dir.len());
    log::info!("Don't panic if it looks stuck; it just takes a while");
    let global = Arc::new(GlobalContext {
        output_dir: args.output_dir,
        script_dir: temp_script_dir.path().into(),
        ghidra_path,
        limit: Semaphore::new(limit),
    });

    let mut tasks: JoinSet<Result<(), Error>> = JoinSet::new();

    for entry in runners_dir {
        if !entry.is_file() {
            continue;
        }
        let name = entry.filename();
        let output_path = global.output_dir.join(name.to_owned() + ".c");
        if output_path.exists() {
            continue;
        }
        let global = Arc::clone(&global);
        let _: AbortHandle = tasks.spawn(async move { decompile_runner(entry, global).await });
    }

    init_total_bar(tasks.len(), tasks.len() as u64);
    join_tasks(tasks).await
}

async fn decompile_runner(runner: FsEntry, global: Arc<GlobalContext>) -> MyResult<()> {
    let _cpu_permit = global.limit.acquire().await?;

    let name = runner.filename();
    let output_path = global.output_dir.join(name.to_owned() + ".c");
    log::debug!("Decompiling {name}");
    decompile_runner_impl(&runner.path, &output_path, &global)
        .await
        .ctx(|| format!("Failed to decompile runner {name:?}"))
}

async fn decompile_runner_impl(runner_path: &Path, output_path: &Path, global: &GlobalContext) -> MyResult<()> {
    let tempdir = TempDir::new_for_ghidra()?;
    let tempfile = TempFile::new_for_ghidra()?;
    let output = tokio::process::Command::new(&global.ghidra_path)
        .arg(tempdir.path())
        .arg("TempProject")
        .arg("-import")
        .arg(runner_path)
        .arg("-scriptPath")
        .arg(&global.script_dir)
        .arg("-postScript")
        .arg("Decompile.java")
        .arg(tempfile.path())
        .arg("-deleteProject")
        .output()
        .await
        .ctx("Failed to invoke Ghidra")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "Ghidra failed with {}; stdout: <<{stdout}\n>> stderr: <<\n{stderr}>>",
            output.status
        );
    }

    format_c_code(tempfile.path()).await?;
    // rename may not work here due to potentially crossing filesystems/devices
    copy_file(tempfile.path(), output_path).await?;
    BAR_TOTAL.inc(1);
    Ok(())
}

fn probe_ghidra(ghidra_dir: &Path) -> MyResult<PathBuf> {
    if !ghidra_dir.try_exists().unwrap_or(true) {
        bail!(
            "Ghidra installation directory does not exist: `{}`",
            ghidra_dir.display()
        );
    }
    let ghidra = ghidra_dir.join("support/analyzeHeadless");
    let meta = std::fs::metadata(&ghidra).ctx(|| format!("Missing ghidra tool at `{}`", ghidra.display()))?;
    if !meta.is_file() {
        bail!("Ghidra tool is actually a {}", file_kind(meta.file_type()));
    }
    Ok(ghidra)
}

const CLANG_FORMAT_STYLE: &str = "{
BasedOnStyle: LLVM,
IndentWidth: 4,
ContinuationIndentWidth: 4,
ColumnLimit: 100,
BreakBeforeBraces: Attach,
ReflowComments: false,
AlignAfterOpenBracket: BlockIndent,
AlignOperands: DontAlign,
BinPackArguments: false,
BinPackParameters: false,
}";

async fn format_c_code(code_file_path: &Path) -> MyResult<()> {
    let output = tokio::process::Command::new("clang-format")
        .arg("--style")
        .arg(CLANG_FORMAT_STYLE)
        .arg("-i") // format file in place
        .arg("--")
        .arg(code_file_path)
        .output()
        .await
        .ctx("Failed to invoke clang-format")?;
    let status = output.status;
    if !status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to format C code; clang-format failed with {status}: <<\n{stderr}>>")
    }
    Ok(())
}
