use crate::error::*;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Limit for sending requests / downloading files from `archive.org`.
pub static ARCHIVEORG_LIMIT: Semaphore = Semaphore::const_new(1);

/// Limit for sending requests / downloading files from all other sites.
pub static DOWNLOAD_LIMIT: Semaphore = Semaphore::const_new(4);

/// Limit for extracting archives using the 7-Zip CLI (`crate::extract::invoke_7zip()`).
/// This one is blocking instead of async.
/// Unlike fetch and download, it is bottlenecked by CPU and disk instead of network.
pub static EXTRACT_LIMIT: Semaphore = Semaphore::const_new(12);

pub fn get_download_limit(url: &reqwest::Url) -> &'static Semaphore {
    let Some(domain) = url.domain() else {
        return &DOWNLOAD_LIMIT;
    };
    if domain == "archive.org" || domain.ends_with(".archive.org") {
        &ARCHIVEORG_LIMIT
    } else {
        &DOWNLOAD_LIMIT
    }
}

pub async fn join_tasks(mut tasks: JoinSet<MyResult<()>>) -> MyResult<()> {
    let total = tasks.len();
    let mut failed: usize = 0;
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                log::log!(log::Level::Error, "{e}");
                failed += 1;
            }
            Err(e) => bail!(e, "Task panicked"),
        }
    }

    if failed > 0 {
        bail!("{failed}/{total} tasks failed");
    }
    log::info!("All done!");
    Ok(())
}
