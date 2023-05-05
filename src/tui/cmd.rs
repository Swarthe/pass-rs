use Cmd::{Read, Edit, Meta};

use crate::find::{MatchKind, RecordPath};

use std::{num, fmt};

use std::fmt::Display;

use std::time::Duration;

/// The command to be executed.
pub enum Cmd {
    Read(ReadCmd),
    Edit(EditCmd),
    Meta(MetaCmd)
}

/// Obtaining or showing data.
pub enum ReadCmd {
    Show(Vec<RecordPath>),
    Clip(RecordPath),
    List(Option<Vec<RecordPath>>),
    Tree(Option<Vec<RecordPath>>),
    Export
}

/// Editing data.
pub enum EditCmd {
    // Group and item operations.
    Remove { paths: Vec<RecordPath> },
    // TODO: these 2 should work like createitem, with split name
    Move { src: RecordPath, dest: RecordPath },   // XXX: also used for renaming, possible also root
    Copy { src: RecordPath, dest: RecordPath },
    // Group operations.
    CreateGroup { dests_names: Vec<(RecordPath, String)> },
    // Item operations
    CreateItem { dests_names: Vec<(RecordPath, String)> },  // XXX: this and changevalue
                                                    // accept whitespace escapes
                                                    // (multiline values) for input
    ChangeValue { paths: Vec<RecordPath> },
}

/// TUI management and information.
pub enum MetaCmd {
    /// XXX: only affects temporary runtime conf
    SetOpt(OptVal),
    ShowConfig,
    ShowUsage(Option<Vec<CmdVerb>>),
    /// Exiting the TUI and saving.
    Exit,
    /// Exiting the TUI without saving.
    Abort
}

#[derive(Clone, Copy)]
pub enum OptVal {
    ClipTime(Duration),
    MatchKind(MatchKind)
}

/// Non-algebraic [`Cmd`] for parsing and validation.
#[derive(Clone, Copy)]
pub enum CmdVerb {
    Show,
    Clip,
    List,
    Tree,
    Export,

    Remove,
    Move,
    Copy,
    CreateItem,
    CreateGroup,
    ChangeValue,

    SetOption,
    ShowConfig,
    ShowUsage,
    Exit,
    Abort
}

pub enum Error {
    InvalidInput(shell_words::ParseError),
    InvalidCmd(String),
    MissingArg,
    ExtraArg(String),
    InvalidArg(String),
    InvalidName(RecordPath),
    InvalidIntArg(String, num::ParseIntError)
}

pub type Result<T> = std::result::Result<T, Error>;

impl Cmd {
    /// returns None if no command ('s' empty)
    pub fn from_str(s: &str) -> Result<Option<Self>> {
        let mut words = shell_words::split(s)
            .map_err(Error::InvalidInput)?;

        if words.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self::from_parts(
                CmdVerb::from_str(words.remove(0))?,
                words
            )?))
        }
    }
}

// TODO: some sort of advice "try 'help' for more info"
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            InvalidInput(e)     => write!(f, "invalid input: {e}"),
            InvalidCmd(c)       => write!(f, "invalid command '{c}'"),
            MissingArg          => write!(f, "missing argument"),
            ExtraArg(a)         => write!(f, "extra argument '{a}'"),
            InvalidName(r)      => write!(f, "invalid record name '{r}'"),
            InvalidArg(r)       => write!(f, "invalid argument '{r}'"),
            InvalidIntArg(a, e) => write!(f, "invalid argument '{a}': {e}")
        }
    }
}

impl Cmd {
    /// XXX: args can be empty
    fn from_parts(verb: CmdVerb, args: Vec<String>) -> Result<Self> {
        use CmdVerb::*;

        let have_args = !args.is_empty();
        let mut args = verb.check_args(args)?.into_iter();

        Ok(match verb {
            Export => Read(ReadCmd::Export),
            Exit => Meta(MetaCmd::Exit),
            Abort => Meta(MetaCmd::Abort),
            ShowConfig => Meta(MetaCmd::ShowConfig),

            Clip => Read(ReadCmd::Clip(into_next(args))),
            ChangeValue => Edit(EditCmd::ChangeValue { paths: into_collect(args) }),
            Show => Read(ReadCmd::Show(into_collect(args))),
            Remove => Edit(EditCmd::Remove { paths: into_collect(args) }),

            // By splitting the name from a path element, we guarantee that it
            // is valid as a new record name (doesn't contain separators).
            CreateGroup => Edit(EditCmd::CreateGroup {
                dests_names: split_each_name(args.map(RecordPath::from))?
            }),
            CreateItem => Edit(EditCmd::CreateItem {
                dests_names: split_each_name(args.map(RecordPath::from))?
            }),

            List => Read(ReadCmd::List(match have_args {
                true => Some(into_collect(args)),
                false => None,
            })),
            Tree => Read(ReadCmd::Tree(match have_args {
                true => Some(into_collect(args)),
                false => None,
            })),
            ShowUsage => Meta(MetaCmd::ShowUsage(match have_args {
                true => Some(
                    args.map(CmdVerb::from_str)
                        .collect::<Result<Vec<_>>>()?
                ),
                false => None,
            })),

            Move => Edit(EditCmd::Move {
                src: into_next(&mut args),
                dest: into_next(&mut args)
            }),
            Copy => Edit(EditCmd::Copy {
                src: into_next(&mut args),
                dest: into_next(&mut args)
            }),
            SetOption => Meta(MetaCmd::SetOpt(OptVal::new(
                into_next(&mut args),
                into_next(&mut args),
            )?))
        })
    }
}

