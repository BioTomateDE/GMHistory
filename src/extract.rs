use crate::cli_panic;
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

pub fn invoke_7zip(archive_file: &Path, destination_dir: &Path, password: Option<&str>) {
    let mut output_flag = OsString::with_capacity(destination_dir.as_os_str().len() + 2);
    output_flag.push("-o");
    output_flag.push(destination_dir);

    let mut cmd = Command::new("7z");
    cmd.arg("-bso0"); // makes it shut up (except progress indicator)
    cmd.arg("-y"); // assumes yes on all queries (replace?)
    cmd.arg("x");
    cmd.arg(archive_file);
    cmd.arg(output_flag);
    if let Some(pw) = password {
        cmd.arg(format!("-p{pw}"));
    }
    let status: ExitStatus = cmd.status().expect("7z waiting failed");
    if !status.success() {
        cli_panic!("7-Zip failed with status {status}, see above");
    }
}
