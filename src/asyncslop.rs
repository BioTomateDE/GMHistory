use tokio::sync::Semaphore;

// i think archive.org doesn't like request spam, so the fetch limit might need to be lower idk

/// Limit for sending requests / downloading files from archive.org
pub static ARCHIVEORG_LIMIT: Semaphore = Semaphore::const_new(1);

/// Limit for sending requests / downloading files from all other sites
pub static DOWNLOAD_LIMIT: Semaphore = Semaphore::const_new(4);

// /// Limit for sending requests (`CLIENT.get(url).send().await`).
// pub static FETCH_LIMIT: Semaphore = Semaphore::const_new(4);
//
// /// Limit for downloading file chunks (`response.chunk().await`).
// pub static DOWNLOAD_LIMIT: Semaphore = Semaphore::const_new(3);

/// Limit for extracting archives using the 7-Zip CLI (`crate::extract::invoke_7zip()`).
/// This one is blocking instead of async.
/// Unlike fetch and download, it is bottlenecked by CPU and disk instead of network.
pub static EXTRACT_LIMIT: Semaphore = Semaphore::const_new(12);
