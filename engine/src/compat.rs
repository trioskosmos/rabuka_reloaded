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
