use crate::card::Ability;
use crate::Arc;

/// A lightweight handle to an ability stored in the bytecode blob.
///
/// `AbilityRef` stores a `u16` bytecode index (2 bytes). Call `resolve()`
/// to decode the ability from the compact bytecode on demand. The returned
/// `Arc<Ability>` is owned by the caller and dropped when no longer needed.
/// No global cache, no leaked memory.
///
/// # RAM savings
/// Before (lazy decode cache): ~120 KB of leaked Arc<Ability> per game
/// After: zero leaked memory — abilities decoded on demand and dropped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AbilityRef(pub u16);

impl AbilityRef {
    pub fn index(idx: u16) -> Self {
        AbilityRef(idx)
    }

    pub fn idx(&self) -> u16 {
        self.0
    }

    /// Decode the ability from bytecode and return an owned `Arc<Ability>`.
    /// Unlike the old Deref-based lazy cache, this decodes fresh each time
    /// and the caller drops the Arc when done — no memory leak.
    pub fn resolve(&self) -> Arc<Ability> {
        Arc::new(crate::ability::vm::get_ability(self.0 as usize).unwrap_or_default())
    }

    /// Legacy alias: returns `self.resolve()`.
    pub fn to_arc(&self) -> Arc<Ability> {
        self.resolve()
    }
}

pub fn cache_size() -> usize {
    0
}

pub fn init_ability_store(_count: usize) {}
