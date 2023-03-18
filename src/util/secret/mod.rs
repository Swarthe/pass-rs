use std::{
    mem,
    ptr
};

use std::ops::{
    Deref,
    DerefMut
};

pub mod erase;

pub use erase::Erase;

/// Wrapper for securing data in memory, intended for cryptographic secrets.
///
/// The contained data is automatically erased when dropped by being
/// overwritten with zeros. The operation is ensured to not be compromised by
/// optimisations, with additional guarantees specified in the documentation for
/// [`Erase`].
///
/// No other protections are applied to the memory occupied by the data. It is
/// recommended to prevent it from being swapped to disk or exposed in
/// core dumps if this is a necessity. Functions in the
/// [`mem`][crate::util::proc] module can be used for this. Furthermore,
/// constant time algorithms may be used if the data is compared to other data
/// to mitigate the vulnerability to side channel attacks.
// We do not provide `Debug` or `Display` implementations to prevent
// accidental exposures of the contained data or its metadata.
pub struct Secret<T: Erase>(T);

impl<T: Erase> Secret<T> {
    /// Constructs a new `Secret`.
    pub fn new(data: T) -> Self {
        Self(data)
    }

    /// Unwraps this `Secret`, returning the data contained within it.
    ///
    /// Consumes the given `Secret`, thereby disabling the memory protections
    /// applied to the data.
    pub fn into_inner(self) -> T {
        // SAFETY: A single copy of the data is made and the original is
        // "forgotten", so its destructor will only run once. The `Secret`
        // destructor will never run at all, so the data is not modified.
        unsafe {
            let inner = ptr::read(&self.0);

            mem::forget(self);
            inner
        }
    }
}

impl<T: Erase> Drop for Secret<T> {
    /// Erases the data contained within the `Secret`.
    ///
    /// The data's destructor is called afterwards as usual.
	fn drop(&mut self) {
        self.0.erase();
	}
}

impl<T: Erase> Deref for Secret<T> {
	type Target = T;

    /// Returns a reference into the given `Secret`.
    ///
    /// Improper copying of the referred-to data may expose it, by creating a
    /// copies that aren't protected by a `Secret`.
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: Erase> DerefMut for Secret<T> {
    /// Returns a mutable reference into the given `Secret`.
    ///
    /// Improper manipulation of the reference obtained with this function may
    /// deallocate data without erasing it, thereby potentially exposing it. For
    /// example, this may happen if [`Vec::shrink_to`] is called on a [`Vec`]
    /// within the `Secret`. It is the responsibility of the caller that this
    /// does not happen.
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<T: Erase, U> AsRef<U> for Secret<T>
    where
        U: ?Sized,
        <Secret<T> as Deref>::Target: AsRef<U>
{
    fn as_ref(&self) -> &U {
        self.deref().as_ref()
    }
}

impl<T: Erase, U> AsMut<U> for Secret<T>
    where
        U: ?Sized,
        <Secret<T> as Deref>::Target: AsMut<U>
{
    fn as_mut(&mut self) -> &mut U {
        self.deref_mut().as_mut()
    }
}
