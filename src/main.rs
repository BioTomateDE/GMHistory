mod download;
mod error;
mod extract;
mod ide;
mod password;
mod runtime;
mod tempslop;
mod util;
mod version;

use crate::ide::download_ides;
use crate::runtime::download_runtimes;
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
    UpgradeGms1,
}

/// Download GameMaker IDE installers and potentially extract them.
#[derive(Debug, Parser)]
struct DownloadIdeArgs {
    /// The directory where IDEs will be collected.
    #[arg(long, short, default_value = "gamemaker-ides")]
    output_dir: PathBuf,

    /// The IDE versions to download.
    /// If unspecified, download all.
    #[arg(long, short)]
    versions: Vec<Version>,

    /// Extract the archive contained in the installers.
    #[arg(long, short = 'x')]
    extract: bool,

    /// Replace GM:S 1 IDEs with their extracted upgrade zip.
    /// Only has an effect if `--extract` is specified.
    #[arg(long, short)]
    upgrade: bool,

    /// Keep the original installer filename instead of renaming to `{version}.exe`.
    /// Has no effect if `--extract` is specified.
    #[arg(long, short)]
    keep_name: bool,
}

/// Downloads GameMaker runtimes (GMS2+).
#[derive(Debug, Parser)]
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
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tempslop::set_ctrlc_handler();
    let result: error::MyResult<()> = match args.command {
        Command::DownloadIdes(args) => download_ides(args).await,
        Command::DownloadRuntimes(args) => download_runtimes(args).await,
        Command::UpgradeGms1 => todo!(),
    };
    tempslop::cleanup();
    if let Err(error) = result {
        error.exit();
    }
}
