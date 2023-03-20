use super::secret::Secret;

use chacha20poly1305::aead::stream;

use chacha20poly1305::aead::KeyInit;

// Authenticated encryption: allows us to detect if the key is invalid (and
// therefore the password incorrect).
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

/// Cryptographic context, containing necessary cryptographic data.
///
/// Includes the private key, salt and nonce.
pub struct CryptCtx<'k, 'h> {
    key: &'k Key,
    head: &'h Header
}

#[allow(clippy::enum_variant_names)]
pub enum Error {
    EncryptingBlock,
    DecryptingBlock,
    WritingBlock(io::Error),
    ReadingBlock(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl<'k, 'h> CryptCtx<'k, 'h> {
    /// Constructs a new `CryptCtx`.
    pub fn new(key: &'k Key, head: &'h Header) -> Self {
        Self { key, head }
    }

    /// Encrypts the data in `src` and writes it to `dest` block by block.
    ///
    /// Has no effect if `src` is empty. Plain data is never copied.
    pub fn encrypt<D>(&self, mut src: &[u8], mut dest: D) -> Result<()>
        where
            D: Write
    {
        use stream::EncryptorBE32 as Cryptor;

        let cipher = XChaCha20Poly1305::new(self.key.as_slice().into());
        let mut cryptor = Cryptor::from_aead(cipher, self.head.nonce().into());

        while !src.is_empty() {
            let (block, at_last_block) = if src.len() <= PLAIN_BLOCK_LEN {
                (src, true)
            } else {
                (&src[..PLAIN_BLOCK_LEN], false)
            };

            // Use the salt as the AAD.
            let payload = payload_with(block, self.head.salt());

            // Unfortunately, `encrypt_next` allocated a new `Vec` for every
            // block decrypted, which may impact performance. However, a decent
            // allocator should reuse the same memory on every loop iteration,
            // so the performance impact may be minimal.
            let crypted_block = cryptor.encrypt_next(payload)
                .map_err(|_| Error::EncryptingBlock)?;

            dest.write_all(&crypted_block)
                .map_err(Error::WritingBlock)?;

            if at_last_block {
                break;
            } else {
                src = &src[PLAIN_BLOCK_LEN..];    // Advance by one chunk.
            }
        }

        Ok(())
    }

    /// Encrypts the data in `src` block by block and returns it.
    ///
    /// Returns an empty `Vec` if `src` is empty. Clears plain data buffers
    /// using [`Erase::erase`][1] before returning, and clears the decrypted
    /// data if an error occurs.
    ///
    /// [1]: [`super::secret::Erase`]
    pub fn decrypt<S>(&self, mut src: S) -> Result<Vec<u8>>
        where
            S: Read
    {
        use stream::DecryptorBE32 as Cryptor;

        let cipher = XChaCha20Poly1305::new(self.key.as_slice().into());
        let mut cryptor = Cryptor::from_aead(cipher, self.head.nonce().into());

        let mut result = Secret::new(Vec::<u8>::new());
        let mut crypted_block = [0_u8; ENCRYPTED_BLOCK_LEN];

        loop {
            let read_len = src.read(&mut crypted_block)
                .map_err(Error::ReadingBlock)?;

            if read_len == 0 {
                break;
            }

            let (block, at_last_block) = if read_len < crypted_block.len() {
                (&crypted_block[..read_len], true)
            } else {
                (crypted_block.as_slice(), false)
            };

            // As with `encrypt`.
            let payload = payload_with(block, self.head.salt());

            let decrypted_block = Secret::new(
                // As with `encrypt`.
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

fn payload_with<'m, 'a>(msg: &'m [u8], aad: &'a [u8]) -> Payload<'m, 'a> {
    Payload { msg, aad }
}
