/// Platform-compatibility re-exports.
/// Maps std types to their no_std equivalents when compiling for PSP.
///
/// The `HashMap`/`HashSet` aliases below are the engine's single most-used
/// containers and are keyed overwhelmingly on small integers — `i16` card
/// ids (a dense 0..2280 space), `HeartColor` (8 values), `u64` keys. The
/// console builds already had a cheap hasher here and the desktop build did
/// not, so the configuration everything is profiled on was paying SipHash-1-3
/// for what is a multiply-and-rotate on this key space.
///
/// FxHash-style mixing rather than the byte-at-a-time multiply-131 the
/// consoles used, because this is a hashbrown SwissTable and it wants a
/// well-distributed final mix rather than a weak fold. The seed is a fixed
/// constant, not a random one, so iteration order stays reproducible across
/// runs; it is simply not the same order std produced, which is why the
/// determinism tests and the full suite gate this change.
///
/// MEASUREMENT NOTE: do not re-A/B this on wall-clock alone. On this box
/// `sim_bench --policy random` reads 452 gps when the machine is quiet and
/// 188-280 gps on identical binaries under load, so any single before/after
/// pair is meaningless. Use allocation counts, or the profiling build's
/// per-path Total ms. See docs/PERF_INCREMENTALIZATION_PLAN.md §1.
mod fast_hash {
    use core::hash::{BuildHasher, Hasher};

    const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

    #[derive(Default, Clone, Copy)]
    pub struct FastHasher(u64);

    impl FastHasher {
        #[inline]
        fn add(&mut self, word: u64) {
            self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
        }
    }

    impl Hasher for FastHasher {
        #[inline]
        fn write(&mut self, bytes: &[u8]) {
            let mut chunks = bytes.chunks_exact(8);
            for c in &mut chunks {
                self.add(u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]));
            }
            let mut tail = 0u64;
            for (i, &b) in chunks.remainder().iter().enumerate() {
                tail |= (b as u64) << (i * 8);
            }
            self.add(tail);
        }

        #[inline]
        fn write_u8(&mut self, i: u8) {
            self.add(i as u64);
        }
        #[inline]
        fn write_u16(&mut self, i: u16) {
            self.add(i as u64);
        }
        #[inline]
        fn write_u32(&mut self, i: u32) {
            self.add(i as u64);
        }
        #[inline]
        fn write_u64(&mut self, i: u64) {
            self.add(i);
        }
        #[inline]
        fn write_usize(&mut self, i: usize) {
            self.add(i as u64);
        }

        #[inline]
        fn finish(&self) -> u64 {
            self.0
        }
    }

    impl BuildHasher for FastHasher {
        type Hasher = FastHasher;
        #[inline]
        fn build_hasher(&self) -> FastHasher {
            FastHasher(0)
        }
    }
}

/// Public because the engine's own public structs expose these types — e.g.
/// every `GameModifiers` field, and `Player::deployed_this_turn` — so
/// out-of-crate callers (integration tests, tools) need a way to name and
/// build the exact type, not `std::collections`' structurally different one.
pub type HashMap<K, V> = hashbrown::HashMap<K, V, fast_hash::FastHasher>;
pub type HashSet<K> = hashbrown::HashSet<K, fast_hash::FastHasher>;

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
