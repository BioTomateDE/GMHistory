use indicatif::ProgressBar;
use indicatif::ProgressDrawTarget;
use indicatif::ProgressStyle;
use reqwest::ClientBuilder;
use reqwest::IntoUrl;
use reqwest::Response;

use crate::error::*;
use crate::tempslop::OpenTempFile;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub fn new() -> MyResult<Self> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(360))
            .build()
            .ctx("Failed to build reqwest client")?;
        Ok(Self { client })
    }

    pub async fn send_request(&self, url: impl IntoUrl) -> MyResult<Response> {
        let url = url.as_str();
        self.client
            .get(url)
            .send()
            .await
            .ctx(|| format!("Failed to send GET request to {url}"))?
            .error_for_status()
            .ctx(|| format!("Recieved non-success HTTP response code for GET request to {url}"))
    }

    pub async fn download(&self, url: impl IntoUrl, target_dir: &Path) -> MyResult<OpenTempFile> {
        let mut response: Response = self.send_request(url).await?;
        let content_length = response
            .content_length()
            .ctx("No content length specified; possibly a 404 page")?;

        let mut open_tempfile = OpenTempFile::new(target_dir)?;
        let pb = make_progress_bar(content_length);

        // This takes the majority of time
        while let Some(chunk) = response.chunk().await.ctx("Failed to get response bytes")? {
            open_tempfile
                .file_mut()
                .write_all(&chunk)
                .ctx("Failed to write response bytes to file")?;
            pb.inc(chunk.len() as u64);
        }

        pb.finish_and_clear();
        Ok(open_tempfile)
    }
}

fn make_progress_bar(content_length: u64) -> ProgressBar {
    let pb = ProgressBar::new(content_length);
    #[expect(clippy::literal_string_with_formatting_args)]
    let style = ProgressStyle::with_template("{percent:>3}% [{bar:50}] {bytes}/{total_bytes}");
    pb.set_style(style.unwrap().progress_chars("=> "));
    pb.set_draw_target(ProgressDrawTarget::stdout_with_hz(4));
    pb
}
