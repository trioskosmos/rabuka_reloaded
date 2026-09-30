use crate::card::Ability;
use crate::Arc;

/// A lightweight handle to an ability stored in the bytecode blob.
///
/// `AbilityRef` stores a `u16` bytecode index (2 bytes). Call `resolve()`
/// to decode the ability from the compact bytecode on demand.
///
/// # Decoded-ability cache (std builds)
/// Decoding is deterministic per index, so resolved abilities are cached in a
/// per-slot `OnceLock` and repeat resolves are a cheap `Arc::clone`. The cache
/// stays lazy: only abilities actually triggered in a game are ever decoded,
/// keeping the console RAM profile (previously ~30-45 abilities/game decoded).
/// Cache overhead when empty: 936 × `OnceLock<Arc>` ≈ 15 KB.
///
/// # no_std builds
/// No cache (no std sync primitives); decodes on demand as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AbilityRef(pub u16);

#[cfg(not(feature = "no_std"))]
fn decoded_slots() -> &'static Vec<std::sync::OnceLock<Arc<Ability>>> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<OnceLock<Arc<Ability>>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        (0..crate::ability::abilities_gen::NUM_ABILITIES)
            .map(|_| OnceLock::new())
            .collect()
    })
}

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
        #[cfg(not(feature = "no_std"))]
        {
            let slot = &decoded_slots()[self.0 as usize];
            if let Some(a) = slot.get() {
                return Arc::clone(a);
            }
            let decoded = Arc::new(self.decode());
            // Lost race is fine: both sides return a valid ability.
            let _ = slot.set(Arc::clone(&decoded));
            decoded
        }

        #[cfg(feature = "no_std")]
        {
            Arc::new(self.decode())
        }
    }

    /// Borrowed form of [`Self::resolve`] for callers that only READ the ability.
    ///
    /// `resolve()` returns an owned `Arc`, and an `Arc::clone` is an atomic
    /// read-modify-write on a cache line that is shared by every worker thread
    /// (the decode slots are process-global). There are ~50 `.resolve()` call
    /// sites, many inside per-card loops, so the engine was doing on the order
    /// of a few hundred atomic RMWs per action purely to extend a lifetime for a
    /// field read like `use_limit: u8`. This costs nothing.
    ///
    /// Returns `None` only before the slot has been decoded; on `no_std` there
    /// is no slot cache at all, so callers there must keep using `resolve()`.
    #[inline]
    pub fn peek(&self) -> Option<&Ability> {
        #[cfg(not(feature = "no_std"))]
        {
            decoded_slots()[self.0 as usize].get().map(|a| &**a)
        }
        #[cfg(feature = "no_std")]
        {
            None
        }
    }

    fn decode(&self) -> Ability {
        match crate::ability::vm::get_ability(self.0 as usize) {
            Ok(ability) => ability,
            Err(e) => {
                log::error!("AbilityRef::resolve() failed for index {}: {e}", self.0);
                Ability::default()
            }
        }
    }

    /// Legacy alias: returns `self.resolve()`.
    pub fn to_arc(&self) -> Arc<Ability> {
        self.resolve()
    }
}
