mod platform;
pub mod rss;

pub use self::platform::Platform;
use self::rss::Rss;
use crate::download::Downloader;
use crate::error::*;
use crate::version::Version;
use crate::{DownloadRuntimeArgs, password::generate_password};
use reqwest::Response;
use strum::VariantArray;

pub async fn download_runtimes(mut args: DownloadRuntimeArgs) -> MyResult<()> {
    let platforms = if args.platforms.is_empty() {
        self::platform::Platform::VARIANTS
    } else {
        args.platforms.dedup();
        &args.platforms
    };

    println!("Fetching manifest Zeus-Runtime.rss");
    let downloader = Downloader::new()?;
    let response: Response = downloader
        .send_request("https://gms.yoyogames.com/Zeus-Runtime.rss")
        .await
        .ctx("Failed to fetch Zeus-Runtime.rss manifest")?;
    let text: String = response.text().await.ctx("Failed to get response text")?;
    let rss: Rss = text.parse()?;
    println!("Got {} items (versions) in manifest", rss.channel.items.len());

    for item in &rss.channel.items {
        let v = item.enclosure.version;
        println!("{v}  {}", Version::ALL.contains(&v));
        for module in &item.enclosure.modules {
            assert_eq!(v, module.version);
        }
    }

    if false {
        _ = generate_password("kys");
    }

    Ok(())
}
