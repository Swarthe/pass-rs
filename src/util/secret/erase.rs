use std::{
    collections::BTreeMap,
    rc::Rc,
    cell::RefCell
};

/// For securely erasing data from memory.
///
/// Allows wrapping the type in a [`Secret`][`super::Secret`].
///
/// The implementation of [`Erase::erase`] must abide by the constraints
/// expressed in its documentation. Thus, when implementing it (and already
/// existing implementations cannot be used) functions like [`set_volatile`] and
/// [`atomic_fence`] should be used.
pub trait Erase {
    /// Erases an object by overwriting its data with zeros.
    ///
    /// This function is intended for pointer types, like [`Vec`],
    /// [`String`]. It does only one thing: erasing the raw data they contain.
    /// However, it does not erase associated metadata, like the length stored
    /// within a [`Vec`]. This is because it very rarely serves a purpose to do
    /// so - the most common use case of this trait is in the context of a
    /// [`Secret`][`super::Secret`], which is constructed by moving an object
    /// into it. Indeed, this is likely to copy and leak the aforementioned
    /// metadata anyway (as is the case of the length stored in a [`Vec`]). If
    /// the metadata must also be kept secret, it must never be copied or moved
    /// and must be manually erased once it is no longer needed.
    ///
    /// `self` remains valid after this operation, although it no longer
    /// contains any useful data.
    ///
    /// Uses volatile writes to ensure that the operation is not compromised by
    /// compiler optimisations. Additionally, atomic fences are used after these
    /// writes to prevent the compiler or CPU from reordering memory operations.
    /// Thus, the data appears erased to all accessors after this function is
    /// applied to it.
    ///
    /// The write operations themselves must be constant-time per byte,
    /// irrespective of its value. This is to mitigate side-channel attacks.
    /// The data is overwritten in place, without being copied or otherwise
    /// leaked. This must be carefully ensured with [`Copy`] types.
    ///
    /// This function should never be inlined to prevent other types of
    /// optimisation-related security breaches. The attribute `inline(never)`
    /// can be used for this purpose.
    fn erase(&mut self);
}

impl<T: Erase> Erase for Vec<T> {
    #[inline(never)]
    fn erase(&mut self) {
        for v in self.as_mut_slice() {
            v.erase();
        }

        atomic_fence();
    }
}

impl Erase for Vec<u8> {
    #[inline(never)]
    fn erase(&mut self) {
        for v in self.as_mut_slice() {
            set_volatile(v, 0);
        }

        atomic_fence();
    }
}

impl Erase for String {
    #[inline(never)]
    fn erase(&mut self) {
        // SAFETY: The `Erase` implementation for `Vec` overwrites its data with
        // zeroes, which are valid utf-8 code points. Thus, `self` is left in a
        // valid state.
        unsafe {
            self.as_mut_vec().erase();
        }
    }
}

impl<'k, V: Erase> Erase for BTreeMap<&'k str, V> {
    #[inline(never)]
    fn erase(&mut self) {
        for v in self.values_mut() {
            v.erase();
        }

        atomic_fence();
    }
}

impl<K, V> Erase for BTreeMap<K, V>
    where
        K: Erase + Ord,
        V: Erase
{
    #[inline(never)]
    fn erase(&mut self) {
        // TODO: creates copy if K or V are Copy, fix if possible using mutable
        // references
        //  inefficient, does comparisons although we can pop any element
        while let Some((mut k, mut v)) = self.pop_last() {
            k.erase();
            v.erase();
        }

        atomic_fence();
    }
}

impl<T: Erase> Erase for Rc<RefCell<T>> {
    #[inline(never)]
    fn erase(&mut self) {
        self.borrow_mut().erase();
    }
}

/// Sets each element of `dest` to `val` such that the operation cannot be
/// "optimised away".
pub fn set_volatile<T: Copy>(dest: &mut T, val: T) {
    use std::ptr::write_volatile;

    // SAFETY: `dest` is a valid, properly aligned and writable pointer (as it
    // is an exclusive reference). Furthermore, the value replaced implements
    // `Copy` and can therefore be safely overwritten.
    unsafe {
        write_volatile(dest, val);
    }
}

/// Prevent memory accesses around a call to this function from being reordered.
pub fn atomic_fence() {
    use std::sync::atomic::Ordering;
    use std::sync::atomic::fence;

    fence(Ordering::SeqCst);
}
