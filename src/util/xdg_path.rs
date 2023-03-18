use std::path::PathBuf;

use xdg::BaseDirectoriesError as XdgError;

pub type Result<T> = std::result::Result<T, XdgError>;

pub type Error = XdgError;

// TODO: probably use crate `directories` instead, and rename stuff (modules and
//      help text)
//  perhaps then delete this module

/* XXX
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            ResolvingDataDir(e) =>
                write!(f, "cannot resolve XDG data directory: {e}")
        }
    }
}
*/

/// Returns a path to the `name`-specific XDG data directory.
pub fn data_dir(name: &str) -> Result<PathBuf> {
    use xdg::BaseDirectories;

    Ok(BaseDirectories::with_prefix(name)?
        .get_data_home())
}
