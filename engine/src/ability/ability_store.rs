use crate::card::Ability;
use crate::Arc;

#[cfg(feature = "no_std")]
use alloc::boxed::Box;
#[cfg(not(feature = "no_std"))]
use std::collections::HashMap;

/// A lightweight handle to an ability stored in the global ability cache.
///
/// `AbilityRef` stores a `u16` bytecode index (2 bytes). The ability is
/// decoded lazily on first access and cached in a global `Arc<Ability>` pool.
///
/// - `Deref` returns `&Ability` from the cached Arc — **zero clone**, just
///   a pointer dereference through the Arc.
/// - `to_arc()` returns `Arc::clone(&cached)` — **cheap refcount bump**,
///   not a struct clone.
///
/// # RAM savings
/// Before: 800 abilities × ~3.5KB = ~2.8MB (all decoded eagerly at load)
/// After: ~30-45 abilities decoded per game × ~3.5KB = ~120KB (lazy decode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AbilityRef(pub u16);

impl AbilityRef {
    pub fn index(idx: u16) -> Self {
        AbilityRef(idx)
    }

    pub fn idx(&self) -> u16 {
        self.0
    }

    /// Clone the cached Arc<Ability> — cheap refcount bump, NOT a struct clone.
    /// The Arc lives forever in the global cache (leaked via Box::leak).
    pub fn to_arc(&self) -> Arc<Ability> {
        Arc::clone(resolve_arc(self.0))
    }
}

impl core::ops::Deref for AbilityRef {
    type Target = Ability;

    /// Returns &Ability from the cached Arc — zero clone, just pointer deref.
    fn deref(&self) -> &Ability {
        &*resolve_arc(self.0)
    }
}

/// Resolve an ability by its bytecode index. Returns `&'static Arc<Ability>`.
/// Decodes from bytecode on cache miss, leaks the Arc into static memory.
#[cfg(not(feature = "no_std"))]
fn resolve_arc(idx: u16) -> &'static Arc<Ability> {
    // Fast path: already cached
    {
        let cache = ability_cache().lock().unwrap();
        if let Some(arc) = cache.get(&idx) {
            return arc;
        }
    }
    // Slow path: decode from bytecode, wrap in Arc, leak into static memory
    let ability = crate::ability::vm::get_ability(idx as usize).unwrap_or_default();
    let arc = Arc::new(ability);
    let leaked: &'static Arc<Ability> = Box::leak(Box::new(arc));
    let mut cache = ability_cache().lock().unwrap();
    cache.entry(idx).or_insert(leaked);
    leaked
}

#[cfg(feature = "no_std")]
fn resolve_arc(idx: u16) -> &'static Arc<Ability> {
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if let Some(arc) = cache.get(&idx) {
            return arc;
        }
        let ability = crate::ability::vm::get_ability(idx as usize).unwrap_or_default();
        let arc = Arc::new(ability);
        let leaked: &'static Arc<Ability> = Box::leak(Box::new(arc));
        cache.entry(idx).or_insert(leaked);
        leaked
    })
}

pub fn cache_size() -> usize {
    #[cfg(not(feature = "no_std"))]
    {
        ability_cache().lock().unwrap().len()
    }
    #[cfg(feature = "no_std")]
    {
        CACHE.with(|c| c.borrow().len())
    }
}

pub fn init_ability_store(_count: usize) {}

// ── Global cache ──

#[cfg(not(feature = "no_std"))]
use std::sync::OnceLock;

#[cfg(not(feature = "no_std"))]
fn ability_cache() -> &'static std::sync::Mutex<HashMap<u16, &'static Arc<Ability>>> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<u16, &'static Arc<Ability>>>> = OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

#[cfg(feature = "no_std")]
thread_local! {
    static CACHE: core::cell::RefCell<crate::HashMap<u16, &'static Arc<Ability>>> =
        core::cell::RefCell::new(crate::HashMap::default());
}
