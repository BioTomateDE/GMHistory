use crate::error::*;
use std::fmt;
use std::fs::FileType;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

/// An entry in the filesystem. Either a file or a directory.
#[derive(Debug)]
pub struct FsEntry {
    pub path: PathBuf,
    pub kind: FileType,
}

impl FsEntry {
    pub fn filename(&self) -> &str {
        // This was validated in construction
        self.path.file_name().unwrap().to_str().unwrap()
    }

    pub fn is_dir(&self) -> bool {
        self.kind.is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.kind.is_file()
    }
}

impl fmt::Display for FsEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}) `{}`", file_kind(self.kind), self.path.display())
    }
}

pub fn listdir(dir_path: impl AsRef<Path>) -> MyResult<Vec<FsEntry>> {
    let dir_path = dir_path.as_ref();
    let ctx = || format!("Failed to read directory `{}`", dir_path.display());
    let mut entries: Vec<FsEntry> = Vec::new();
    for entry in std::fs::read_dir(dir_path).ctx(ctx)? {
        let entry: std::fs::DirEntry = entry.ctx(ctx)?;
        let path: PathBuf = entry.path();
        if path.file_name().unwrap().as_encoded_bytes().starts_with(b".") {
            continue;
        }
        let kind: FileType = entry
            .file_type()
            .ctx(|| format!("Failed to get filetype of `{}`", path.display()))
            .ctx(ctx)?;
        let _: &str = path
            .file_name()
            .unwrap()
            .to_str()
            .ctx(|| format!("Invalid UTF-8 in filename of path `{}`", path.display()))
            .ctx(ctx)?;
        entries.push(FsEntry { path, kind });
    }
    Ok(entries)
}

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

pub fn file_kind(file_type: FileType) -> &'static str {
    if file_type.is_file() {
        return "file";
    }
    if file_type.is_dir() {
        return "directory";
    }
    if file_type.is_symlink() {
        return "symlink";
    }
    cfg_select! {
        unix => unix_file_kind(file_type),
        _ => "<unknown file type>",
    }
}

#[cfg(unix)]
fn unix_file_kind(file_type: FileType) -> &'static str {
    use std::os::unix::fs::FileTypeExt;

    if file_type.is_block_device() {
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

pub async fn copy_file(source_path: impl AsRef<Path>, dest_path: impl AsRef<Path>) -> MyResult<()> {
    let source_path = source_path.as_ref();
    let dest_path = dest_path.as_ref();
    if dest_path.exists() {
        return Ok(());
    }
    let _: u64 = tokio::fs::copy(source_path, dest_path).await.ctx(|| {
        format!(
            "Failed to copy runner file `{}` to `{}`",
            source_path.display(),
            dest_path.display()
        )
    })?;
    Ok(())
}
