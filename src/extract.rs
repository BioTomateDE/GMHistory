use crate::asyncslop::EXTRACT_LIMIT;
use crate::error::*;
use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;

// TODO: ideally get a rust crate that can extract these installers, but thats kinda unrealistic.
// maybe use the standard zip crate for zip extractions tho? not a bottleneck so idk

pub fn probe_7zip() -> MyResult<()> {
    let ok = std::process::Command::new("7z")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    if ok {
        return Ok(());
    }
    log::error!("You need to install 7-Zip (command 7z not found)");
    Err(err!("You need to install 7-Zip"))
}

pub async fn invoke_7zip(archive_file: &Path, destination_dir: &Path, password: Option<&str>) -> MyResult<()> {
    let _permit = EXTRACT_LIMIT.acquire().await.unwrap();

    let mut output_flag = OsString::with_capacity(destination_dir.as_os_str().len() + 2);
    output_flag.push("-o");
    output_flag.push(destination_dir);

    let mut cmd = tokio::process::Command::new("7z");
    _ = cmd
        .arg("-bso0") // makes it shut up (except progress indicator)
        .arg("-bd") // disables progress indicator
        .arg("-y") // assumes yes on all queries (like replacing existing files)
        .arg("x")
        .arg(archive_file)
        .arg(output_flag)
        .kill_on_drop(true);
    if let Some(pw) = password {
        _ = cmd.arg(format!("-p{pw}"));
    }
    let output = cmd.output().await.ctx("Could not invoke 7-Zip")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(err!("7-Zip failed with status {}: <<\n{stderr}>>", output.status))
}
