mod asyncslop;
mod error;
mod extract;
mod ide;
mod logger;
mod net;
mod progressbar;
mod runner;
mod runtime;
mod tempslop;
mod util;
mod version;

use crate::error::MyResult;
use crate::ide::download_ides;
use crate::logger::init_logger;
use crate::runner::collect_runners;
use crate::runtime::download_runtimes;
use crate::runtime::fs::DirStructure;
use crate::tempslop::set_ctrlc_handler;
use crate::version::Version;
use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

/// Tool to download and extract GameMaker IDE installers and runtimes.
#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    DownloadIdes(DownloadIdeArgs),
    DownloadRuntimes(DownloadRuntimeArgs),
    CollectRunners(CollectRunnerArgs),
}

/// Download GameMaker IDE installers and potentially extract them.
#[derive(Debug, Clone, Parser)]
struct DownloadIdeArgs {
    #[arg(value_enum, default_value_t = IdeEvolution::Upgrade)]
    evolution: IdeEvolution,

    /// The directory where IDEs will be collected.
    #[arg(long, short, default_value = "gamemaker-ides")]
    output_dir: PathBuf,

    /// The IDE versions to download.
    /// If unspecified, download all.
    #[arg(long, short)]
    versions: Vec<Version>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, clap::ValueEnum)]
enum IdeEvolution {
    /// Only download installers, keep original filenames.
    ///
    /// Filenames can be `GMStudio-Installer-{version}.exe`, `GameMakerStudio-Installer-{version}.exe` or `GameMaker-Installer-{version}.exe`.
    OriginalInstallers,

    /// Only download installers and normalize filesnames to `{version}.exe`.
    RenamedInstallers,

    /// Extract installer archive (only free version; kinda useless).
    Extract,

    /// Extract password-protected UpgradeZip in the installer archive
    Upgrade,
}

/// Download GameMaker runtimes (GMS2+).
#[derive(Debug, Clone, Parser)]
struct DownloadRuntimeArgs {
    /// The directory where runtimes will be collected.
    #[arg(long, short, default_value = "gamemaker-runtimes")]
    output_dir: PathBuf,

    /// The runtime versions to download.
    /// If unspecified, download all.
    #[arg(long, short)]
    versions: Vec<Version>,

    /// The platform modules to download.
    /// If unspecified, download all.
    #[arg(long, short)]
    platforms: Vec<runtime::Platform>,

    /// List all platforms and exit.
    #[arg(long)]
    list_platforms: bool,

    #[command(flatten)]
    file_structure: DirStructure,
}

/// Find runners from IDE and runtime dirs and collect them
#[derive(Debug, Clone, Parser)]
struct CollectRunnerArgs {
    /// The directory you downloaded the runtimes to.
    #[arg(long, short, default_value = "gamemaker-runtimes")]
    runtimes_dir: PathBuf,

    /// The directory where runners will be collected.
    #[arg(long, short, default_value = "gamemaker-runners")]
    output_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    init_logger();
    let args = Args::parse();
    set_ctrlc_handler();
    let result: MyResult<()> = match args.command {
        Command::DownloadIdes(args) => download_ides(args).await,
        Command::DownloadRuntimes(args) => download_runtimes(args).await,
        Command::CollectRunners(args) => collect_runners(args).await,
    };
    crate::progressbar::BAR_TOTAL.finish_and_clear();
    if let Err(error) = result {
        log::error!("{error}");
        std::process::exit(1);
    }
}
