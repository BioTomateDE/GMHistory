use crate::error::*;
use crate::util::ignore_notfound;
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

const TIP: &str = "Only run one instance of this program at a time! Delete the tempfile and try again";

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

///////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct TempFile(PathBuf);

impl TempFile {
    pub fn new_for_ghidra() -> MyResult<Self> {
        let ctx = "Failed to create tempdir in tmpfs";
        let mut temp_items = TEMP_ITEMS.lock().ctx("Mutex is poisoned").ctx(ctx)?;
        let filename = format!("gmhistory-ghidra-{}.c", temp_items.counter);
        let path = std::env::temp_dir().join(filename);
        create_file(&path, &mut temp_items).ctx(ctx)?;
        Ok(Self(path))
    }

    pub fn new(inside_dir: &Path) -> MyResult<Self> {
        let ctx = || format!("Failed to create tempfile inside `{}`", inside_dir.display());
        let mut temp_items = TEMP_ITEMS.lock().ctx("Mutex is poisoned").ctx(ctx)?;
        let filename = format!(".gmhistory-tempfile-{}.tmp", temp_items.counter);
        let path: PathBuf = inside_dir.join(filename);
        create_file(&path, &mut temp_items).ctx(ctx)?;
        Ok(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub async fn rename(self, new_path: &Path) -> MyResult<()> {
        rename(self.path(), new_path, "file").await
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        remove_tempfile(&self.0);
    }
}

///////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct TempDir(PathBuf);

impl TempDir {
    /// Ghidra is a little bitch and will throw an error if a part of the the path starts with a dot.
    pub fn new_for_ghidra() -> MyResult<Self> {
        let ctx = "Failed to create tempdir in tmpfs";
        let mut temp_items = TEMP_ITEMS.lock().ctx("Mutex is poisoned").ctx(ctx)?;
        let filename = format!("gmhistory-ghidra-{}.d", temp_items.counter);
        let path = std::env::temp_dir().join(filename);
        create_dir(&path, &mut temp_items).ctx(ctx)?;
        Ok(Self(path))
    }

    pub fn new_in(inside_dir: &Path) -> MyResult<Self> {
        let ctx = || format!("Failed to create tempdir inside `{}`", inside_dir.display());
        let mut temp_items = TEMP_ITEMS.lock().ctx("Mutex is poisoned").ctx(ctx)?;
        let filename = format!(".gmhistory-tempdir-{}.tmp", temp_items.counter);
        let path: PathBuf = inside_dir.join(filename);
        create_dir(&path, &mut temp_items).ctx(ctx)?;
        Ok(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub async fn rename(self, new_path: &Path) -> MyResult<()> {
        rename(self.path(), new_path, "dir").await
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        remove_tempdir(&self.0);
    }
}

///////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct OpenTempFile {
    tempfile: TempFile,
    file: File,
}

impl OpenTempFile {
    pub fn new(inside_dir: &Path) -> MyResult<Self> {
        let tempfile = TempFile::new(inside_dir)?;
        let file = File::create_new(tempfile.path()).ctx("Failed to create tempfile")?;
        file.try_lock().ctx("Failed to lock created tempfile")?;
        Ok(Self { tempfile, file })
    }

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

    pub async fn rename(self, new_path: &Path) -> MyResult<()> {
        rename(self.path(), new_path, "file").await
    }
}

///////////////////////////////////////////////////////////////////////////////

pub fn set_ctrlc_handler() {
    // This will fail if already set, so only create one Downloader
    // triggers for SIGINT and for SIGTERM (`ctrlc` crate feature `termination`)
    ctrlc::set_handler(|| {
        crate::progressbar::BAR_TOTAL.finish_and_clear();
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

async fn rename(from: &Path, to: &Path, kind: &'static str) -> MyResult<()> {
    let Err(error) = tokio::fs::rename(from, to).await else {
        return Ok(());
    };
    Err(err!(
        error,
        "Failed to rename temp{} from `{}` to `{}`",
        kind,
        from.display(),
        to.display(),
    ))
}

fn create_dir(path: &Path, temp_items: &mut TempItems) -> MyResult<()> {
    match std::fs::create_dir(path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err(err!("Tempdir already exists: {}", path.display()).ctx(TIP));
        }
        Err(e) => Err(e)?,
    }
    temp_items.counter += 1;
    temp_items.dirs.push(TempDir(path.to_path_buf()));
    Ok(())
}

fn create_file(path: &Path, temp_items: &mut TempItems) -> MyResult<()> {
    if path.exists() {
        return Err(err!("Tempfile already exists: {}", path.display()).ctx(TIP));
    }
    temp_items.counter += 1;
    temp_items.files.push(TempFile(path.to_path_buf()));
    Ok(())
}
