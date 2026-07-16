/// Platform-portable shuffle helper.
///
/// On desktop (`thread_rng` is available): delegates to rand's `thread_rng`.
/// On 3DS (`feature = "3ds"`): uses a global xorshift64 seeded from
/// `svcGetSystemTick` via a `no_std`-friendly FFI call.  This avoids the
/// TLS usage inside `rand::thread_rng()` that panics on 3DS.

// ── 3DS path ─────────────────────────────────────────────────────────────────
// Note: AtomicU64 is not available on ARMv6k (3DS CPU).  We use Mutex instead.
#[cfg(feature = "3ds")]
mod inner {
    use std::sync::Mutex;

    extern "C" {
        fn _3ds_system_tick() -> u64;
    }

    static STATE: Mutex<u64> = Mutex::new(0);

    fn next_u64() -> u64 {
        let mut s = *STATE.lock().unwrap();
        if s == 0 {
            // lazy seed from the 3DS hardware tick counter
            s = unsafe { _3ds_system_tick() };
            if s == 0 {
                s = 1;
            }
        }
        // xorshift64
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        *STATE.lock().unwrap() = s;
        s
    }

    pub fn shuffle_slice<T>(slice: &mut [T]) {
        let n = slice.len();
        if n <= 1 {
            return;
        }
        // Fisher-Yates
        for i in (1..n).rev() {
            let j = (next_u64() as usize) % (i + 1);
            slice.swap(i, j);
        }
    }
}

// ── PSP path ────────────────────────────────────────────────────────────────
#[cfg(feature = "psp")]
mod inner {
    use core::cell::UnsafeCell;

    struct SyncUnsafeCell<T>(UnsafeCell<T>);
    unsafe impl<T> Sync for SyncUnsafeCell<T> {}

    static STATE: SyncUnsafeCell<u64> = SyncUnsafeCell(UnsafeCell::new(0));

    pub fn seed(seed: u64) {
        unsafe {
            *STATE.0.get() = seed;
        }
    }

    fn next_u64() -> u64 {
        let s = unsafe { *STATE.0.get() };
        let s = if s == 0 { 1 } else { s };
        let s = s ^ (s << 13);
        let s = s ^ (s >> 7);
        let s = s ^ (s << 17);
        unsafe {
            *STATE.0.get() = s;
        }
        s
    }

    pub fn shuffle_slice<T>(slice: &mut [T]) {
        let n = slice.len();
        if n <= 1 {
            return;
        }
        for i in (1..n).rev() {
            let j = (next_u64() as usize) % (i + 1);
            slice.swap(i, j);
        }
    }
}

// ── desktop path ─────────────────────────────────────────────────────────────
#[cfg(not(any(feature = "3ds", feature = "psp")))]
mod inner {
    use rand::seq::SliceRandom;

    pub fn shuffle_slice<T>(slice: &mut [T]) {
        slice.shuffle(&mut rand::thread_rng());
    }
}

// Public surface
#[cfg(feature = "psp")]
pub use inner::seed;
pub use inner::shuffle_slice;
