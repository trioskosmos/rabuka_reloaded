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

/// Effective value of a printed u8 stat plus its modifier, saturating to u8.
///
/// ONE home for the `printed + modifier` chain that condition evaluation and
/// cost math spell out (comparison.rs, predicates.rs, condition/card.rs).
///
/// The modifier arrives as `i16` because that is what `ModifierEntry` stores
/// and what every `mods.get_*_modifier` accessor returns; this function is the
/// single place that widens for the addition and narrows the result. Callers
/// pass the modifier straight through, with no cast on either side.
#[inline]
pub fn effective_stat(printed: u8, modifier: i16) -> u8 {
    saturate_u8(i32::from(printed) + i32::from(modifier))
}

/// The sibling of [`effective_stat`] for the OTHER case: a figure that already
/// has an absolute set-override in place of its printed value.
///
/// Kept separate rather than folded into `effective_stat` because the two
/// inputs are not the same kind of number. A printed stat is a `u8` — it came
/// off the card. A set override is a modifier `i16`: 「ブレードの値が5になる」
/// stores whatever the ability said, and it stacks additively on top like any
/// other modifier. One function taking `impl Into<i32>` for both would compile
/// at every call site and quietly invite passing a set value where a printed
/// one belongs, which is a rule error, not a type error.
#[inline]
pub fn overridden_stat(set: i16, modifier: i16) -> u8 {
    saturate_u8(i32::from(set) + i32::from(modifier))
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
pub fn floored_cost(base: u8, modifier: i16) -> u8 {
    saturate_u8(i32::from(base) + i32::from(modifier)).max(1)
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
    /// Narrow to the width a render-only badge actually needs.
    ///
    /// Added for the display layer, where a projected modifier is drawn as a
    /// figure and so does not need the engine's `i16`. It CLAMPS at both
    /// ends. That distinction is the whole point of the method: `x as i8`
    /// would wrap a value past 127 into a negative number and render, say,
    /// "-129 Blade" — a wrong sign, not a wrong magnitude. Clamping degrades
    /// to "127", which is still a truthful badge.
    fn i8_count(self) -> i8;
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
    fn i8_count(self) -> i8 {
        // A `usize` has no sign, so the lower clamp is a no-op and only the
        // upper one can bite.
        i8::try_from(self.min(usize::from(u8::MAX))).unwrap_or(i8::MAX)
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
    fn i8_count(self) -> i8 {
        i8::try_from(self.clamp(i32::from(i8::MIN), i32::from(i8::MAX))).unwrap()
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
    fn i8_count(self) -> i8 {
        self.clamp(i16::from(i8::MIN), i16::from(i8::MAX)) as i8
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
    fn i8_count(self) -> i8 {
        // A `u8` spans 0..=255 but an `i8` only reaches 127, so this DOES have
        // to clamp at the top — values above 127 saturate rather than wrapping
        // to a negative. There is no `From<u8> for i8` precisely because that
        // conversion is not always valid.
        i8::try_from(self.min(u8::try_from(i8::MAX).unwrap())).unwrap_or(i8::MAX)
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
    fn i8_count(self) -> i8 {
        i8::try_from(self.clamp(i64::from(i8::MIN), i64::from(i8::MAX))).unwrap()
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
    fn i8_count(self) -> i8 {
        i8::try_from(self.min(u32::from(u8::MAX))).unwrap_or(i8::MAX)
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
        // `u64` is the widest integer type, so this conversion is a no-op. It
        // stays spelled through the trait so every count conversion in the
        // crate reads the same rather than leaving bare `as` casts around.
        self
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
    fn i8_count(self) -> i8 {
        i8::try_from(self.min(u64::from(u8::MAX))).unwrap_or(i8::MAX)
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
    fn i8_count(self) -> i8 {
        if self.is_nan() {
            return 0;
        }
        self.clamp(i8::MIN as f64, i8::MAX as f64) as i8
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `i8_count` is the narrowing boundary between the engine's `i16`
    /// modifier tables and the `i8` figures the display renders. Its whole
    /// contract is "clamp, never wrap", and the sign is the part a test has
    /// to pin: a wrapped value does not merely lose magnitude, it becomes the
    /// OPPOSITE sign, so a large blade bonus would render as a penalty.
    #[test]
    fn i8_count_clamps_instead_of_wrapping() {
        // In range: exact, and symmetric.
        assert_eq!(0i16.i8_count(), 0);
        assert_eq!(17i16.i8_count(), 17);
        assert_eq!(-17i16.i8_count(), -17);
        assert_eq!(i16::from(i8::MAX).i8_count(), i8::MAX);
        assert_eq!(i16::from(i8::MIN).i8_count(), i8::MIN);

        // Out of range: saturates at the bound, keeping the sign.
        assert_eq!(i16::from(i8::MAX).saturating_add(1).i8_count(), i8::MAX);
        assert_eq!(i16::from(i8::MIN).saturating_sub(1).i8_count(), i8::MIN);
        assert_eq!(i16::MAX.i8_count(), i8::MAX);
        assert_eq!(i16::MIN.i8_count(), i8::MIN);

        // The specific failure this exists to prevent. A truncating cast here
        // would yield -49, i.e. a +200 bonus shown as "-49 Blade".
        assert_eq!(200i16.i8_count(), i8::MAX);
        assert_eq!((-200i16).i8_count(), i8::MIN);
    }

    /// The unsigned impls share that contract, and the `u8` one is the
    /// non-obvious case: a `u8` always FITS in an `i8` in the reader's head,
    /// but only up to 127. There is deliberately no `From<u8> for i8`.
    #[test]
    fn i8_count_clamps_for_unsigned_sources() {
        assert_eq!(0u8.i8_count(), 0);
        assert_eq!(127u8.i8_count(), 127);
        assert_eq!(200u8.i8_count(), i8::MAX);
        assert_eq!(255u8.i8_count(), i8::MAX);
        assert_eq!(u32::MAX.i8_count(), i8::MAX);
        assert_eq!(u64::MAX.i8_count(), i8::MAX);
        assert_eq!(usize::MAX.i8_count(), i8::MAX);
    }

    /// `f64` is the one source that can be NaN, and NaN has no place in the
    /// range at all — every comparison against it is false, so an unclamped
    /// cast would reach the integer conversion as 0 on some paths and as an
    /// arbitrary value on others.
    #[test]
    fn i8_count_treats_nan_as_zero() {
        assert_eq!(f64::NAN.i8_count(), 0);
        assert_eq!(f64::INFINITY.i8_count(), i8::MAX);
        assert_eq!(f64::NEG_INFINITY.i8_count(), i8::MIN);
    }
}
