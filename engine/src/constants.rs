// Game constants extracted from magic numbers
// These constants represent game rules and structural limits

/// Number of stage positions (Left Side, Center, Right Side)
pub const STAGE_SIZE: usize = 3;

/// Value used to indicate an empty stage slot
pub const EMPTY_SLOT: i16 = -1;

/// Maximum number of energy cards that can be placed in energy zone
pub const MAX_ENERGY_CARDS: usize = 20;

/// Maximum number of cards that can be set in live card zone
pub const MAX_LIVE_CARDS: usize = 3;

/// Default maximum size for game state history (for loop detection)
pub const DEFAULT_HISTORY_SIZE: usize = 100;

/// Maximum size for undo/redo history
// pub const MAX_UNDO_REDO_HISTORY: usize = 50; // Currently unused

/// Initial number of cards drawn (Rule 6.2.1.5)
pub const INITIAL_DRAW_COUNT: usize = 6;

/// Victory condition: number of cards in success live card zone to win
pub const VICTORY_CARD_COUNT: usize = 3;
