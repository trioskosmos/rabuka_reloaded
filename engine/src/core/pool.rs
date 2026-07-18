use core::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use super::card::{Condition, EffectKind};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

// ── Fixed-size object pool ─────────────────────────────────────────────
// Pre-allocates N slots of size_of::<T>(). alloc() and free_idx() are O(1)
// with a Mutex-guarded free list. Falls back to Box::new() when the pool
// is exhausted. Used by EkBox and CondBox.

pub struct Pool<T> {
    slots: Box<[UnsafeCell<MaybeUninit<T>>]>,
    free: Mutex<Vec<usize>>,
    next: AtomicUsize,
}

impl<T> Pool<T> {
    pub fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(UnsafeCell::new(MaybeUninit::uninit()));
        }
        Pool {
            slots: slots.into_boxed_slice(),
            free: Mutex::new(Vec::with_capacity(capacity)),
            next: AtomicUsize::new(0),
        }
    }

    pub fn alloc(&self) -> Option<usize> {
        if let Some(idx) = self.free.lock().unwrap().pop() {
            return Some(idx);
        }
        let idx = self.next.fetch_add(1, Ordering::Relaxed);
        if idx < self.slots.len() {
            Some(idx)
        } else {
            None
        }
    }

    pub unsafe fn put(&self, idx: usize, val: T) -> &mut T {
        let ptr = self.slots[idx].get().cast::<T>();
        ptr.write(val);
        &mut *ptr
    }

    pub unsafe fn drop_value(&self, idx: usize) {
        self.slots[idx].get().cast::<T>().drop_in_place();
    }

    pub fn free_idx(&self, idx: usize) {
        self.free.lock().unwrap().push(idx);
    }
}

unsafe impl<T: Send> Send for Pool<T> {}
unsafe impl<T: Sync> Sync for Pool<T> {}

// ── PoolBox macro ──────────────────────────────────────────────────────
// Generates a pool-backed smart pointer that recycles T allocations.
// Falls back to Box::new() when the pool is exhausted.

macro_rules! make_pool_box {
    ($name:ident, $t:ty, $pool_name:ident, $getter:ident, $capacity:expr) => {
        static $pool_name: std::sync::OnceLock<Pool<$t>> = std::sync::OnceLock::new();

        fn $getter() -> &'static Pool<$t> {
            $pool_name.get_or_init(|| Pool::new($capacity))
        }

        #[derive(Debug)]
        pub struct $name {
            slot: Option<usize>,
            heap: Option<Box<$t>>,
        }

        impl $name {
            pub fn new(val: $t) -> Self {
                if let Some(idx) = $getter().alloc() {
                    unsafe {
                        $getter().put(idx, val);
                    }
                    $name {
                        slot: Some(idx),
                        heap: None,
                    }
                } else {
                    $name {
                        slot: None,
                        heap: Some(Box::new(val)),
                    }
                }
            }
        }

        impl Deref for $name {
            type Target = $t;
            fn deref(&self) -> &$t {
                if let Some(idx) = self.slot {
                    unsafe { &*$getter().slots[idx].get().cast::<$t>() }
                } else {
                    self.heap.as_ref().unwrap()
                }
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut $t {
                if let Some(idx) = self.slot {
                    unsafe { &mut *$getter().slots[idx].get().cast::<$t>() }
                } else {
                    self.heap.as_mut().unwrap()
                }
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                $name::new(Deref::deref(self).clone())
            }
        }
        impl Default for $name {
            fn default() -> Self {
                $name::new(Default::default())
            }
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Deref::deref(self) == Deref::deref(other)
            }
        }
        impl Eq for $name {}
        impl AsRef<$t> for $name {
            fn as_ref(&self) -> &$t {
                Deref::deref(self)
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                if let Some(idx) = self.slot {
                    unsafe {
                        $getter().drop_value(idx);
                    }
                    $getter().free_idx(idx);
                }
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                Deref::deref(self).serialize(s)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                <$t>::deserialize(d).map($name::new)
            }
        }
    };
}

make_pool_box!(EkBox, EffectKind, __EK_POOL, ek_get_pool, 128);
make_pool_box!(CondBox, Condition, __COND_POOL, cond_get_pool, 64);
