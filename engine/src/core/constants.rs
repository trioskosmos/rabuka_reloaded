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
    u8::try_from(v.clamp(0, i32::from(u8::MAX))).unwrap()
}

/// Same contract as [`saturate_u8`] for i16 quantities.
#[inline]
pub fn saturate_i16(v: i32) -> i16 {
    i16::try_from(v.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap()
}

/// Narrow a `usize` count to `u8`, saturating at the top.
///
/// The `len() as u8` / `count as u8` idiom is everywhere in this crate: zone
/// sizes, hand sizes, deck sizes, amounts a card can move. None of them can
/// actually reach 256 in a legal game, but the cast is written anyway, and
/// `as` wraps rather than saturates. These four helpers are the single home
/// for that narrowing: they take the count at its natural `usize` width, so
/// the entry cast is inside the helper where it is clamped, not spread across
/// 44 files. The saturating top is the only observable difference from the
/// old `as` on legal input, where every one of these values is far below 255.
#[inline]
pub fn count_u8(v: usize) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

/// Narrow a `usize` count to `u16`, saturating at the top.
#[inline]
pub fn count_u16(v: usize) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

/// Narrow a `usize` count to `u32`, saturating at the top.
#[inline]
pub fn count_u32(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Widen an `i16` identifier to `u32` for hash keys, flooring at zero.
///
/// Card and ability indices are non-negative by construction; the old
/// `as u32` on a negative value produced a huge key instead, which then
/// collided with nothing and silently failed to match its own entry.
#[inline]
pub fn id_u32(v: i16) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

/// Widen a `usize` count to `i32`, saturating at the top.
#[inline]
pub fn count_i32(v: usize) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// Narrow a `usize` count to `i16`, saturating at the top.
#[inline]
pub fn count_i16(v: usize) -> i16 {
    i16::try_from(v).unwrap_or(i16::MAX)
}

/// Widen an `i32` count to `usize`, flooring at zero.
///
/// The `count as usize` counterpart to the helpers above: a negative count in
/// the data means "none", and `-1 as usize` is a huge value rather than zero.
#[inline]
pub fn count_usize(v: i32) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// Widen an `i16` count to `usize`, flooring at zero.
#[inline]
pub fn count_usize_i16(v: i16) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// Widen an `i64` count to `usize`, flooring at zero.
#[inline]
pub fn count_usize_i64(v: i64) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// Widen a `u32` count to `usize`. Exact on every target this crate builds
/// for, but goes through `try_from` so the intent is checked rather than
/// assumed.
#[inline]
pub fn count_usize_u32(v: u32) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

/// Narrow a score-space `f64` to `i32` by truncation, saturating out-of-range
/// and non-finite inputs.
///
/// Scores are accumulated in f64 and compared, but end up in i32 sort keys
/// and JSON payloads. The clamp is the fix for `cast_possible_truncation`
/// here: without it a score above `i32::MAX` or a NaN wraps to an arbitrary
/// value, which then wins or loses a comparison it should not. Inside this
/// helper the `as` is provably in range, which is what the allow records.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
#[inline]
pub fn score_to_i32(v: f64) -> i32 {
    if v.is_nan() {
        return 0;
    }
    v.clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

/// Same clamping contract as [`score_to_i32`], but round-to-nearest first.
///
/// Distinct from [`score_to_i32`] on purpose: the `as i32` sites are two
/// different idioms in this crate, `(x * f).round() as i32` and a bare
/// `x as i32`, and they do not agree on half-values. Pick the one that
/// matches the call site.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
#[inline]
pub fn score_to_i32_rounded(v: f64) -> i32 {
    if v.is_nan() {
        return 0;
    }
    v.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

/// Effective value of a printed u8 stat plus a constant modifier, kept wide.
///
/// ONE home for the `printed as i32 + get_*_modifier` chain smeared across
/// condition evaluation and cost math (comparison.rs, predicates.rs,
/// condition/card.rs). Callers narrow with [`saturate_u8`]/[`saturate_i16`]
/// or keep the i32 for sums — either way the entry cast lives here, once.
#[inline]
pub fn effective_stat(printed: u8, modifier: i32) -> i32 {
    printed as i32 + modifier
}

/// Effective playable cost with floor-at-1: `(base + modifier).max(1)`,
/// saturated to u8.
///
/// Canonical home for the baton-touch price formula: the payment itself
/// (core/player.rs), the estimates (game_setup.rs) and the double-baton
/// path (turn/phases.rs) previously spelled it three different ways
/// (`as u8` wrapping, `try_from(..).unwrap_or(0)`). Values above 255
/// saturate instead of wrapping — unreachable in this game (costs ≤ ~15,
/// modifiers ±single digits) and never intended.
#[inline]
pub fn floored_cost(base: u8, modifier: i32) -> u8 {
    u8::try_from((base as i32 + modifier).max(1)).unwrap_or(u8::MAX)
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
        u8::try_from(self.min(usize::from(u8::MAX))).unwrap()
    }
}
