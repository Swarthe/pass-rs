use std::{
    fmt,
    thread
};

use std::fmt::Display;

use std::{
    time::Duration,
    borrow::Cow
};

pub struct Clipboard(arboard::Clipboard);

#[allow(clippy::enum_variant_names)]
pub enum Error {
    AccessingClipboard(arboard::Error),
    SettingClipboard(arboard::Error),
    ClearingClipboard(arboard::Error)
}

pub type Result<T> = std::result::Result<T, Error>;

impl Clipboard {
    pub fn new() -> Result<Self> {
        Ok(Self(
            arboard::Clipboard::new()
                .map_err(Error::AccessingClipboard)?
        ))
    }

    /// XXX: copies `text` to the primary clipboard and keeps it there for
    /// `time`
    pub fn hold<'t, T>(&mut self, text: T, time: Duration) -> Result<()>
        where
            T: Into<Cow<'t, str>>
    {
        use arboard::SetExtLinux;
        use arboard::LinuxClipboardKind;

        self.0.set()
            .clipboard(LinuxClipboardKind::Primary)
            .text(text)
            .map_err(Error::SettingClipboard)?;

        thread::sleep(time);

        self.0.clear()
            .map_err(Error::ClearingClipboard)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            AccessingClipboard(e) =>
                write!(f, "cannot access clipboard: {e}"),
            SettingClipboard(e) =>
                write!(f, "cannot set clipboard: {e}"),
            ClearingClipboard(e) =>
                write!(f, "cannot clear clipboard: {e}")
        }
    }
}
