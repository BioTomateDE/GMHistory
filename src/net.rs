use crate::asyncslop::get_download_limit;
use crate::error::*;
use crate::progressbar::DownloadBar;
use crate::tempslop::OpenTempFile;
use reqwest::Client;
use reqwest::IntoUrl;
use reqwest::Response;
use std::io::Write;
use std::path::Path;
use std::sync::LazyLock;

static CLIENT: LazyLock<Client> = LazyLock::new(make_client);

fn make_client() -> Client {
    Client::builder()
        .user_agent("GMHistory (Rust reqwest)")
        .build()
        .unwrap()
}

/// Panic early if something's wrong
pub fn init_client() {
    _ = *CLIENT;
}

pub async fn send_get_request(url: impl IntoUrl) -> MyResult<Response> {
    let url = url.as_str();
    CLIENT
        .get(url)
        .send()
        .await
        .ctx(|| format!("Failed to send GET request to {url}"))?
        .error_for_status()
        .ctx(|| format!("Recieved non-success HTTP response code for GET request to {url}"))
}

pub async fn fetch_text(url: impl IntoUrl) -> MyResult<String> {
    let resp: Response = send_get_request(url).await?;
    resp.text().await.ctx("Failed to get response text")
}

pub async fn download(
    url: impl IntoUrl,
    target_dir: &Path,
    bar: &DownloadBar,
) -> MyResult<OpenTempFile> {
    let url: reqwest::Url = url.into_url().ctx("Bad URL")?;
    let limit = get_download_limit(&url);
    let _permit = limit.acquire().await.unwrap();

    let mut response: Response = send_get_request(url.clone()).await?;

    let mut open_tempfile = OpenTempFile::new(target_dir)?;

    // This takes the majority of time
    while let Some(chunk) = response.chunk().await.ctx("Failed to get response bytes")? {
        open_tempfile
            .file_mut()
            .write_all(&chunk)
            .ctx("Failed to write response bytes to file")?;
        bar.inc(chunk.len() as u64);
    }
    Ok(open_tempfile)
}
