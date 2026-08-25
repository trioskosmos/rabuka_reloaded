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

/// Clamp an i32 computation into u8 at both ends.
///
/// Single home for the modifier-arithmetic idiom "compute wide, clamp,
/// narrow back" (`(base + mod).max(0) as u8` used to be smeared across the
/// codebase). Unlike the raw idiom this also saturates the TOP end instead
/// of wrapping values above 255 back around — that wrap was never intended.
#[inline]
pub fn saturate_u8(v: i32) -> u8 {
    v.clamp(0, i32::from(u8::MAX)) as u8
}

/// Same contract as [`saturate_u8`] for i16 quantities.
#[inline]
pub fn saturate_i16(v: i32) -> i16 {
    v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// Saturating usize → u8 for card counts, as an extension method so call
/// sites read `.len().u8_count()` instead of `.len() as u8`. Zone sizes are
/// small in practice, but waitrooms/decks CAN exceed 255 in long games and a
/// raw `as u8` silently wraps such counts to garbage (casting cut-downs).
pub trait U8Count {
    fn u8_count(self) -> u8;
}

impl U8Count for usize {
    #[inline]
    fn u8_count(self) -> u8 {
        self.min(usize::from(u8::MAX)) as u8
    }
}
