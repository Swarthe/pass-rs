use crate::util::secret::Erase;

use super::header::Header;

use std::fmt;

use std::fmt::Display;

/// A private encryption key.
///
/// Should be secured and erased from memory after use, for example by wrapping
/// it in a [`Secret`][`crate::util::secret::Secret`].
#[derive(PartialEq, Eq)]
pub struct Key(Vec<u8>);

pub enum Error {
    HashingPassword(argon2::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Key {
    /// The length in bytes of an encryption key, according to
    /// [`chacha20poly1305`] documentation.
    pub const LEN: usize = 32;

    /// Returns a `Key` hashed from `pw`.
    ///
    /// Uses the salt in `head`.
    pub fn from_password<P>(pw: P, head: &Header) -> Result<Self>
        where
            P: AsRef<[u8]>
    {
        use argon2::{Config, Variant, Version};

        let hash_conf = Config {
            variant: Variant::Argon2id,
            version: Version::Version13,
            hash_length: Self::LEN as u32,
            mem_cost: 0x800,    // The default causes a crash on debug.
            ..Default::default()
        };

        let result = argon2::hash_raw(
            pw.as_ref(),
            head.salt(),
            &hash_conf
        ).map_err(Error::HashingPassword)?;

        Ok(Self(result))
    }

    /// Returns a reference to the contained key.
    ///
    /// The returned slice is guaranteed to be `Self::LEN` bytes long.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Erase for Key {
    #[inline(never)]
    fn erase(&mut self) {
        self.0.erase();
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            HashingPassword(e) => write!(f, "cannot hash password: {e}"),
        }
    }
}
