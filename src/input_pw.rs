use crate::util::{
    user_io,
    crypt::key
};

use crate::util::{
    crypt::{Key, Header},
    secret::Secret,
};

use crate::err;

use std::fmt;

use std::fmt::Display;

pub enum Error {
    HidingInput(user_io::Error),
    ShowingInput(user_io::Error),
    ReadingInput(user_io::Error),
    GeneratingKey(key::Error)
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn confirm_to_key(
    head: &Header,
    prompt_1: &str,
    prompt_2: &str
) -> Result<Key> {
    loop {
        // Instead of comparing the passwords directly (potentially in variable
        // time, and thus enabling side-channel attacks), we compare their
        // hashes in constant time.
        let (key, key_confirm) = (
            Secret::new(read_to_key(head, prompt_1)?),
            Secret::new(read_to_key(head, prompt_2)?)
        );

        if *key == *key_confirm {
            break Ok(key.into_inner()) ;
        } else {
            err!("passwords do not match");
        }
    }
}

/// XXX: reads pw and generates key with `head`
pub fn read_to_key(head: &Header, prompt: &str) -> Result<Key> {
    let pw = Secret::new(read(prompt)?);

    let result = Key::from_password(pw.as_bytes(), head)
        .map_err(Error::GeneratingKey)?;

    Ok(result)
}

/// hidden input
pub fn read(prompt: &str) -> Result<String> {
    use crate::{input, warn};

    user_io::hide_input()
        .map_err(Error::HidingInput)?;

    match input!("{prompt}") {
        Ok(pw) => {
            // The user-entered newline is hidden; print it ourselves.
            eprintln!();

            let result = Secret::new(pw);

            match user_io::show_input() {
                Ok(()) => Ok(result.into_inner()),
                Err(e) => Err(Error::ShowingInput(e))
            }
        },

        Err(e) => {
            if let Err(e) = user_io::show_input() {
                warn!("{}", Error::ShowingInput(e));
            }

            Err(Error::ReadingInput(e))
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            HidingInput(e)   => write!(f, "cannot hide input: {e}"),
            ShowingInput(e)  => write!(f, "cannot show input: {e}"),
            ReadingInput(e)  => write!(f, "{e}"),
            GeneratingKey(e) => write!(f, "{e}")
        }
    }
}
