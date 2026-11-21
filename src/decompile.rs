use crate::DecompileRunnerArgs;
use crate::asyncslop::join_tasks;
use crate::error::*;
use crate::progressbar::BAR_TOTAL;
use crate::progressbar::inc_and_update_message;
use crate::progressbar::init_total_bar;
use crate::tempslop::TempDir;
use crate::tempslop::TempFile;
use crate::util::FsEntry;
use crate::util::file_kind;
use crate::util::listdir;
use crate::util::mkdir;
use crate::util::run_command;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::AbortHandle;
use tokio::task::JoinSet;

const JAVA_CODE: &str = include_str!("Decompile.java");

const MIB: u64 = 1024 * 1024;

/// Average memory usage of one Ghidra unit.
const GHIDRA_RAM_USAGE: u64 = 3072 * MIB;

/// How much system memory to keep free (1 GiB).
const RAM_BUFFER: u64 = 1024 * MIB;

#[derive(Debug)]
struct GlobalContext {
    output_dir: PathBuf,
    script_dir: PathBuf,
    ghidra_path: PathBuf,
    limit: Semaphore,
}

pub async fn decompile_runners(args: DecompileRunnerArgs) -> MyResult<()> {
    let ghidra_path: PathBuf = probe_ghidra(&args.ghidra_dir)?;
    if !args.no_format {
        probe_clang_format().await?;
    }

    let temp_script_dir = TempDir::new_for_ghidra()?;
    let temp_script_path = temp_script_dir.path().join("Decompile.java");
    std::fs::write(temp_script_path, JAVA_CODE).ctx("Failed to write java code to tempfile")?;

    let runners_dir: Vec<FsEntry> = listdir(&args.runners_dir)?;
    mkdir(&args.output_dir)?;

    let limit: usize = if let Some(n) = args.jobs {
        if let Ok(ideal) = ideal_job_count()
            && ideal + 4 < n.get() as usize
        {
            warn!("Optimal number of Ghidra jobs {ideal} is significantly lower than provided job count {n}");
        }
        usize::from(n.get())
    } else {
        ideal_job_count().ctx("Failed to calculate optimal number of Ghidra jobs")?
    };

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
    let _permit = global.limit.acquire().await?;
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

    let mut command = tokio::process::Command::new(&global.ghidra_path);
    _ = command
        .env("MAXMEM", "2G")
        .arg(tempdir.path())
        .arg("TempProject")
        .arg("-max-cpu")
        .arg("1")
        .arg("-import")
        .arg(runner_path)
        .arg("-scriptPath")
        .arg(&global.script_dir)
        .arg("-postScript")
        .arg("Decompile.java")
        .arg(tempfile.path())
        .arg("-deleteProject");
    let stdout: String = run_command(command, "Ghidra").await?;
    if !tempfile.exists()? {
        let dashes = "-----------------------------------------------------";
        let stdout: &str = if let Some((_, stdout)) = stdout.rsplit_once(dashes) {
            stdout
        } else {
            &stdout
        };
        bail!("Ghidra failed:\n{}", stdout.trim());
    }

    // Make another tempfile in the output directory (instead of formatting in-place)
    // so I can use an (atomic) rename.
    let out_tempfile = TempFile::new(output_path.parent().unwrap())?;
    format_c_code(tempfile.path(), out_tempfile.path())
        .await
        .ctx("Failed to format the decompiled C code")?;
    drop(tempfile);
    out_tempfile.rename(output_path).await?;
    BAR_TOTAL.inc(1); // TODO: also update message. should this even use a progress bar?
    inc_and_update_message();
    Ok(())
}

fn ideal_job_count() -> MyResult<usize> {
    let cpu_count: usize = match std::thread::available_parallelism() {
        Ok(n) => n.get(),
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
    Ok(ram_limit.min(cpu_count))
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

async fn probe_clang_format() -> MyResult<()> {
    let mut command = tokio::process::Command::new("clang-format");
    _ = command.arg("--version");
    let _: String = run_command(command, "clang-format")
        .await
        .ctx("Make sure to install clang-format")?;
    Ok(())
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

async fn format_c_code(input_path: &Path, output_path: &Path) -> MyResult<()> {
    let mut command = tokio::process::Command::new("clang-format");
    _ = command
        .arg("--style")
        .arg(CLANG_FORMAT_STYLE)
        .arg("--") //
        .arg(input_path);
    let formatted_code: String = run_command(command, "clang-format").await?;
    tokio::fs::write(output_path, formatted_code.trim())
        .await
        .ctx(|| format!("Failed to write formatted C code to `{}`", output_path.display()))
}
