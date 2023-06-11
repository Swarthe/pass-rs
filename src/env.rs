use crate::config;
use crate::tui;

use crate::find::{
    RecordPath,
    MatchKind
};

use crate::util::{
    xdg_path,
    file
};

use crate::util::file::SafePath;

use std::{
    fmt,
    io
};

use std::fmt::Display;

use std::{
    path::{Path, PathBuf},
    time::Duration
};

/// Commonly used data structures relevant to the environment, and useful for
/// parsing it.
pub mod prelude {
    pub use super::{
        Cmd, FileCmd,
        ReadCmd, ChangeCmd, CreateCmd
    };
}

/// Program binary name.
pub const PROGNAME: &str = env!("CARGO_BIN_NAME");

/// The command to be executed.
// TODO: maybe find way to improve such that there is no impurity (creating dirs)
//  maybe shouldnt even create safepath: only Option<PathBuf> for user provided
//  path (thus renaming mod to `args` or smth)
pub enum Cmd {
    ShowUsage(Usage),
    ShowVersion(Version),
    HandleFile(FileCmd, SafePath)
}

/// Handling a pass file.
pub enum FileCmd {
    Read(ReadCmd),
    Change(ChangeCmd),
    Create(CreateCmd)
}

/// Reading data from a pass file.
pub enum ReadCmd {
    /// Displaying an item.
    Show(Vec<RecordPath>, MatchKind),
    /// Copying an item to the clipboard, and keeping it there for a `Duration`.
    Clip(RecordPath, MatchKind, Duration),
    /// Displaying the names of a group's records, or of an item.
    List(Option<Vec<RecordPath>>, MatchKind),
    /// Displaying a tree representation of a group, or an item. Only the names
    /// of the records are shown, and their layout. If no target is provided,
    /// the root group is considered the target.
    Tree(Option<Vec<RecordPath>>, MatchKind),
    /// Displaying a serial representation of the data.
    Export
}

/// Editing a pass file.
pub enum ChangeCmd {
    /// Modifying the data.
    Modify(tui::Config),
    /// Changing the password used to access the data.
    ChangePassword
}

/// Creating a new pass file.
pub enum CreateCmd {
    /// Creating a pass file with from input data in serial form.
    Import,
    /// Creating a pass file with no data, and with specified name for root
    /// group.
    CreateEmpty(String)
}

pub struct Usage;

/// XXX: version and copyright
pub struct Version;

pub enum Error {
    ParsingArgs(lexopt::Error),
    ResolvingDataPath(xdg_path::Error),
    CreatingBackupDir(io::Error, PathBuf)
}

pub type Result<T> = std::result::Result<T, Error>;

impl Cmd {
    /// Parses command line arguments and sets up the environment.
    ///
    /// May create some standard XDG directories if necessary.
    pub fn from_env() -> Result<Self> {
        use FileCmdVerb::*;
        use Cmd::*;

        use lexopt::prelude::*;
        use lexopt::Parser;
        use lexopt::Error::Custom;

        let mut parser = Parser::from_env();
        let mut opts = FileCmdOpts::default();
        let mut cmd = FileCmdVerb::default();
        let mut file_path = Option::<PathBuf>::None;

        while let Some(arg) = parser.next()? {
            let old_cmd = cmd;

            match arg {
                Short('c') | Long("clip") => cmd = Clip,
                Short('l') | Long("list") => cmd = List,
                Short('t') | Long("tree") => cmd = Tree,

                Short('e') | Long("exact") =>
                    opts.match_kind = MatchKind::Exact,
                Short('d') | Long("duration") =>
                    opts.clip_time = parser.value()?.parse()?,
                Short('f') | Long("file") =>
                    file_path = Some(parser.value()?.into()),

                Short('M') | Long("modify")    => cmd = Edit,
                Short('P') | Long("change-pw") => cmd = ChangePassword,

                Short('E') | Long("export") => cmd = Export,
                Short('I') | Long("import") => cmd = Import,

                Short('C') | Long("create") => {
                    opts.root_name = parser.value()?.parse()?;
                    cmd = CreateEmpty;
                }

                Value(v) => opts.record_paths_raw.push(v.parse()?),

                Short('h') | Long("help") => return Ok(ShowUsage(Usage)),
                Short('v') | Long("version") => return Ok(ShowVersion(Version)),

                _ => return Err(arg.unexpected().into())
            }

            // Verify that the user doesn't pass multiple conflicting options.
            if old_cmd.conflicts_with(cmd) {
                return Err(Custom("conflicting options".into()).into());
            }
        }

        let file_cmd = FileCmd::from_parts(cmd, opts)?;

        let data_dir = xdg_path::data_dir(PROGNAME)?;

        let file_path = file_path.unwrap_or_else(|| {
            data_dir.join(config::DEFAULT_PASS_FILE_NAME)
        });

        let path = ensured_path_from(file_path, data_dir)?;

        Ok(Cmd::HandleFile(file_cmd, path))
    }
}

