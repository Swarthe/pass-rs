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
    /// Sensitive metadata contained within the object, for example the length
    /// and capacity of a [`Vec`], is also set to zero if it exists. The memory
    /// it uses is then freed. In other words, containers are emptied after
    /// their contents are erased. However, the objects must remain valid after
    /// the operation (despite contain no data).
    ///
    /// Uses volatile writes to ensure that the operation is not compromised by
    /// compiler optimisations. Additionally, atomic fences are used after these
    /// writes to prevent the compiler or CPU from reordering memory operations.
    /// Thus, the data appears erased to all accessors after this function is
    /// applied to it.
    ///
    /// The write operations themselves must be constant-time per byte,
    /// irrespective of its value. This is to mitigate side-channel attacks.
    /// Furthermore, no explicit or implicit reference can be made to the
    /// data being overwritten. It is overwritten in place, without being copied
    /// or otherwise leaked. This must be carefully ensured with `Copy` types.
    ///
    /// This function should never be inlined to prevent other types of
    /// optimisation-related security breaches. The attribute `inline(never)`
    /// can be used for this.
    fn erase(&mut self);
}

impl Erase for u8 {
    #[inline(never)]
    fn erase(&mut self) {
        set_volatile(self, 0);
        atomic_fence();
    }
}

impl<T: Erase> Erase for Vec<T> {
    #[inline(never)]
    fn erase(&mut self) {
        for v in self.as_mut_slice() {
            v.erase();
        }

        self.clear();
        self.shrink_to_fit();

        atomic_fence();
    }
}

impl Erase for String {
    #[inline(never)]
    fn erase(&mut self) {
        // SAFETY: The `Erase` implementation for `Vec` empties it, thus
        // emptying the `String` here. Being empty is a valid state for a
        // `String`.
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

        self.clear();
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
        // TODO: inefficient, does comparisons although we can pop any element
        while let Some((mut k, mut v)) = self.pop_last() {
            k.erase();
            v.erase();
        }

        self.clear();
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
