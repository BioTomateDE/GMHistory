use crate::{error::*, util::ignore_notfound, warn};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError},
};

static TEMP_ITEMS: Mutex<TempItems> = Mutex::new(TempItems::new());

#[derive(Debug)]
struct TempItems {
    items: Vec<(TempItemKind, PathBuf)>,
    counter: usize,
}

impl TempItems {
    const fn new() -> Self {
        Self {
            items: Vec::new(),
            counter: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TempItemKind {
    File,
    Dir,
}
impl TempItemKind {
    fn word(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Dir => "dir",
        }
    }
}

#[derive(Debug)]
pub struct TempFile(PathBuf);
impl TempFile {
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn rename(self, new_path: &Path) -> MyResult<()> {
        rename(self.path(), new_path, TempItemKind::File)
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        remove_tempfile(&self.0);
    }
}

#[derive(Debug)]
pub struct TempDir(PathBuf);
impl TempDir {
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn rename(self, new_path: &Path) -> MyResult<()> {
        rename(self.path(), new_path, TempItemKind::Dir)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        remove_tempdir(&self.0);
    }
}

#[derive(Debug)]
pub struct OpenTempFile {
    path: PathBuf,
    file: File,
}
impl OpenTempFile {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn file(&self) -> &File {
        &self.file
    }
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }
    pub fn rename(self, new_path: &Path) -> MyResult<()> {
        rename(self.path(), new_path, TempItemKind::File)
    }
}
impl Drop for OpenTempFile {
    fn drop(&mut self) {
        // TODO: does this need to unlock the file first?
        remove_tempfile(&self.path);
    }
}

#[allow(clippy::significant_drop_tightening)] // lint is currently stupid asf
fn new_temp_item(dir: &Path, kind: TempItemKind) -> MyResult<PathBuf> {
    let mut temp_items = TEMP_ITEMS.lock().ctx("Mutex is poisoned")?;
    let filename: String = match kind {
        TempItemKind::File => format!(".download{}.tmp", temp_items.counter),
        TempItemKind::Dir => format!(".archive{}.tmp.d", temp_items.counter),
    };
    let path: PathBuf = dir.join(filename);
    if path.exists() {
        bail!(
            "Temp item already exists! This is really bad: {}",
            path.display(),
        );
    }
    temp_items.counter += 1;
    temp_items.items.push((kind, path.clone()));
    Ok(path)
}

pub fn new_tempfile(inside_dir: &Path) -> MyResult<TempFile> {
    new_temp_item(inside_dir, TempItemKind::File)
        .map(TempFile)
        .ctx(|| {
            format!(
                "Failed to create tempfile inside `{}`",
                inside_dir.display()
            )
        })
}

pub fn new_open_tempfile(inside_dir: &Path) -> MyResult<OpenTempFile> {
    let path: PathBuf = new_temp_item(inside_dir, TempItemKind::File)?;
    let file = File::create_new(&path).ctx("Failed to create tempfile")?;
    file.try_lock().ctx("Failed to lock created tempfile")?;
    Ok(OpenTempFile { path, file })
}

pub fn new_tempdir(inside_dir: &Path) -> MyResult<TempDir> {
    new_temp_item(inside_dir, TempItemKind::Dir)
        .map(TempDir)
        .ctx(|| format!("Failed to create tempdir inside `{}`", inside_dir.display()))
}

pub fn set_ctrlc_handler() {
    // This will fail if already set, so only create one Downloader
    // triggers for SIGINT and for SIGTERM (`ctrlc` crate feature `termination`)
    ctrlc::set_handler(|| {
        cleanup();
        std::process::exit(130); // SIGINT
    })
    .expect("Failed to set Ctrl+C handler");
}

pub fn cleanup() {
    let mut temp_items = match TEMP_ITEMS.lock() {
        Ok(items) => items,
        Err(poison_error) => poison_error.into_inner(),
    };
    temp_items.items.clear();
}

fn remove_tempfile(path: &Path) {
    if let Err(e) = ignore_notfound(std::fs::remove_file(path)) {
        warn!(e, "Failed to remove tempfile `{}`", path.display());
    }
}

fn remove_tempdir(path: &Path) {
    if let Err(e) = ignore_notfound(std::fs::remove_dir_all(path)) {
        warn!(e, "Failed to remove tempdir `{}`", path.display());
    }
}

fn rename(from: &Path, to: &Path, kind: TempItemKind) -> MyResult<()> {
    let Err(error) = std::fs::rename(from, to) else {
        return Ok(());
    };

    Err(err!(
        error,
        "Failed to rename temp{} from `{}` to `{}`",
        kind.word(),
        from.display(),
        to.display(),
    ))
}
