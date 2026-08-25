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
}