impl Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\
Usage: {} [OPTION...] [TARGET...]
Securely manage hierarchical data.

  -c, --clip        copy target item to primary clipboard instead of displaying
  -l, --list        list the target's contents (root if not specified)
  -t, --tree        display a tree of the target (root if not specified)

  -e, --exact       find exact match of target (default: fuzzy match)
  -d, --duration    time in seconds to keep target in clipboard (default: {})
  -f, --file        specify a pass file (default: standard data file)

  -M, --modify      launch editing interface (respects '-e' and '-d')
  -P, --change-pw   change the pass file's password

  -E, --export      output data in serial form
  -I, --import      create a pass file from serial data (read from stdin)
  -C, --create      create an empty pass file with the specified root name

  -h, --help        display this help text
  -v, --version     display version information

Note: By default, the target item is printed to standard output.
      Targets are passed as dot-separated record paths.
      Passing a group as a target item implies its child item '{}'.

Example: pass -d5 -c foo.bar",
            PROGNAME,
            config::DEFAULT_CLIP_TIME,
            config::DEFAULT_ITEM
        )
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const LICENSE_URL: &str = "https://gnu.org/licenses/gpl.html";

        write!(
            f, concat!(
env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"), "
Copyright (C) 2023 ", env!("CARGO_PKG_AUTHORS"), ".
License ", env!("CARGO_PKG_LICENSE"), " <{}>.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law."
            ), LICENSE_URL
        )
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            ParsingArgs(e) =>
                write!(f, "{e}"),
            ResolvingDataPath(e) =>
                write!(f, "cannot resolve XDG data directory: {e}"),
            CreatingBackupDir(e, p) =>
                write!(f, "backup directory '{}': {e}", p.display()),
        }
    }
}

impl From<lexopt::Error> for Error {
    fn from(e: lexopt::Error) -> Self {
        Self::ParsingArgs(e)
    }
}

impl From<xdg_path::Error> for Error {
    fn from(e: xdg_path::Error) -> Self {
        Self::ResolvingDataPath(e)
    }
}

/// Options relevant to handling a file.
struct FileCmdOpts {
    /// The path of the target `Record`.
    record_paths_raw: Vec<String>,
    match_kind: MatchKind,
    clip_time: u64,
    root_name: String
}

/// Non-algebraic [`FileCmd`] for parsing.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum FileCmdVerb {
    #[default]
    Show,
    Clip,
    List,
    Tree,

    Edit,
    ChangePassword,

    Export,
    Import,
    CreateEmpty
}

