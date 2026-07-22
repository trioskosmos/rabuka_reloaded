pub mod card;
pub mod card_loader;
pub mod constants;
pub mod game_modifiers;
pub mod game_state;
pub mod player;
#[cfg(not(feature = "no_std"))]
pub mod pool;
pub mod types;
pub mod zones;

/// Compact card binary format for RAM-constrained targets (GBA, DS).
#[cfg(feature = "compact_card_data")]
pub mod card_binary;
#[cfg(feature = "compact_card_data")]
pub mod cards_gen;
