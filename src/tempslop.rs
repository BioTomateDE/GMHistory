use crate::{error::*, util::ignore_notfound, warn};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Mutex,
};

static TEMP_ITEMS: Mutex<TempItems> = Mutex::new(TempItems::new());

#[derive(Debug)]
struct TempItems {
    files: Vec<TempFile>,
    dirs: Vec<TempDir>,
    counter: usize,
}

impl TempItems {
    const fn new() -> Self {
        Self {
            files: Vec::new(),
            dirs: Vec::new(),
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
    const fn word(self) -> &'static str {
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
    #[expect(dead_code)]
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
    tempfile: TempFile,
    file: File,
}
impl OpenTempFile {
    pub fn path(&self) -> &Path {
        self.tempfile.path()
    }
    #[expect(dead_code)]
    pub const fn file(&self) -> &File {
        &self.file
    }
    pub const fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }
    pub fn rename(self, new_path: &Path) -> MyResult<()> {
        rename(self.path(), new_path, TempItemKind::File)
    }
}

const TIP: &str = "Only run one instance of this program at a time! Delete the tempfile and try again";

pub fn new_tempfile(inside_dir: &Path) -> MyResult<TempFile> {
    let ctx = || format!("Failed to create tempfile inside `{}`", inside_dir.display());
    let mut temp_items = TEMP_ITEMS.lock().ctx("Mutex is poisoned").ctx(ctx)?;
    let filename = format!(".download{}.tmp", temp_items.counter);
    let path: PathBuf = inside_dir.join(filename);
    if path.exists() {
        return Err(err!("Tempfile already exists: {}", path.display()).ctx(TIP).ctx(ctx));
    }
    temp_items.counter += 1;
    temp_items.files.push(TempFile(path.clone()));
    Ok(TempFile(path))
}

pub fn new_open_tempfile(inside_dir: &Path) -> MyResult<OpenTempFile> {
    let tempfile: TempFile = new_tempfile(inside_dir)?;
    let file = File::create_new(tempfile.path()).ctx("Failed to create tempfile")?;
    file.try_lock().ctx("Failed to lock created tempfile")?;
    Ok(OpenTempFile { tempfile, file })
}

pub fn new_tempdir(inside_dir: &Path) -> MyResult<TempDir> {
    let ctx = || format!("Failed to create tempdir inside `{}`", inside_dir.display());
    let mut temp_items = TEMP_ITEMS.lock().ctx("Mutex is poisoned").ctx(ctx)?;
    let filename = format!(".archive{}.tmp.d", temp_items.counter);
    let path: PathBuf = inside_dir.join(filename);
    if path.exists() {
        return Err(err!("Tempdir already exists: {}", path.display()).ctx(TIP).ctx(ctx));
    }
    temp_items.counter += 1;
    temp_items.dirs.push(TempDir(path.clone()));
    Ok(TempDir(path))
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
    temp_items.files.clear();
    temp_items.dirs.clear();
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
