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

/// Saturating narrowing for the count and index widths this crate uses.
///
/// `x as u8` truncates or wraps, and the wrap is never intended: a deck that
/// grows past 255 cards, or a modifier delta that overflows i16, should
/// clamp, not become a small value that then wins an ordering comparison it
/// should have lost. `usize as i32` has the same problem one octave down.
///
/// Every narrowing in the crate goes through this trait so the clamp lives in
/// one place, and so the direction of each conversion is visible at the call
/// site: `zone.len().u8_count()` says "narrow this count to a byte", where
/// `zone.len() as u8` only says "there was a cast here".
///
/// Only the conversions actually used by the crate are implemented. Unsigned
/// sources saturate at the top; signed sources clamp at both ends for signed
/// targets and floor at zero for unsigned ones, so a negative count means
/// "none" rather than `-1 as usize` (a value near `usize::MAX`).
pub trait CountCast {
    fn u8_count(self) -> u8;
    fn u16_count(self) -> u16;
    fn u32_count(self) -> u32;
    fn u64_count(self) -> u64;
    fn i32_count(self) -> i32;
    fn i16_count(self) -> i16;
    fn usize_count(self) -> usize;
}

impl CountCast for usize {
    #[inline]
    fn u8_count(self) -> u8 {
        u8::try_from(self).unwrap_or(u8::MAX)
    }
    #[inline]
    fn u16_count(self) -> u16 {
        u16::try_from(self).unwrap_or(u16::MAX)
    }
    #[inline]
    fn u32_count(self) -> u32 {
        u32::try_from(self).unwrap_or(u32::MAX)
    }
    #[inline]
    fn u64_count(self) -> u64 {
        u64::try_from(self).unwrap_or(u64::MAX)
    }
    #[inline]
    fn i32_count(self) -> i32 {
        i32::try_from(self).unwrap_or(i32::MAX)
    }
    #[inline]
    fn i16_count(self) -> i16 {
        i16::try_from(self).unwrap_or(i16::MAX)
    }
    #[inline]
    fn usize_count(self) -> usize {
        self
    }
}

