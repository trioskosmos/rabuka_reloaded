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

// ── std path (desktop + 3DS) ─────────────────────────────────────────────
#[cfg(not(feature = "psp"))]
mod inner {
    use std::sync::Mutex;

    #[cfg(feature = "3ds")]
    extern "C" {
        fn _3ds_system_tick() -> u64;
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
        #[cfg(not(feature = "3ds"))]
        {
            1
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

// ── PSP path (no_std) ────────────────────────────────────────────────────
#[cfg(feature = "psp")]
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
#[cfg(feature = "psp")]
pub use inner::seed;
pub use inner::shuffle_slice;
