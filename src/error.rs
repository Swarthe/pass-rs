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
    ReadingStdin(user_io::Error),
    ReadingHeader(crypt::header::Error),
    WritingHeader(crypt::header::Error),
    Crypt(crypt::Error),
    FileSerial(serial::Error),
    InputSerial(serial::Error),
    FindingRecord(find::Error),
    AddingRecord(record::Error, String),
    SerialisingRecord(serial::Error),
    Clipboard(clip::Error),
    SecuringMemory(proc::Error),
    ExposingMemory(proc::Error),
    StartingProcess(proc::Error),

    RecoveringBackup(backup::Error, SafePath),
    OpeningFile(file::Error, file::Mode, SafePath),
    MakingBackup(file::Error, SafePath),
    RemovingFile(file::Error, SafePath),
    RemovingBackup(file::Error, SafePath),
    ClearingFile(file::Error)
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
    SerialFormat
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
        use backup::Error::RemovalRefusal;
        use crypt::Error::DecryptingBlock;

        use file::Mode;

        use io::ErrorKind as IoError;

        Some(match self {
            Environment(ParsingArgs(..)) =>
                Advice::ViewingUsage,
            Environment(ResolvingDataPath(..)) =>
                Advice::SpecifyingFile,
            ReadingHeader(e) if e.kind() == IoError::UnexpectedEof =>
                Advice::InvalidFile,
            Crypt(DecryptingBlock) =>
                Advice::IncorrectPassword,
            FileSerial(..) =>
                Advice::InvalidFile,
            InputSerial(..) =>
                Advice::SerialFormat,

            RecoveringBackup(RemovalRefusal, ..) =>
                Advice::MovingBackup,
            RecoveringBackup(..) =>
                Advice::RecoveringBackup,
            OpeningFile(e, ..)
            if e.kind() == IoError::NotFound =>
                Advice::CreatingFile,
            OpeningFile(e, Mode::CreateWrite, ..)
            if e.kind() == IoError::NotFound =>
                Advice::SpecifyingFile,
            RemovingFile(..) =>
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
            ReadingStdin(e) =>
                write!(f, "cannot read stdin: {e}"),
            ReadingHeader(e) =>
                write!(f, "cannot read file header: {e}"),
            WritingHeader(e) =>
                write!(f, "cannot write header to file: {e}"),
            Crypt(e) =>
                write!(f, "{e}"),
            FileSerial(e) =>
                write!(f, "invalid file contents: {e}"),
            InputSerial(e) =>
                write!(f, "invalid input: {e}"),
            FindingRecord(e) =>
                write!(f, "{e}"),
            AddingRecord(e, name) =>
                write!(f, "cannot add '{name}': {e}"),
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
            OpeningFile(e, mode, p) => match mode {
                Mode::CreateWrite =>
                    write!(f, "cannot create '{}': {e}", p.display()),
                _ =>
                    write!(f, "cannot open '{}': {e}", p.display())
            }
            MakingBackup(e, p) =>
                write!(f, "cannot backup '{}': {e}", p.display()),
            RemovingFile(e, p) =>
                write!(f, "cannot recover backup '{}': {e}", p.backup.display()),
            RemovingBackup(e, p) =>
                write!(f, "cannot remove backup '{}': {e}", p.backup.display()),
            ClearingFile(e) =>
                write!(f, "cannot clear pass file: {e}"),
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
            IncorrectPassword =>
                write!(f, "The entered password may be incorrect."),
            SerialFormat =>
                // TODO: point to documentation/examples or something
                write!(f, "find out how `ron` works and utf-8")
        }
    }
}
