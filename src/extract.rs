use crate::error::*;
use std::{
    ffi::OsString,
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

pub fn probe_7zip() -> bool {
    Command::new("7z")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

pub fn invoke_7zip(
    archive_file: &Path,
    destination_dir: &Path,
    password: Option<&str>,
) -> MyResult<()> {
    let mut output_flag = OsString::with_capacity(destination_dir.as_os_str().len() + 2);
    output_flag.push("-o");
    output_flag.push(destination_dir);

    let mut cmd = Command::new("7z");
    _ = cmd.arg("-bso0"); // makes it shut up (except progress indicator)
    _ = cmd.arg("-y"); // assumes yes on all queries (replace?)
    _ = cmd.arg("x");
    _ = cmd.arg(archive_file);
    _ = cmd.arg(output_flag);
    if let Some(pw) = password {
        _ = cmd.arg(format!("-p{pw}"));
    }
    let status: ExitStatus = cmd.status().ctx("Could not invoke 7-Zip")?;
    if !status.success() {
        bail!("7-Zip failed with status {status}, see above");
    }
    Ok(())
}
