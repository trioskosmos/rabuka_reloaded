use crate::card::Ability;
use crate::Arc;

/// Thin wrapper over `Arc<Ability>` for card ability references.
///
/// # Current state (P1.7 partial)
/// Abilities are decoded eagerly at load time into full `Ability` structs.
/// Each `AbilityRef` holds an `Arc<Ability>` (~3.5KB per ability).
/// 800 abilities × ~3.5KB = ~2.8MB resident RAM.
///
/// # TODO: True lazy loading for 150KB target
/// For console targets (GBA/DS/PSP), `AbilityRef` should store a `u16`
/// bytecode index instead of an `Arc<Ability>`. The ability would be
/// decoded on first access via a shared resolver with bounded LRU cache.
///
/// Architecture:
/// ```text
/// Card.abilities: Vec<AbilityRef>  // AbilityRef = u16 index (2 bytes)
///     ↓ (on first access)
/// AbilityResolver::resolve(idx) -> Arc<Ability>
///     ↓ (cache miss)
/// vm::get_ability(idx) -> decode from BYTECODE blob (136KB in ROM)
///     ↓
/// Cache: HashMap<u16, Arc<Ability>>  // bounded, LRU eviction
/// ```
///
/// This eliminates ~2.8MB of decoded structs from RAM. Only abilities
/// actually triggered in a game are decoded (~30-45 out of 800).
///
/// See MEMORY_REFACTOR.md P1.7 for the full plan.
#[derive(Debug, Clone)]
pub struct AbilityRef(pub Arc<Ability>);

impl AbilityRef {
    pub fn to_arc(&self) -> Arc<Ability> {
        Arc::clone(&self.0)
    }
}

impl core::ops::Deref for AbilityRef {
    type Target = Ability;
    fn deref(&self) -> &Ability {
        &self.0
    }
}

/// No-op — abilities are decoded at load time, no global store needed.
///
/// TODO: For true lazy loading, this should initialize a global AbilityStore
/// (OnceLock<HashMap<u16, Arc<Ability>>>) with capacity for the number of
/// unique abilities. The store would be populated on-demand as abilities
/// are first accessed, with LRU eviction when capacity is exceeded.
pub fn init_ability_store(_count: usize) {}
