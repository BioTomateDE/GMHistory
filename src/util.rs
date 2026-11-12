use crate::error::*;
use std::fs::FileType;
use std::io::ErrorKind;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

fn ignore_kind(result: std::io::Result<()>, kind: ErrorKind) -> MyResult<()> {
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == kind => Ok(()),
        Err(e) => Err(Error::from(e)),
    }
}

pub fn ignore_notfound(result: std::io::Result<()>) -> MyResult<()> {
    ignore_kind(result, ErrorKind::NotFound)
}

pub fn ignore_exists(result: std::io::Result<()>) -> MyResult<()> {
    ignore_kind(result, ErrorKind::AlreadyExists)
}

pub fn mkdir(path: &Path) -> MyResult<()> {
    if let Ok(meta) = std::fs::metadata(path)
        && !meta.is_dir()
    {
        bail!(
            "Could not create directory `{}` because it already exists as a {}",
            path.display(),
            file_kind(meta.file_type())
        );
    }
    ignore_exists(std::fs::create_dir(path))
}

fn file_kind(file_type: FileType) -> &'static str {
    if file_type.is_dir() {
        "directory"
    } else if file_type.is_file() {
        "file"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_block_device() {
        "block device"
    } else if file_type.is_char_device() {
        "char device"
    } else if file_type.is_fifo() {
        "fifo"
    } else if file_type.is_socket() {
        "socket"
    } else {
        "<unknown file type>"
    }
}