impl FileCmd {
    fn from_parts(cmd: FileCmdVerb, opts: FileCmdOpts) -> Result<Self> {
        use FileCmd::*;
        use FileCmdVerb::*;

        use tui::Config;
        use lexopt::Error::{MissingValue, UnexpectedArgument};

        let FileCmdOpts {
            record_paths_raw: rec_paths_raw,
            match_kind,
            clip_time,
            root_name
        } = opts;

        let clip_time = Duration::from_secs(clip_time);

        // Check the validity of the arguments.
        match cmd {
            Show | Clip
            if rec_paths_raw.is_empty() =>
                return Err(MissingValue { option: None }.into()),

            Clip
            if rec_paths_raw.len() > 1 =>
                // `record_paths` second element was verified to exist.
                return Err(UnexpectedArgument(
                    take(rec_paths_raw, 1).into()
                ).into()),

            Edit | ChangePassword | Export | Import
            if !rec_paths_raw.is_empty() =>
                // `record_paths` is not empty so its first element exists.
                return Err(UnexpectedArgument(
                    take(rec_paths_raw, 0).into()
                ).into()),

            _ => ()
        }

        let rec_paths = rec_paths_raw.into_iter()
            .map(RecordPath::from)
            .collect::<Vec<_>>();

        Ok(match cmd {
            Show => Read(ReadCmd::Show(rec_paths, match_kind)),
            Clip => Read(ReadCmd::Clip(take(rec_paths, 0), match_kind, clip_time)),
            List => Read(ReadCmd::List(empty_or_some(rec_paths), match_kind)),
            Tree => Read(ReadCmd::Tree(empty_or_some(rec_paths), match_kind)),

            Edit => Change(ChangeCmd::Modify(Config { match_kind, clip_time })),
            ChangePassword => Change(ChangeCmd::ChangePassword),

            Export => Read(ReadCmd::Export),
            Import => Create(CreateCmd::Import),
            CreateEmpty => Create(CreateCmd::CreateEmpty(root_name))
        })
    }
}

impl Default for FileCmdOpts {
    fn default() -> Self {
        Self {
            /// The path of the target `Record`, root group by default.
            record_paths_raw: Default::default(),
            match_kind: Default::default(),
            clip_time: config::DEFAULT_CLIP_TIME,
            root_name: Default::default(),
        }
    }
}

impl FileCmdVerb {
    /// Verifies if `other` can logically supersede `self`.
    ///
    /// Returns true if `self` is neither equal to `other` nor the default
    /// command, and returns false otherwise. In other words, returns true if
    /// `other` cannot logically override `self.`
    fn conflicts_with(self, other: Self) -> bool {
        self != other && self != Self::default()
    }
}

/// Returns a [`SafePath`] with `file_path` as the main path, and a file
/// located in a subdirectory of `data_dir` as the backup path.
///
/// The data path and backup paths are created if they do not exist.
fn ensured_path_from(
    file_path: PathBuf,
    data_dir: PathBuf
) -> Result<SafePath> {
    use std::fs::create_dir_all;

    const BACKUP_DIR: &str = "backup";

    let backup_dir = joined(data_dir, BACKUP_DIR);

    if let Err(e) = create_dir_all(&backup_dir) {
        return Err(Error::CreatingBackupDir(e, backup_dir))
    }

    let backup_path = file::backup_path_from(&file_path, backup_dir);

    Ok(SafePath::new(file_path, backup_path))
}

/// Appends `other` to `path` and returns the result.
///
/// Doesn't allocate a new [`PathBuf`], unlike [`Path::join`].
fn joined<P: AsRef<Path>>(mut path: PathBuf, other: P) -> PathBuf {
    path.push(other);
    path
}

/// Moves value at `idx` out `v` and returns it.
///
/// Panics if `idx` is out of bounds of `v`.
fn take<T>(v: Vec<T>, idx: usize) -> T {
    v.into_iter().nth(idx).unwrap()
}

/// Converts `v` into an [`Option`].
///
/// Returns `None` if `v` is empty, otherwise `Some(v)`.
fn empty_or_some<T>(v: Vec<T>) -> Option<Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}
