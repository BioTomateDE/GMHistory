use crate::DownloadRuntimeArgs;
use crate::download::Downloader;
use crate::error::*;
use reqwest::{Client, Response};
use serde::Deserialize;
use std::time::Duration;
use strum::{EnumString, IntoStaticStr, VariantArray};

pub async fn download_runtimes(mut args: DownloadRuntimeArgs) -> MyResult<()> {
    let platforms = if args.platforms.is_empty() {
        Platform::VARIANTS
    } else {
        args.platforms.dedup();
        &args.platforms
    };

    println!("Fetching manifest Zeus-Runtime.rss");
    let downloader = Downloader::new(&args.output_dir)?;
    let response: Response = downloader
        .send_request("https://gms.yoyogames.com/Zeus-Runtime.rss")
        .await
        .ctx("Failed to fetch Zeus-Runtime.rss manifest")?;
    let text: String = response.text().await.ctx("Failed to get response text")?;
    let rss: Rss = quick_xml::de::from_str(&text).ctx("Failed to parse RSS manifest")?;
    debug(&rss);
    // TODO

    Ok(())
}

fn parse_rss(text: &str) -> MyResult<Rss> {
    let xml = text.trim_start_matches('\u{feff}'); // strip the BOM
    quick_xml::de::from_str(xml).ctx("Failed to parse XML")
}

fn debug(rss: &Rss) {
    for item in &rss.channel.item {
        println!("{} ({})", item.title, item.pub_date);
        for m in &item.enclosure.modules {
            println!("  {:<12} {}", m.name, m.url);
        }
    }
}

#[derive(Debug, Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Debug, Deserialize)]
struct Channel {
    title: String,
    #[serde(default)]
    item: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    title: String,
    #[serde(rename = "pubDate")]
    pub_date: String,
    #[serde(rename = "sparkle:releaseNotesLink")]
    release_notes_link: String,
    comments: String,
    enclosure: Enclosure,
}

#[derive(Debug, Deserialize)]
struct Enclosure {
    #[serde(rename = "@url")]
    url: String,

    #[serde(rename = "@sparkle:version")]
    version: String,

    #[serde(rename = "@length")]
    length: u64,

    #[serde(rename = "@sparkle:dsaSignature")]
    dsa_signature: String,

    #[serde(rename = "module", default)]
    modules: Vec<Module>,
}

#[derive(Debug, Deserialize)]
struct Module {
    #[serde(rename = "@name")]
    name: String,

    #[serde(rename = "@url")]
    url: String,

    #[serde(rename = "@sparkle:version")]
    version: String,

    #[serde(rename = "@length")]
    length: u64,

    #[serde(rename = "@sparkle:dsaSignature")]
    dsa_signature: String,
}

/// Corresponds to the "name" field in the XML tag of the RSS manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, IntoStaticStr, VariantArray)]
pub enum Platform {
    #[strum(serialize = "amazonfire")]
    AmazonFire,

    #[strum(serialize = "android")]
    Android,

    #[strum(serialize = "base-module-linux-arm")]
    BaseModuleLinuxArm,

    #[strum(serialize = "base-module-linux-arm64")]
    BaseModuleLinuxArm64,

    #[strum(serialize = "base-module-linux-x64")]
    BaseModuleLinuxX64,

    #[strum(serialize = "base-module-osx-arm64")]
    BaseModuleOsxArm64,

    #[strum(serialize = "base-module-osx-x64")]
    BaseModuleOsxX64,

    #[strum(serialize = "base-module-windows-x64")]
    BaseModuleWindowsX64,

    #[strum(serialize = "html5")]
    Html5,

    #[strum(serialize = "ios")]
    Ios,

    #[strum(serialize = "linux")]
    Linux,

    #[strum(serialize = "linuxYYC")]
    LinuxYYC,

    #[strum(serialize = "mac")]
    Mac,

    #[strum(serialize = "mac.mono")]
    MacMono,

    #[strum(serialize = "macYYC")]
    MacYYC,

    #[strum(serialize = "operagx")]
    OperaGx,

    #[strum(serialize = "operagxYYC")]
    OperaGxYYC,

    #[strum(serialize = "ps4")]
    Ps4,

    #[strum(serialize = "ps5")]
    Ps5,

    #[strum(serialize = "switch")]
    Switch,

    #[strum(serialize = "switch2")]
    Switch2,

    #[strum(serialize = "tvos")]
    TvOs,

    #[strum(serialize = "windows")]
    Windows,

    #[strum(serialize = "windowsuap")]
    WindowsUap,

    #[strum(serialize = "windowsYYC")]
    WindowsYYC,

    #[strum(serialize = "xboxone")]
    XboxOne,

    #[strum(serialize = "xboxseriesxs")]
    XboxSeriesXs,
}
