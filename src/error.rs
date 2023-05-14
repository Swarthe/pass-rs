use crate::{
    env,
    backup,
    input_pw,
    serial,
    find
};

use crate::{
    err,
    warn
};

use crate::env::PROGNAME;

use crate::util::file::SafePath;

use crate::util::{
    user_io,
    crypt,
    file,
    proc,
    clip,
    record
};

use std::{
    fmt,
    io
};

use std::fmt::Display;

/// Program errors.
pub enum Error {
    Environment(env::Error),
    ReadingInput(user_io::Error),
    InputPw(input_pw::Error),

    ReadingHeader(crypt::header::Error),
    WritingHeader(crypt::header::Error),
    Crypt(crypt::Error),
    OpeningFile(file::Error, file::Mode, SafePath),
    ReadingStdin(user_io::Error),
    FileSerial(serial::Error),
    InputSerial(serial::Error),

    FindingRecord(find::Error),
    /// name of the record, and the group to which it is added
    AddingRecord(record::Error, String, String),
    SerialisingRecord(serial::Error),

    Clipboard(clip::Error),
    SecuringMemory(proc::Error),
    ExposingMemory(proc::Error),
    StartingProcess(proc::Error),

    RecoveringBackup(backup::Error, SafePath),
    MakingBackup(file::Error, SafePath),
    ClearingFile(file::Error),
    RemovingFile(file::Error, SafePath),
    RemovingBackup(file::Error, SafePath),
}

pub type Result<T> = std::result::Result<T, Error>;

/// User advice messages.
///
/// Typically recommending actions to be manually done by the user, if failed by
/// the program.
pub enum Advice {
    ViewingUsage,
    CreatingFile,
    SpecifyingFile,
    MovingBackup,
    RemovingBackup,
    RecoveringBackup,
    RemovingFile,
    InvalidFile,
    IncorrectPassword,
    InvalidInput
}

impl Error {
    /// XXX: also prints advice if applicable
    pub fn print_full(self) {
        err!("{self}");

        if let Some(a) = self.advice() {
            eprintln!("{a}");
        }
    }

    /// XXX: also prints advice if applicable
    pub fn warn_full(self) {
        warn!("{self}");

        if let Some(a) = self.advice() {
            eprintln!("{a}");
        }
    }

