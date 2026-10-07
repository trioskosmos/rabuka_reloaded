// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::constants::*;




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
