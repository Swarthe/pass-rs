use super::secret::Secret;

use chacha20poly1305::aead::stream;

use chacha20poly1305::aead::KeyInit;

// XXX: this is authenticated encryption
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::Payload
};

use std::{
    fmt,
    io
};

use std::io::{
    Read,
    Write
};

use std::fmt::Display;

pub mod header;
pub mod key;

pub use header::Header;
pub use key::Key;

/// The length in bytes of a block of data to encrypt at a time with stream
/// encryption. This is approximately equivalent to the total amount of memory
/// used by [`CryptCtx::encrypt`].
pub const PLAIN_BLOCK_LEN: usize = 0x1000;

/// The length in bytes of a block of date decryptable into a plain data block
/// with stream decryption. This is the length of a plain data block added to
/// the length of the [Mac tag](https://en.wikipedia.org/wiki/VMAC). It
/// represents the approximate minimum amount of memory used by
/// [`CryptCtx::decrypt`].
pub const ENCRYPTED_BLOCK_LEN: usize = PLAIN_BLOCK_LEN + 0x10;

/// XXX: cryptographic context, containing necessary cryptographic data
/// including the private key
pub struct CryptCtx<'k, 'm> {
    key: &'k Key,
    head: &'m Header
}

#[allow(clippy::enum_variant_names)]
pub enum Error {
    EncryptingBlock,
    DecryptingBlock,
    WritingBlock(io::Error),
    ReadingBlock(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl<'k, 'm> CryptCtx<'k, 'm> {
    pub fn new(key: &'k Key, head: &'m Header) -> Self {
        Self { key, head }
    }

    /// XXX: passing `BufWriter` with a big buffer may be a good idea as long as
    ///  the buffer is cleared afterwards (it could contain plaintext)
    /// XXX:  Uses the salt as associated data
    /// if data is empty, does nothing
    pub fn encrypt<D>(&self, mut src: &[u8], mut dest: D) -> Result<()>
        where
            D: Write
    {
        use stream::EncryptorBE32 as Cryptor;

        let cipher = XChaCha20Poly1305::new(self.key.as_slice().into());
        let mut cryptor = Cryptor::from_aead(cipher, self.head.nonce().into());

        // TODO: use in_place methods
        loop {
            if src.is_empty() {
                break;
            }

            let (block, at_last_block) = if src.len() <= PLAIN_BLOCK_LEN {
                (src, true)
            } else {
                (&src[..PLAIN_BLOCK_LEN], false)
            };

            // Use the salt as the AAD.
            let payload = payload_from(block, self.head.salt());

            let encrypted_block = cryptor.encrypt_next(payload)
                .map_err(|_| Error::EncryptingBlock)?;

            dest.write_all(&encrypted_block)
                .map_err(Error::WritingBlock)?;

            if at_last_block {
                break;
            } else {
                src = &src[PLAIN_BLOCK_LEN..];    // Advance by one chunk.
            }
        }

        Ok(())
    }

    /// XXX: zeroes encryption buffers after use, and zeroes decrypted data (if
    ///      any) on error
    ///
    ///   Uses the salt as associated data
    ///
    ///   if `data` is not in memory, like a File, use `BufReader` with a big
    ///   buffer to increase performance (many reads are made sequentially)
    ///
    ///   works on `data` of any size, even if it doesnt fit in RAM
    ///
    /// if data is empty, returns empty
    pub fn decrypt<S>(&self, mut src: S) -> Result<Vec<u8>>
        where
            S: Read
    {
        use stream::DecryptorBE32 as Cryptor;

        let cipher = XChaCha20Poly1305::new(self.key.as_slice().into());
        let mut cryptor = Cryptor::from_aead(cipher, self.head.nonce().into());

        let mut result = Secret::new(Vec::<u8>::new());
        let mut buffer = [0_u8; ENCRYPTED_BLOCK_LEN];

        // TODO: use in_place methods
        loop {
            let read_len = src.read(&mut buffer)
                .map_err(Error::ReadingBlock)?;

            if read_len == 0 {
                break;
            }

            let (block, at_last_block) = if read_len < buffer.len() {
                (&buffer[..read_len], true)
            } else {
                (buffer.as_slice(), false)
            };

            // Use the salt as the AAD.
            let payload = payload_from(block, self.head.salt());

            let decrypted_block = Secret::new(
                cryptor.decrypt_next(payload)
                    .map_err(|_| Error::DecryptingBlock)?
            );

            result.extend_from_slice(&decrypted_block);

            if at_last_block {
                break;
            }
        }

        Ok(result.into_inner())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            EncryptingBlock => write!(f, "cannot encrypt block"),
            DecryptingBlock => write!(f, "cannot decrypt block"),
            WritingBlock(e) => write!(f, "cannot write block: {e}"),
            ReadingBlock(e) => write!(f, "cannot read block: {e}"),
        }
    }
}

fn payload_from<'m, 'a>(msg: &'m [u8], aad: &'a [u8]) -> Payload<'m, 'a> {
    Payload { msg, aad }
}