    /// Returns user advice relevant to this error if applicable.
    pub fn advice(&self) -> Option<Advice> {
        use Error::*;
        use env::Error::*;
        use backup::Error::{RemovalRefusal, File, Removal};
        use crypt::Error::DecryptingBlock;

        use file::Mode::CreateWrite;

        use io::ErrorKind::{UnexpectedEof, NotFound};

        Some(match self {
            Environment(ParsingArgs(..)) =>
                Advice::ViewingUsage,
            Environment(ResolvingDataPath(..)) =>
                Advice::SpecifyingFile,
            ReadingHeader(e) if e.kind() == UnexpectedEof =>
                Advice::InvalidFile,
            Crypt(DecryptingBlock) =>
                Advice::IncorrectPassword,
            FileSerial(..) =>
                Advice::InvalidFile,
            InputSerial(..) =>
                Advice::InvalidInput,

            RecoveringBackup(RemovalRefusal, ..) =>
                Advice::MovingBackup,
            RecoveringBackup(File(e) | Removal(e), ..) if e.kind() != NotFound =>
                Advice::RecoveringBackup,
            OpeningFile(_, CreateWrite, ..) =>
                Advice::SpecifyingFile,
            OpeningFile(e, ..) if e.kind() == NotFound =>
                Advice::CreatingFile,
            RemovingFile(e, ..) if e.kind() != NotFound =>
                Advice::RemovingFile,
            RemovingBackup(..) =>
                Advice::RemovingBackup,

            _ => return None
        })
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        use file::Mode;

        match self {
            Environment(e) =>
                write!(f, "{e}"),
            ReadingInput(e) =>
                write!(f, "{e}"),
            InputPw(e) =>
                write!(f, "{e}"),

            ReadingHeader(e) =>
                write!(f, "cannot read file header: {e}"),
            WritingHeader(e) =>
                write!(f, "cannot write header to file: {e}"),
            Crypt(e) =>
                write!(f, "{e}"),
            OpeningFile(e, mode, p) => match mode {
                Mode::CreateWrite =>
                    write!(f, "cannot create '{}': {e}", p.display()),
                _ =>
                    write!(f, "cannot open '{}': {e}", p.display())
            }
            ReadingStdin(e) =>
                write!(f, "cannot read stdin: {e}"),
            FileSerial(e) =>
                write!(f, "invalid file contents: {e}"),
            InputSerial(e) =>
                write!(f, "invalid input: {e}"),

            FindingRecord(e) =>
                write!(f, "{e}"),
            AddingRecord(e, name, dest) =>
                write!(f, "cannot create '{name}' in '{dest}': {e}"),
            SerialisingRecord(e) =>
                write!(f, "{e}"),

            Clipboard(e) =>
                write!(f, "{e}"),
            SecuringMemory(e) =>
                write!(f, "cannot secure process memory: {e}"),
            ExposingMemory(e) =>
                write!(f, "cannot disable process memory protections: {e}"),
            StartingProcess(e) =>
                write!(f, "cannot start clipboard process: {e}"),

            RecoveringBackup(e, p) =>
                write!(f, "cannot recover backup '{}': {e}", p.backup.display()),
            MakingBackup(e, p) =>
                write!(f, "cannot backup '{}': {e}", p.display()),
            ClearingFile(e) =>
                write!(f, "cannot clear pass file: {e}"),
            RemovingFile(e, p) =>
                write!(f, "cannot recover backup '{}': {e}", p.backup.display()),
            RemovingBackup(e, p) =>
                write!(f, "cannot remove backup '{}': {e}", p.backup.display())
        }
    }
}

impl From<env::Error> for Error {
    fn from(e: env::Error) -> Self {
        Self::Environment(e)
    }
}

impl From<input_pw::Error> for Error {
    fn from(e: input_pw::Error) -> Self {
        Self::InputPw(e)
    }
}

impl From<crypt::Error> for Error {
    fn from(e: crypt::Error) -> Self {
        Self::Crypt(e)
    }
}

impl From<serial::Error> for Error {
    fn from(e: serial::Error) -> Self {
        Self::FileSerial(e)
    }
}

impl From<find::Error> for Error {
    fn from(e: find::Error) -> Self {
        Self::FindingRecord(e)
    }
}

impl From<clip::Error> for Error {
    fn from(e: clip::Error) -> Self {
        Self::Clipboard(e)
    }
}

impl From<user_io::Error> for Error {
    fn from(e: user_io::Error) -> Self {
        Self::ReadingInput(e)
    }
}

impl Display for Advice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Advice::*;

        match self {
            ViewingUsage =>
                write!(f, "Try '{PROGNAME} -h' for more information."),
            SpecifyingFile =>
                write!(f, "Try '{PROGNAME} -f' to specify a pass file."),
            CreatingFile =>
                write!(f, "Try '{PROGNAME} -C' to create a pass file."),
            RemovingBackup =>
                write!(f, "Try manually removing the backup file."),
            RecoveringBackup =>
                write!(f, "Try manually recovering the backup file."),
            RemovingFile =>
                write!(f, "Try manually removing the file."),
            MovingBackup =>
                write!(f, "Rename or move the backup file to continue anyway."),
            InvalidFile =>
                write!(f, "The pass file may be invalid."),
            InvalidInput =>
                // TODO: point to ron documentation/examples or something
                write!(f, "The input format might be invalid."),
            IncorrectPassword =>
                write!(f, "The entered password may be incorrect.")
        }
    }
}
