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

#[cfg(feature = "no_std")]
pub(crate) use alloc::sync::Arc;
#[cfg(not(feature = "no_std"))]
pub(crate) use std::sync::Arc;

#[cfg(feature = "no_std")]
pub(crate) use alloc::boxed::Box;
#[cfg(not(feature = "no_std"))]
pub(crate) use std::boxed::Box;

#[cfg(feature = "no_std")]
pub(crate) use alloc::collections::VecDeque;
#[cfg(not(feature = "no_std"))]
pub(crate) use std::collections::VecDeque;