impl OptVal {
    fn new(name: String, val: String) -> Result<Self> {
        Ok(match name.as_str() {
             "ct" | "clip-time" => Self::ClipTime(Duration::from_secs(
                val.parse::<u64>()
                    .map_err(|e| Error::InvalidIntArg(val, e))?
            )),

             "mk" | "match-kind" => Self::MatchKind(
                MatchKind::from_str(&val)
                    .ok_or(Error::InvalidArg(val))?
            ),

            _ => return Err(Error::InvalidArg(name))
        })
    }
}

impl CmdVerb {
    fn from_str<S: AsRef<str>>(s: S) -> Result<Self> {
        use CmdVerb::*;

        Ok(match s.as_ref() {
            "sh" | "show" => Show,
            "cl" | "clip" => Clip,
            "ls" | "list" => List,
            "tr" | "tree" => Tree,
            "ex" | "export" => Export,

            "rm" | "remove" => Remove,
            "mv" | "move" => Move,
            "cp" | "copy" => Copy,
            "mg" | "mkgrp" => CreateGroup,
            "mi" | "mkitm" => CreateItem,
            "cv" | "chval" => ChangeValue,

            "so" | "setopt" => SetOption,
            "sc" | "showconf" => ShowConfig,
            "hp" | "help" => ShowUsage,
            "et" | "exit" => Exit,
            "at" | "abort" => Abort,

            other => return Err(Error::InvalidCmd(other.to_owned()))
        })
    }

    // TODO: perhaps equivalent method in env
    // env could generally be cleaner like here
    /// we have to do this because some commands accept varying numbers of args
    /// (like `List`)
    /// returns `a` unchanged
    fn check_args(self, a: Vec<String>) -> Result<Vec<String>> {
        use CmdVerb::*;
        use Error::{MissingArg, ExtraArg};

        match self {
            Export | Exit | Abort | ShowConfig =>
                if a.is_empty() { Ok(a) } else { Err(ExtraArg(take(a, 0))) }

            Clip => match a.len() {
                1 => Ok(a),
                0 => Err(MissingArg),
                _ => Err(Error::ExtraArg(take(a, 1)))
            }

            Move | Copy | SetOption => match a.len() {
                2 => Ok(a),
                1 | 0 => Err(MissingArg),
                _ => Err(ExtraArg(take(a, 2)))
            }

            Show | CreateGroup | CreateItem | ChangeValue | Remove =>
                if !a.is_empty() { Ok(a) } else { Err(MissingArg) }

            List | Tree | ShowUsage => Ok(a)
        }
    }
}

/// splits each path of `paths` into the leading path and the trailing name
///
/// the trailing returned strings are guaranteed to be a valid record name
/// without path delimiters
fn split_each_name<I>(paths: I) -> Result<Vec<(RecordPath, String)>>
    where
        I: Iterator<Item = RecordPath>
{
    paths.map(|path| {
        // If the path only contains one element, root will be taken as the
        // leading path.
        let (leading, trailing) = path
            .split_last()
            .map_err(Error::InvalidName)?;

        Ok((leading, trailing.into_inner()))
    }).collect()
}

fn into_collect<I, J>(iter: impl Iterator<Item = I>) -> Vec<J>
    where
        I: Into<J>
{
    iter.map(Into::<J>::into).collect()
}

/// Panics XXX
fn into_next<I, J>(mut iter: impl Iterator<Item = I>) -> J
    where
        I: Into<J>
{
    iter.next().unwrap().into()
}

/// Moves value at `idx` out `v` and returns it.
///
/// Panics if `idx` is out of bounds of `v`.
fn take<T>(v: Vec<T>, idx: usize) -> T {
    v.into_iter().nth(idx).unwrap()
}