impl CountCast for i32 {
    #[inline]
    fn u8_count(self) -> u8 {
        u8::try_from(self.clamp(0, i32::from(u8::MAX))).unwrap()
    }
    #[inline]
    fn u16_count(self) -> u16 {
        u16::try_from(self.clamp(0, i32::from(u16::MAX))).unwrap()
    }
    #[inline]
    fn u32_count(self) -> u32 {
        u32::try_from(self.max(0)).unwrap()
    }
    #[inline]
    fn u64_count(self) -> u64 {
        u64::try_from(self.max(0)).unwrap()
    }
    #[inline]
    fn i32_count(self) -> i32 {
        self
    }
    #[inline]
    fn i16_count(self) -> i16 {
        i16::try_from(self.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap()
    }
    #[inline]
    fn usize_count(self) -> usize {
        usize::try_from(self).unwrap_or(0)
    }
}

impl CountCast for i16 {
    #[inline]
    fn u8_count(self) -> u8 {
        u8::try_from(self.clamp(0, i16::from(u8::MAX))).unwrap()
    }
    #[inline]
    fn u16_count(self) -> u16 {
        u16::try_from(self.max(0)).unwrap()
    }
    #[inline]
    fn u32_count(self) -> u32 {
        u32::try_from(self.max(0)).unwrap()
    }
    #[inline]
    fn u64_count(self) -> u64 {
        u64::try_from(self.max(0)).unwrap()
    }
    #[inline]
    fn i32_count(self) -> i32 {
        i32::from(self)
    }
    #[inline]
    fn i16_count(self) -> i16 {
        self
    }
    #[inline]
    fn usize_count(self) -> usize {
        usize::try_from(self).unwrap_or(0)
    }
}

impl CountCast for u8 {
    #[inline]
    fn u8_count(self) -> u8 {
        self
    }
    #[inline]
    fn u16_count(self) -> u16 {
        u16::from(self)
    }
    #[inline]
    fn u32_count(self) -> u32 {
        u32::from(self)
    }
    #[inline]
    fn u64_count(self) -> u64 {
        u64::from(self)
    }
    #[inline]
    fn i32_count(self) -> i32 {
        i32::from(self)
    }
    #[inline]
    fn i16_count(self) -> i16 {
        i16::from(self)
    }
    #[inline]
    fn usize_count(self) -> usize {
        usize::from(self)
    }
}

impl CountCast for i64 {
    #[inline]
    fn u8_count(self) -> u8 {
        u8::try_from(self.clamp(0, i64::from(u8::MAX))).unwrap()
    }
    #[inline]
    fn u16_count(self) -> u16 {
        u16::try_from(self.clamp(0, i64::from(u16::MAX))).unwrap()
    }
    #[inline]
    fn u32_count(self) -> u32 {
        u32::try_from(self.max(0)).unwrap()
    }
    #[inline]
    fn u64_count(self) -> u64 {
        u64::try_from(self.max(0)).unwrap()
    }
    #[inline]
    fn i32_count(self) -> i32 {
        i32::try_from(self).unwrap_or(if self < 0 { i32::MIN } else { i32::MAX })
    }
    #[inline]
    fn i16_count(self) -> i16 {
        i16::try_from(self.clamp(i64::from(i16::MIN), i64::from(i16::MAX))).unwrap()
    }
    #[inline]
    fn usize_count(self) -> usize {
        usize::try_from(self).unwrap_or(0)
    }
}

/// Compatibility shim for the `U8Count` -> `CountCast` rename.
///
/// Purely additive: the trait gained methods and the old name disappeared,
/// but call sites across the crate still `use crate::core::constants::U8Count`
/// and still call `id_u32`/`count_u32`. Re-exporting the old trait name keeps
/// method resolution working for them, and the two free functions are
/// restored verbatim. Delete once every caller has migrated.
pub use CountCast as U8Count;

/// Widen a `usize` count to `u32`, saturating at the top.
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

impl CountCast for u32 {
    #[inline]
    fn u8_count(self) -> u8 {
        u8::try_from(self).unwrap_or(u8::MAX)
    }
    #[inline]
    fn u16_count(self) -> u16 {
        u16::try_from(self).unwrap_or(u16::MAX)
    }
    #[inline]
    fn u32_count(self) -> u32 {
        self
    }
    #[inline]
    fn u64_count(self) -> u64 {
        u64::from(self)
    }
    #[inline]
    fn i32_count(self) -> i32 {
        i32::try_from(self).unwrap_or(i32::MAX)
    }
    #[inline]
    fn i16_count(self) -> i16 {
        i16::try_from(self).unwrap_or(i16::MAX)
    }
    #[inline]
    fn usize_count(self) -> usize {
        usize::try_from(self).unwrap_or(usize::MAX)
    }
}

impl CountCast for u64 {
    #[inline]
    fn u8_count(self) -> u8 {
        u8::try_from(self).unwrap_or(u8::MAX)
    }
    #[inline]
    fn u16_count(self) -> u16 {
        u16::try_from(self).unwrap_or(u16::MAX)
    }
    #[inline]
    fn u32_count(self) -> u32 {
        u32::try_from(self).unwrap_or(u32::MAX)
    }
    #[inline]
    fn u64_count(self) -> u64 {
        u64::try_from(self).unwrap_or(u64::MAX)
    }
    #[inline]
    fn i32_count(self) -> i32 {
        i32::try_from(self).unwrap_or(i32::MAX)
    }
    #[inline]
    fn i16_count(self) -> i16 {
        i16::try_from(self).unwrap_or(i16::MAX)
    }
    #[inline]
    fn usize_count(self) -> usize {
        usize::try_from(self).unwrap_or(usize::MAX)
    }
}

// The `as` casts below are the f64→int conversions themselves, and every one
// of them is preceded by a clamp that puts the value provably in range, plus
// a NaN guard. That clamp IS the fix for cast_possible_truncation here: an
// unclamped cast of an out-of-range or NaN score wraps to an arbitrary value
// that then wins or loses a comparison it should not. The lints are allowed
// on this impl only, because this is the one place the range is proved.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
impl CountCast for f64 {
    #[inline]
    fn u8_count(self) -> u8 {
        self.clamp(0.0, f64::from(u8::MAX)) as u8
    }
    #[inline]
    fn u16_count(self) -> u16 {
        self.clamp(0.0, f64::from(u16::MAX)) as u16
    }
    #[inline]
    fn u32_count(self) -> u32 {
        if self.is_nan() {
            return 0;
        }
        self.clamp(0.0, f64::from(u32::MAX)) as u32
    }
    #[inline]
    fn u64_count(self) -> u64 {
        if self.is_nan() || self <= 0.0 {
            return 0;
        }
        self.clamp(0.0, u64::MAX as f64) as u64
    }
    #[inline]
    fn i32_count(self) -> i32 {
        if self.is_nan() {
            return 0;
        }
        self.clamp(i32::MIN as f64, i32::MAX as f64) as i32
    }
    #[inline]
    fn i16_count(self) -> i16 {
        if self.is_nan() {
            return 0;
        }
        self.clamp(i16::MIN as f64, i16::MAX as f64) as i16
    }
    #[inline]
    fn usize_count(self) -> usize {
        if self.is_nan() || self <= 0.0 {
            return 0;
        }
        self.clamp(0.0, usize::MAX as f64) as usize
    }
}

/// Saturating f64 → f32. The `as f32` spelling silently turns an
/// out-of-range density into an infinity, which then propagates through the
/// feature vector as a NaN rather than as a saturated 1.0. The `as` below is
/// the conversion itself; the is_finite check is the clamp that makes it
/// safe, which is why the lint is allowed only here.
#[allow(clippy::cast_possible_truncation)]
#[inline]
pub fn f32_count(v: f64) -> f32 {
    let narrowed = v as f32;
    if narrowed.is_finite() {
        narrowed
    } else if v.is_nan() {
        0.0
    } else if v > 0.0 {
        f32::MAX
    } else {
        f32::MIN
    }
}
