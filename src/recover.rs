use crate::{
    user_io,
    file
};

use crate::{
    info,
    confirm
};

use crate::env::PROGNAME;

use crate::util::file::SafePath;

use std::fmt;

use std::fmt::Display;

pub enum Error {
    UserIo(user_io::Error),
    File(file::Error),
    Removal(file::Error),
    RemovalRefusal,
}

pub type Result = std::result::Result<(), Error>;

impl From<user_io::Error> for Error {
    fn from(e: user_io::Error) -> Self {
        Self::UserIo(e)
    }
}

/// Informs the user of `path`'s backup if it exists, and recovers it if the
/// user consents.
pub fn maybe_recover(path: &SafePath) -> Result {
    let is_backed_up = path.is_backed_up()
        .map_err(Error::File)?;

    if is_backed_up {
        info!("Backup found at '{}'", path.backup.display());
        info!("This might mean that '{PROGNAME}' crashed while editing it");

        if confirm!("Recover the backup?")? {
            path.recover()
                .map_err(Error::File)
        } else if confirm!("Remove the backup anyway?")? {
            path.remove_backup()
                .map_err(Error::Removal)
        } else {
            Err(Error::RemovalRefusal)
        }
    } else {
        Ok(())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            UserIo(e)       => write!(f, "{e}"),
            File(e)         => write!(f, "{e}"),
            Removal(e)      => write!(f, "removal failed: {e}"),
            RemovalRefusal  => write!(f, "user refusal")
        }
    }
}
