mod download;
mod error;
mod extract;
mod ides;
mod password;
mod runtimes;
mod util;
mod version;

use crate::ides::download_ides;
use crate::runtimes::download_runtimes;
use crate::version::Version;
use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

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

/// Downloads GameMaker IDE installers.
#[derive(Debug, Parser)]
struct DownloadIdeArgs {
    /// The directory where IDEs will be collected.
    /// Defaults to "gamemaker-ides".
    #[arg(long, short, default_value = "gamemaker-ides")]
    output_dir: PathBuf,

    /// The IDE versions to download.
    /// If unspecified, downloads all.
    #[arg(long, short)]
    versions: Vec<Version>,

    /// Extracts the archive contained in the installers.
    #[arg(long, short = 'x')]
    extract: bool,

    /// Replace GM:S 1 IDEs with their extracted upgrade zip.
    /// Only has an effect if `--extract` is specified.
    #[arg(long, short)]
    upgrade: bool,

    /// Keeps the original installer filename instead of renaming to `{version}.exe`.
    /// Has no effect if `--extract` is specified.
    #[arg(long, short)]
    keep_name: bool,
}

/// Downloads GameMaker runtimes (GMS2+).
#[derive(Debug, Parser)]
struct DownloadRuntimeArgs {
    /// The directory where runtimes will be collected.
    /// Defaults to "gamemaker-runtimes".
    #[arg(long, short, default_value = "gamemaker-runtimes")]
    output_dir: PathBuf,

    /// The runtime versions to download.
    /// If unspecified, downloads all.
    #[arg(long, short)]
    versions: Vec<Version>,

    /// The platform modules to download.
    /// If unspecified, downloads all.
    #[arg(long, short)]
    platforms: Vec<runtimes::Platform>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let result: error::MyResult<()> = match args.command {
        Command::DownloadIdes(args) => download_ides(args).await,
        Command::DownloadRuntimes(args) => download_runtimes(args).await,
        Command::UpgradeGms1 => todo!(),
    };
    if let Err(error) = result {
        error.exit();
    }
}
