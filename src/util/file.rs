use std::{
    fs,
    io,
    path
};

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    fs::File
};

/// A path to a backed up file.
///
/// Contains two paths, one to the file itself and one to its backup. Supports
/// methods to create and handle the backup.
pub struct SafePath {
    pub main: PathBuf,
    pub backup: PathBuf
}

/// Permissions with which to open a file.
///
/// Similar to [`File::options`].
#[derive(Clone, Copy)]
pub enum Mode {
    Read,
    ReadWrite,
    /// Fail if the file already exists.
    CreateWrite
}

pub type Error = io::Error;

pub type Result<T> = std::result::Result<T, Error>;

impl SafePath {
    /// Constructs a new `SafePath`.
    pub fn new<P, Q>(file_path: P, backup_path: Q) -> Self
        where
            P: Into<PathBuf>,
            Q: Into<PathBuf>
    {
        Self {
            main: file_path.into(),
            backup: backup_path.into()
        }
    }

    /// Returns an object that implements `Display` for printing the main path,
    /// possibly performing lossy conversion, like [`std::path::Path::display`].
    ///
    /// This is a convenience function equivalent to
    /// `SafePath::main().display()`.
    pub fn display(&self) -> path::Display {
        self.main.display()
    }

    /// Opens the main path in a manner determined by `mode`.
    pub fn open(&self, mode: Mode) -> Result<File> {
        use Mode::*;

        let mut opts = File::options();

        match mode {
            Read => opts.read(true),
            ReadWrite => opts.read(true).write(true),
            CreateWrite => opts.write(true).create_new(true)
        }.open(&self.main)
    }

    /// Backs up the file at `main`, copying it to `backup`.
    pub fn make_backup(&self) -> Result<()> {
        // ATOMICITY: Not atomic, but no data loss occurs on failure.
        fs::copy(&self.main, &self.backup)?;
        Ok(())
    }

    /// Verifies if the file at `backup` exists.
    pub fn is_backed_up(&self) -> Result<bool> {
        self.backup.try_exists()
    }

    /// Recovers the file at `backup`, moving it to and replacing `main`.
    pub fn recover(&self) -> Result<()> {
        // ATOMICITY: same as with `backup()`.
        fs::rename(&self.backup, &self.main)
    }

    /// Removes the file at `main`.
    pub fn remove(&self) -> Result<()> {
        fs::remove_file(&self.main)
    }

    /// Removes the file at `backup`.
    pub fn remove_backup(&self) -> Result<()> {
        fs::remove_file(&self.backup)
    }
}

/// Returns a path to a file in `backup_dir` suitable for a backup of
/// `file_path`.
///
/// A file name is considered to be a path devoid of path separators.
///
/// Does not create `backup_dir` or the returned file path if they do not exist.
///
/// This function is guaranteed to map any two different paths (as `file_path`)
/// with different absolute forms to two different file names. In other words,
/// every possible input has a (functionally) unique output, so a name collision
/// should not occur. This is only true for paths in their absolute and resolved
/// forms (for example, the presence of symlinks may nullify these guarantees).
pub fn backup_path_from<P, Q>(file_path: P, backup_dir: Q) -> PathBuf
    where
        P: AsRef<Path>,
        Q: Into<PathBuf>
{
    let backup_name = backup_name_from(file_path.as_ref());
    let mut result = Into::<PathBuf>::into(backup_dir);

    result.push(backup_name);
    result
}

/// Empties and resets `f`.
///
/// Truncates `f` to a length of 0 and rewinds it to the beginning of the file.
pub fn clear(f: &mut File) -> Result<()> {
    use std::io::Seek;

    f.set_len(0)?;
    f.rewind()
}

/// Returns a file name suitable for a backup of `file_path`.
///
/// Same unicity conditions as [`file_name_from`].
fn backup_name_from(file_path: &Path) -> OsString {
    const BACKUP_EXTENSION: &str = ".bak";

    let mut file_name = file_name_from(file_path);

    file_name.push(BACKUP_EXTENSION);

    file_name
}

/// Returns `path` as a file name.
///
/// A file name is considered to be a path devoid of path separators.
///
/// This function is guaranteed to map any two different paths to two different
/// file names. In other words, every possible input has a unique output, so a
/// name collision cannot occur.
fn file_name_from(path: &Path) -> OsString {
    use path_absolutize::Absolutize;
    use std::os::unix::ffi::OsStrExt;

    const SEP_SUBSTITUTE: char = '%';
    const SEP_SUBSTITUTE_STR: &str = "%";
    const ESCAPED_SUBSTITUTE: &str = "%%";

    // TODO: use `std::path::absolute` once available
    // It seems that `absolutize()` can never fail.
    let path = path.absolutize().unwrap();
    let path = path.as_os_str();

    // The length of the result is equal to that of `path` if the latter
    // contains no substitute characters.
    let mut result = OsString::with_capacity(path.len());

    for &b in path.as_bytes() {
        if b == path::MAIN_SEPARATOR as u8 {
            result.push(SEP_SUBSTITUTE_STR);
        } else if b == SEP_SUBSTITUTE as u8  {
            result.push(ESCAPED_SUBSTITUTE);
        } else {
            result.push(OsStr::from_bytes(&[b]))
        }
    }

    result
}
