// Game constants extracted from magic numbers
// These constants represent game rules and structural limits

/// Number of stage positions (Left Side, Center, Right Side)
pub const STAGE_SIZE: usize = 3;

/// Value used to indicate an empty stage slot
pub const EMPTY_SLOT: i16 = -1;

/// Maximum number of energy cards that can be placed in energy zone
pub const MAX_ENERGY_CARDS: usize = 12;

/// Maximum number of cards that can be set in live card zone
pub const MAX_LIVE_CARDS: usize = 3;

/// Victory condition: number of cards in success live card zone to win
pub const VICTORY_CARD_COUNT: usize = 3;
