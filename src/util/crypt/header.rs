use std::io::{
    Read,
    Write
};

/// XXX: [Encryption diagram][1]
///  public encryption metadata
///  large struct, be careful with passing between functions excessively
///
///  [1]: https://docs.rs/aead/latest/aead/stream/index.html
pub struct Header {
    /// The password salt and associated data for AEAD (encryption).
    salt: [u8; SALT_LEN],
    /// The nonce used for AEAD.
    nonce: [u8; NONCE_LEN]
}

pub type Error = std::io::Error;

pub type Result<T> = std::result::Result<T, Error>;

impl Header {
    pub fn generate() -> Self {
        Self {
            salt: rand_bytes(),
            nonce: rand_bytes()
        }
    }

    pub fn salt(&self) -> &[u8] {
        &self.salt
    }

    pub fn nonce(&self) -> &[u8] {
        &self.nonce
    }

    /// XXX: reads `SALT_LEN + NONCE_LEN`
    #[inline(always)]       // The returned struct is very large.
    pub fn read_from<R: Read>(mut src: R) -> Result<Self> {
        let mut salt = [0_u8; SALT_LEN];

        src.read_exact(&mut salt)?;

        let mut nonce = [0_u8; NONCE_LEN];

        src.read_exact(&mut nonce)?;

        Ok(Self { salt, nonce })
    }

    /// XXX: writes everything or fails
    ///  writes `SALT_LEN + NONCE_LEN`
    pub fn write_to<W: Write>(&self, mut dest: W) -> Result<()> {
        dest.write_all(&self.salt)?;
        dest.write_all(&self.nonce)?;

        Ok(())
    }
}

/// The recommended salt length in bytes for `Argon2`, according to [`argon2`]
/// documentation.
const SALT_LEN: usize = 16;

/// The recommended length in bytes for a randomly generated
/// [`XChaCha20Poly1305`][1] nonce, according to [`chacha20poly1305`]
/// documentation.
///
/// The standard length is 24 bytes, but [5 bytes are used by
/// `XChaCha20Poly1305`][1] for a counter and "last block" flag, so we only need
/// 24 - 5 bytes of random data.
///
/// [1]: chacha20poly1305::aead::stream::StreamBE32
const NONCE_LEN: usize = 19;

/// XXX: cryptographically secure
#[inline(always)]       // Copying large arrays is inefficient.
fn rand_bytes<const N: usize>() -> [u8; N] {
    use rand::RngCore;
    use rand::rngs::OsRng;

    let mut result = [0_u8; N];

    // `OsRng` implements `CryptoRng` so it is cryptographically secure.
    OsRng.fill_bytes(&mut result);
    result
}
