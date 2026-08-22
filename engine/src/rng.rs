/// Single xorshift32 PRNG for all platforms.
///
/// xorshift32 uses only native 32-bit ops (fast on 32-bit ARM / 3DS),
/// state is 4 bytes, period is 2^32-1, good enough for game use.
///
/// Platform-specific sync:
///   - Desktop/3DS: `Mutex<u32>` (std)
///   - PSP: `UnsafeCell<u32>` (no_std)
///
/// Platform-specific seeding:
///   - 3DS: `_3ds_system_tick()` from hardware tick counter
///   - PSP: via `seed()` function
///   - Desktop: constant seed (deterministic between runs; bots use their own RNG)

fn xorshift32(state: &mut u32) -> u32 {
    let mut x = *state;
    if x == 0 {
        x = 1;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

// ── instance-based RNG (shared by bot binaries / simulations) ────────────
//
/// PCG-style 64-bit LCG, no_std-safe (no std types, no global state).
///
/// Instance-based so parallel simulation workers keep independent streams.
/// This is the single shared implementation — previously each binary in
/// `src/bin/` carried its own identical copy.
pub struct Lcg(pub u64);

impl Lcg {
    pub const fn new(seed: u64) -> Self {
        Lcg(seed)
    }

    /// Advance and return the raw 64-bit state.
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    /// Uniform-ish value in `0..n` (high bits used; n == 0 yields 0).
    pub fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() >> 33) as usize % n
        }
    }
}

// ── std path (desktop + 3DS) ─────────────────────────────────────────────
#[cfg(not(feature = "no_std"))]
mod inner {
    use std::sync::Mutex;

    #[cfg(feature = "3ds")]
    extern "C" {
        fn _3ds_system_tick() -> u64;
    }

    #[cfg(feature = "dc")]
    extern "C" {
        fn timer_ms_gettime64() -> u64;
    }

    static STATE: Mutex<u32> = Mutex::new(0);

    fn next_u32() -> u32 {
        let mut guard = STATE.lock().unwrap();
        if *guard == 0 {
            *guard = seed_value();
        }
        super::xorshift32(&mut *guard)
    }

    fn seed_value() -> u32 {
        #[cfg(feature = "3ds")]
        {
            let tick = unsafe { _3ds_system_tick() };
            if tick != 0 {
                tick as u32
            } else {
                1
            }
        }
        #[cfg(feature = "dc")]
        {
            let tick = unsafe { timer_ms_gettime64() };
            if tick != 0 {
                tick as u32
            } else {
                1
            }
        }
        #[cfg(not(any(feature = "3ds", feature = "dc")))]
        {
            1
        }
    }

    pub fn seed(seed: u32) {
        let mut guard = STATE.lock().unwrap();
        *guard = seed;
    }

    pub fn shuffle_slice<T>(slice: &mut [T]) {
        let n = slice.len();
        if n <= 1 {
            return;
        }
        for i in (1..n).rev() {
            let j = (next_u32() as usize) % (i + 1);
            slice.swap(i, j);
        }
    }

    pub fn rand_range(max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (next_u32() as usize) % max
    }
}

// ── PSP path (no_std) ────────────────────────────────────────────────────
#[cfg(feature = "no_std")]
mod inner {
    use core::cell::UnsafeCell;

    struct SyncUnsafeCell<T>(UnsafeCell<T>);
    unsafe impl<T> Sync for SyncUnsafeCell<T> {}

    static STATE: SyncUnsafeCell<u32> = SyncUnsafeCell(UnsafeCell::new(0));

    pub fn seed(seed: u32) {
        unsafe {
            *STATE.0.get() = seed;
        }
    }

    fn next_u32() -> u32 {
        let ptr = STATE.0.get();
        unsafe {
            if *ptr == 0 {
                *ptr = 1;
            }
            super::xorshift32(&mut *ptr)
        }
    }

    pub fn shuffle_slice<T>(slice: &mut [T]) {
        let n = slice.len();
        if n <= 1 {
            return;
        }
        for i in (1..n).rev() {
            let j = (next_u32() as usize) % (i + 1);
            slice.swap(i, j);
        }
    }

    pub fn rand_range(max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (next_u32() as usize) % max
    }
}

// Public surface
pub use inner::rand_range;
pub use inner::seed;
pub use inner::shuffle_slice;
