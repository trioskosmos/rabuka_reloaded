use crate::card::Ability;
use crate::Arc;

// ── Default path: AbilityRef is a thin wrapper around Arc<Ability> ──

#[cfg(not(feature = "lazy_abilities"))]
#[derive(Debug, Clone)]
pub struct AbilityRef(pub Arc<Ability>);

#[cfg(not(feature = "lazy_abilities"))]
impl AbilityRef {
    /// Convert to `Arc<Ability>` for ownership transfer (e.g. AbilityQueueEntry).
    pub fn to_arc(&self) -> Arc<Ability> {
        Arc::clone(&self.0)
    }
}

#[cfg(not(feature = "lazy_abilities"))]
impl core::ops::Deref for AbilityRef {
    type Target = Ability;
    fn deref(&self) -> &Ability {
        &self.0
    }
}

// ── Lazy path: AbilityRef is a u16 index into a global AbilityStore ──

#[cfg(feature = "lazy_abilities")]
mod lazy_store {
    use super::Ability;
    use crate::Arc;
    use std::sync::OnceLock;

    /// Global ability store: decodes abilities on demand from the bytecode blob
    /// and caches them in OnceLock slots. Initialized once at startup.
    pub struct AbilityStore {
        abilities: Vec<OnceLock<Arc<Ability>>>,
    }

    impl AbilityStore {
        fn new(count: usize) -> Self {
            let mut abilities = Vec::with_capacity(count);
            for _ in 0..count {
                abilities.push(OnceLock::new());
            }
            AbilityStore { abilities }
        }

        /// Clone an ability as `Arc<Ability>` (for AbilityQueueEntry).
        pub fn clone_arc(&self, idx: usize) -> Arc<Ability> {
            Arc::clone(self.abilities[idx].get().expect("ability not yet decoded"))
        }

        /// Resolve an ability by index, decoding from bytecode on first access.
        /// Returns `&'static Ability` (lifetime tied to the OnceLock slot).
        pub fn get(&self, idx: usize) -> &Ability {
            if let Some(arc) = self.abilities[idx].get() {
                return &**arc;
            }
            // Slow path: decode and cache
            let ability = crate::ability::vm::get_ability(idx)
                .unwrap_or_else(|| panic!("ability index {idx} out of range"));
            let _ = self.abilities[idx].set(Arc::new(ability));
            &**self.abilities[idx].get().unwrap()
        }

        /// Pre-populate abilities (used on default path to eagerly load all).
        pub fn populate(&self, iter: impl Iterator<Item = (usize, Ability)>) {
            for (idx, ability) in iter {
                if idx < self.abilities.len() {
                    let _ = self.abilities[idx].set(Arc::new(ability));
                }
            }
        }
    }

    pub static STORE: OnceLock<AbilityStore> = OnceLock::new();

    /// Initialize the global ability store. Must be called exactly once.
    pub fn init_store(count: usize) -> &'static AbilityStore {
        STORE.get_or_init(|| AbilityStore::new(count))
    }

    #[derive(Debug, Clone, Copy)]
    pub struct AbilityRef(pub u16);

    impl AbilityRef {
        /// Convert to `Arc<Ability>` for ownership transfer (e.g. AbilityQueueEntry).
        pub fn to_arc(self) -> Arc<Ability> {
            STORE
                .get()
                .expect("AbilityStore not initialized")
                .clone_arc(self.0 as usize)
        }

        /// Resolve this reference to the underlying ability.
        /// Returns `&'static Ability` from the global store.
        pub fn resolve(self) -> &'static Ability {
            STORE
                .get()
                .expect("AbilityStore not initialized")
                .get(self.0 as usize)
        }
    }

    impl core::ops::Deref for AbilityRef {
        type Target = Ability;
        fn deref(&self) -> &Ability {
            self.resolve()
        }
    }
}

// ── Public API ──

/// Initialize the ability store. On the default path this is a no-op.
/// On the lazy path, allocates the slot vector (abilities decoded on demand).
pub fn init_ability_store(count: usize) {
    #[cfg(feature = "lazy_abilities")]
    {
        lazy_store::init_store(count);
    }
    let _ = count;
}

#[cfg(feature = "lazy_abilities")]
pub use lazy_store::AbilityRef;
