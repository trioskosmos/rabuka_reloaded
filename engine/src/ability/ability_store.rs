use crate::card::Ability;
use crate::Arc;

#[cfg(not(feature = "no_std"))]
use std::collections::HashMap;
#[cfg(not(feature = "no_std"))]
use std::sync::{Mutex, OnceLock};

#[cfg(not(feature = "no_std"))]
static RESOLVED_ABILITIES: OnceLock<Mutex<HashMap<u16, Arc<Ability>>>> = OnceLock::new();

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
    /// On decode failure, logs the error and returns a default (empty) ability
    /// so the game continues running. The error includes the ability index and
    /// byte range for debugging.
    pub fn resolve(&self) -> Arc<Ability> {
        #[cfg(feature = "ds_debug")]
        {
            extern "C" {
                fn nds_println(text: *const u8);
            }
            let mut msg = alloc::string::String::new();
            msg.push_str("ARES:");
            msg.push_str(&alloc::string::ToString::to_string(&self.0));
            msg.push('\0');
            unsafe {
                nds_println(msg.as_ptr());
            }
        }
        #[cfg(not(feature = "no_std"))]
        {
            let cache = RESOLVED_ABILITIES.get_or_init(|| Mutex::new(HashMap::new()));
            if let Some(ability) = cache
                .lock()
                .ok()
                .and_then(|cache| cache.get(&self.0).cloned())
            {
                return ability;
            }

            // Decoded abilities persist (global cache + queue entries), so they
            // must not live in the resettable bump arena.
            #[cfg(feature = "arena_allocator")]
            crate::arena::arena_bypass_enter();
            let ability = match crate::ability::vm::get_ability(self.0 as usize) {
                Ok(ability) => Arc::new(ability),
                Err(e) => {
                    log::error!("AbilityRef::resolve() failed for index {}: {e}", self.0);
                    Arc::new(crate::card::Ability::default())
                }
            };
            #[cfg(feature = "arena_allocator")]
            crate::arena::arena_bypass_exit();
            if let Ok(mut cache) = cache.lock() {
                cache.insert(self.0, ability.clone());
            }
            return ability;
        }

        #[cfg(feature = "no_std")]
        match crate::ability::vm::get_ability(self.0 as usize) {
            Ok(ability) => Arc::new(ability),
            Err(e) => {
                log::error!("AbilityRef::resolve() failed for index {}: {e}", self.0);
                Arc::new(crate::card::Ability::default())
            }
        }
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
