use crate::card::Ability;
use crate::Arc;

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
pub fn init_ability_store(_count: usize) {}
