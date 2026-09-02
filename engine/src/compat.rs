/// Platform-compatibility re-exports.
/// Maps std types to their no_std equivalents when compiling for PSP.

#[cfg(feature = "no_std")]
mod psp_hash {
    use core::hash::{BuildHasher, Hasher};

    #[derive(Default, Clone, Copy)]
    pub struct PspHasher(u64);

    impl Hasher for PspHasher {
        fn write(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.0 = self.0.wrapping_mul(131).wrapping_add(b as u64);
            }
        }
        fn finish(&self) -> u64 {
            self.0
        }
    }

    impl BuildHasher for PspHasher {
        type Hasher = PspHasher;
        fn build_hasher(&self) -> PspHasher {
            PspHasher(0)
        }
    }

    pub type HashMap<K, V> = hashbrown::HashMap<K, V, PspHasher>;
    pub type HashSet<K> = hashbrown::HashSet<K, PspHasher>;
}

#[cfg(feature = "no_std")]
pub(crate) use psp_hash::{HashMap, HashSet};

#[cfg(not(feature = "no_std"))]
pub(crate) use std::collections::{HashMap, HashSet};

#[cfg(all(feature = "no_std", target_has_atomic = "ptr"))]
pub(crate) use alloc::sync::Arc;
// No-atomic targets (PS1 R3000/MIPS-I, etc.): use Rc. The game is single-
// threaded on consoles and only uses new/clone/make_mut/deref, so the shared
// Rc/Arc API is a drop-in; Rc doesn't require target atomics.
#[cfg(all(feature = "no_std", not(target_has_atomic = "ptr")))]
pub(crate) use alloc::rc::Rc as Arc;
#[cfg(not(feature = "no_std"))]
pub(crate) use std::sync::Arc;

#[cfg(feature = "no_std")]
pub(crate) use alloc::boxed::Box;
#[cfg(not(feature = "no_std"))]
pub(crate) use std::boxed::Box;

#[cfg(all(feature = "no_std", feature = "serde_support"))]
pub(crate) use alloc::collections::BTreeMap;
#[cfg(all(not(feature = "no_std"), feature = "serde_support"))]
pub(crate) use std::collections::BTreeMap;

#[cfg(feature = "no_std")]
pub(crate) use alloc::collections::VecDeque;
#[cfg(not(feature = "no_std"))]
pub(crate) use std::collections::VecDeque;

/// Atomic counter for the defensive runaway-loop guards.
///
/// `core::sync::atomic::AtomicU32` does not exist on targets without 32-bit
/// atomics (GBA ARMv4T, PS1 MIPS-I). Those ports are single-threaded, so a
/// `Cell`-based counter with a relaxed `fetch_add` is a drop-in replacement.
/// (`Ordering` always exists and is used unchanged from `core`.) On targets
/// with atomics (including host builds) it resolves to `core`'s `AtomicU32`.
pub(crate) mod atomic {
    #[cfg(target_has_atomic = "32")]
    pub use core::sync::atomic::AtomicU32;

    #[cfg(not(target_has_atomic = "32"))]
    #[derive(Debug)]
    pub struct AtomicU32(core::cell::Cell<u32>);

    #[cfg(not(target_has_atomic = "32"))]
    impl AtomicU32 {
        pub const fn new(v: u32) -> Self {
            AtomicU32(core::cell::Cell::new(v))
        }
        pub fn fetch_add(&self, v: u32, _order: core::sync::atomic::Ordering) -> u32 {
            let cur = self.0.get();
            self.0.set(cur.wrapping_add(v));
            cur
        }
        pub fn load(&self, _order: core::sync::atomic::Ordering) -> u32 {
            self.0.get()
        }
        pub fn store(&self, v: u32, _order: core::sync::atomic::Ordering) {
            self.0.set(v);
        }
    }

    // Console ports are single-threaded; the counter is only ever touched from
    // the single game thread, so exposing it as Sync is safe.
    #[cfg(not(target_has_atomic = "32"))]
    unsafe impl Sync for AtomicU32 {}

    /// `AtomicUsize` for targets without pointer-width atomics (GBA
    /// ARMv4T has no atomic instructions at all). Same single-threaded
    /// `Cell`-based drop-in as [`AtomicU32`] above.
    #[cfg(target_has_atomic = "ptr")]
    pub use core::sync::atomic::AtomicUsize;

    #[cfg(not(target_has_atomic = "ptr"))]
    pub struct AtomicUsize(core::cell::Cell<usize>);

    #[cfg(not(target_has_atomic = "ptr"))]
    impl AtomicUsize {
        pub const fn new(v: usize) -> Self {
            AtomicUsize(core::cell::Cell::new(v))
        }
        pub fn fetch_add(&self, v: usize, _order: core::sync::atomic::Ordering) -> usize {
            let cur = self.0.get();
            self.0.set(cur.wrapping_add(v));
            cur
        }
        pub fn load(&self, _order: core::sync::atomic::Ordering) -> usize {
            self.0.get()
        }
        #[allow(dead_code)]
        pub fn store(&self, v: usize, _order: core::sync::atomic::Ordering) {
            self.0.set(v);
        }
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    unsafe impl Sync for AtomicUsize {}

    /// `AtomicU8` for targets without 8-bit atomics (GBA ARMv4T). Used by the
    /// bytecode-cache init state machine, which on a single-core console only
    /// ever runs from the game thread.
    #[cfg(target_has_atomic = "8")]
    #[allow(unused_imports)]
    pub use core::sync::atomic::AtomicU8;

    #[cfg(not(target_has_atomic = "8"))]
    #[allow(dead_code)]
    pub struct AtomicU8(core::cell::Cell<u8>);

    #[cfg(not(target_has_atomic = "8"))]
    #[allow(dead_code)]
    impl AtomicU8 {
        pub const fn new(v: u8) -> Self {
            AtomicU8(core::cell::Cell::new(v))
        }
        pub fn load(&self, _order: core::sync::atomic::Ordering) -> u8 {
            self.0.get()
        }
        pub fn store(&self, v: u8, _order: core::sync::atomic::Ordering) {
            self.0.set(v);
        }
        pub fn compare_exchange(
            &self,
            current: u8,
            new: u8,
            _success: core::sync::atomic::Ordering,
            _failure: core::sync::atomic::Ordering,
        ) -> Result<u8, u8> {
            let cur = self.0.get();
            if cur == current {
                self.0.set(new);
                Ok(cur)
            } else {
                Err(cur)
            }
        }
    }

    #[cfg(not(target_has_atomic = "8"))]
    unsafe impl Sync for AtomicU8 {}
}
