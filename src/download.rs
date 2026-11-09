use crate::{error::*, util::ignore_notfound};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Default)]
struct TempStuff {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct Downloader {
    client: reqwest::Client,
    tempfile: PathBuf,
    tempdir: PathBuf,
}

impl Downloader {
    pub fn new(root_dir: &Path) -> MyResult<Self> {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(240))
            .build()
            .ctx("Failed to build reqwest client")?;
        let tempfile = root_dir.join(".tempfile1.tmp");
        let tempdir = root_dir.join(".archive.tmp.d");

        let tempfile2 = tempfile.clone();
        let tempdir2 = tempdir.clone();

        // This will fail if already set, so only create one Downloader
        // triggers for SIGINT and for SIGTERM (`ctrlc` crate feature `termination`)
        ctrlc::set_handler(move || {
            remove_temp(&tempfile2, &tempdir2);
            std::process::exit(130); // SIGINT
        })
        .ctx("Failed to set Ctrl+C handler")?;

        remove_temp(&tempfile, &tempdir);

        Ok(Self {
            client,
            tempfile,
            tempdir,
        })
    }

    pub fn temp_download_file(&self) -> &Path {
        &self.tempfile
    }

    pub fn temp_archive_dir(&self) -> &Path {
        &self.tempdir
    }

    pub fn remove_temp(&self) {
        remove_temp(&self.tempfile, &self.tempdir);
    }

    pub async fn send_request(&self, url: &str) -> MyResult<reqwest::Response> {
        self.client
            .get(url)
            .send()
            .await
            .ctx(|| format!("Failed to send GET request to {url}"))?
            .error_for_status()
            .ctx(|| format!("Recieved non-success HTTP response code for GET request to {url}"))
    }
}

impl Drop for Downloader {
    fn drop(&mut self) {
        self.remove_temp();
    }
}

fn remove_temp(file_path: &Path, dir_path: &Path) {
    if let Err(e) = ignore_notfound(std::fs::remove_file(file_path)) {
        warn!(e, "Failed to remove tempfile `{}`", file_path.display());
    }
    if let Err(e) = ignore_notfound(std::fs::remove_dir_all(dir_path)) {
        warn!(e, "Failed to remove tempdir `{}`", dir_path.display());
    }
}
