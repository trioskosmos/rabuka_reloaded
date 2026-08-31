#[cfg(not(feature = "snes"))]
use super::abilities_gen::{NUM_ABILITIES, OFFSET_DELTAS, STRINGS_OFFSETS, get_string};
#[cfg(all(not(feature = "snes"), not(feature = "gba")))]
use super::abilities_gen::COMPRESSED_BYTECODE;
#[cfg(feature = "gba")]
use super::abilities_gen::BYTECODE;
#[cfg(feature = "snes")]
use super::abilities_gen::{ABILITY_LOCS, NUM_ABILITIES, STRINGS_OFFSETS, bytecode_slice, get_string};
use super::enums::EffectState;
use crate::ability::enums::{ActionType, Zone};
#[cfg_attr(not(feature = "debug_conditions"), allow(unused_imports))]
use crate::card::{
    ek_box_new, Ability, AbilityCost, AbilityEffect, AbilityFilter, AbilityFilterBranch,
    CardProperty, CardState, CardType, ComparisonTarget, ComparisonType, Condition,
    ConditionCardType, DistinctType, DynamicCount, EffectFilter, EffectKind,
    LocationSubChecks, Operator, Operation, PlacementOrder, PositionCharacter, PositionInfo,
    QuotedText,
};
use crate::core::types::ArcStr;

include!("effect_decoder_gen.rs");
include!("condition_decoder_gen.rs");

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};

/// Errors that can occur when decoding an ability from bytecode.
#[derive(Debug, Clone)]
pub enum DecodeError {
    /// Ability index is out of range.
    IndexOutOfRange { idx: usize, max: usize },
    /// Bytecode slice is empty (offset start >= end).
    EmptySlice { idx: usize },
    /// Direct decoder failed.
    DecodeFailed {
        idx: usize,
        byte_range: (usize, usize),
    },
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::IndexOutOfRange { idx, max } => {
                write!(f, "ability index {idx} out of range (max {max})")
            }
            DecodeError::EmptySlice { idx } => {
                write!(f, "ability {idx} has empty bytecode slice")
            }
            DecodeError::DecodeFailed { idx, byte_range } => {
                write!(
                    f,
                    "ability {idx} decode failed (bytes {}..{})",
                    byte_range.0, byte_range.1
                )
            }
        }
    }
}

const TAG_NULL: u8 = 0x00;
const TAG_FALSE: u8 = 0x01;
const TAG_TRUE: u8 = 0x02;
const TAG_I64: u8 = 0x03;
const TAG_F64: u8 = 0x04;
const TAG_STR: u8 = 0x06;
const TAG_ARRAY: u8 = 0x07;
const TAG_OBJECT: u8 = 0x08;
const TAG_OBJECT_VARIANT: u8 = 0x09;

pub fn ability_count() -> usize {
    NUM_ABILITIES
}

// ── Decode-fallback audit (audit item C1) ──
// The bytecode decoder must never silently substitute a default for a value
// it doesn't recognize: that turns an ability into a no-op (or a wrong-op)
// with no signal. Every such site bumps this monotonic counter; the corpus
// oracle test `bytecode_no_silent_decode_fallbacks` pins the total at 0 so a
// new gap fails CI loudly. Repetition across repeated decodes is harmless:
// expected is zero, and nonzero is nonzero.
use crate::compat::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

static DECODE_FALLBACKS: AtomicUsize = AtomicUsize::new(0);

/// Per-ability fallback counters. The set of indices with nonzero counts is
/// the interesting signal: unlike the running total it is stable under
/// concurrent decoding by parallel tests.
pub const DECODE_AUDIT_MAX: usize = 4096;

#[allow(clippy::declare_interior_mutable_const)]
const ATOMIC_ZERO: AtomicUsize = AtomicUsize::new(0);
static DECODE_FALLBACK_ABILITIES: [AtomicUsize; DECODE_AUDIT_MAX] =
    [ATOMIC_ZERO; DECODE_AUDIT_MAX];

/// Record one silent default-substitution during bytecode decoding.
/// `ability` is the ability index when known, `field` names the field being
/// decoded, `value` the unrecognized raw value.
pub fn note_decode_fallback(ability: Option<usize>, field: &str, value: &str) {
    let n = DECODE_FALLBACKS.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(i) = ability {
        if i < DECODE_AUDIT_MAX {
            DECODE_FALLBACK_ABILITIES[i].fetch_add(1, Ordering::Relaxed);
        }
    }
    log::warn!(
        "[decode_audit] fallback #{} (ability {:?}): {} = {:?}",
        n, ability, field, value
    );
}

/// Total silent-fallback substitutions recorded since process start.
pub fn decode_fallback_count() -> usize {
    DECODE_FALLBACKS.load(Ordering::Relaxed)
}

/// Sorted ability indices that recorded at least one silent fallback.
pub fn decode_fallback_abilities() -> Vec<usize> {
    let mut out: Vec<usize> = (0..NUM_ABILITIES.min(DECODE_AUDIT_MAX))
        .filter(|&i| DECODE_FALLBACK_ABILITIES[i].load(Ordering::Relaxed) > 0)
        .collect();
    out.sort_unstable();
    out
}

/// Number of abilities whose compiled slice is empty — these decode to
/// `Ability::default()`. Baseline = the two known `is_null` abilities
/// (PL!HS-PR-010-PR, PL!HS-bp1-019-L) that the parser cannot structure at
/// all; any increase means a new ability lost its effects in compilation.
#[cfg(not(feature = "snes"))]
pub fn count_empty_bytecode_abilities() -> usize {
    OFFSET_DELTAS.iter().filter(|&&d| d == 0).count()
}

#[cfg(feature = "snes")]
pub fn count_empty_bytecode_abilities() -> usize {
    ABILITY_LOCS.iter().filter(|&&(_, _, len)| len == 0).count()
}

/// Decode a single ability from the bytecode blob.
///
/// Returns `Ok(Ability)` on success, or `Err(DecodeError)` with the ability
/// index and byte range so callers can log precise diagnostics.
///
/// # Lazy decoding flow
/// 1. `card_loader` stores only u16 indices (no decode at load time)
/// 2. On first trigger/access, `AbilityRef::resolve()` calls `get_ability(idx)`
/// 3. Decode from bytecode
/// 4. Caller wraps in Arc and drops when done — no memory leak
///
/// # RAM savings
/// Before: All 800 abilities decoded eagerly at load = ~2.8MB
/// After: Only ~30-45 abilities triggered per game decoded = ~120KB
/// Absolute byte offset of `unique_abilities[idx]` within `BYTECODE`.
/// Rebuilt from `OFFSET_DELTAS` (per-ability slice lengths) as a running
/// prefix sum. Ability indexes are small and lookups are rare (decode is
/// lazy/on-demand), so the linear walk is negligible.
#[cfg(not(feature = "snes"))]
fn offset_of(idx: usize) -> usize {
    OFFSET_DELTAS[..idx].iter().map(|&d| d as usize).sum()
}

const BYTECODE_MAGIC: &[u8; 4] = b"RBKA";
const BYTECODE_VERSION: u32 = 1;

#[cfg(not(feature = "gba"))]
fn strip_bytecode_header(data: Vec<u8>) -> Vec<u8> {
    if data.len() >= 8 && &data[0..4] == BYTECODE_MAGIC {
        let ver = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert!(ver == BYTECODE_VERSION, "bytecode version mismatch: got {ver}, expected {BYTECODE_VERSION} — stale blob paired with fresh code (C3). Regenerate via `python cards/compile_abilities.py`");
        log::debug!("[BYTECODE] magic RBKA version {ver} OK, stripping header");
        return data[8..].to_vec();
    }
    log::debug!("[BYTECODE] no magic header (old blob) — accepting without version check (C3 transition)");
    data
}

#[cfg(feature = "gba")]
fn get_decompressed_bytecode() -> &'static [u8] {
    // GBA: 92KB decompressed bytecode lives in ROM via include_bytes!("abilities.bin"),
    // zero heap allocation. Mirrors ps1 external_card_data pattern where large blobs
    // are XIP. Avoids lazy Vec::with_capacity(92000) after mulligan fragments heap.
    if BYTECODE.len() >= 8 && &BYTECODE[0..4] == BYTECODE_MAGIC {
        let ver = u32::from_le_bytes([BYTECODE[4], BYTECODE[5], BYTECODE[6], BYTECODE[7]]);
        assert!(ver == BYTECODE_VERSION, "bytecode version mismatch: got {ver}, expected {BYTECODE_VERSION} — stale blob paired with fresh code (C3). Regenerate via `python cards/compile_abilities.py`");
        &BYTECODE[8..]
    } else {
        BYTECODE
    }
}

#[cfg(all(not(feature = "snes"), not(feature = "gba"), not(feature = "no_std")))]
fn get_decompressed_bytecode() -> &'static [u8] {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<u8>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let raw = miniz_oxide::inflate::decompress_to_vec_zlib(COMPRESSED_BYTECODE).expect("bytecode decompress failed");
        strip_bytecode_header(raw)
    })
}

#[cfg(all(not(feature = "snes"), feature = "no_std", not(feature = "gba")))]
fn get_decompressed_bytecode() -> &'static [u8] {
    use core::cell::UnsafeCell;
    use crate::compat::atomic::AtomicU8;
    use core::sync::atomic::Ordering;

    struct SyncUnsafeCell<T>(UnsafeCell<T>);
    unsafe impl<T> Sync for SyncUnsafeCell<T> {}

    static CACHE: SyncUnsafeCell<Option<Vec<u8>>> = SyncUnsafeCell(UnsafeCell::new(None));
    static STATE: AtomicU8 = AtomicU8::new(0);

    loop {
        match STATE.load(Ordering::Acquire) {
            2 => {
                return unsafe { (*CACHE.0.get()).as_ref().unwrap().as_slice() };
            }
            0 => {
                if STATE
                    .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(
                        COMPRESSED_BYTECODE,
                    )
                    .expect("bytecode decompress failed");
                    let decompressed = strip_bytecode_header(raw);
                    unsafe {
                        *CACHE.0.get() = Some(decompressed);
                    }
                    STATE.store(2, Ordering::Release);
                }
            }
            _ => core::hint::spin_loop(),
        }
    }
}

pub fn get_ability(idx: usize) -> Result<Ability, DecodeError> {
    if idx >= NUM_ABILITIES {
        return Err(DecodeError::IndexOutOfRange {
            idx,
            max: NUM_ABILITIES,
        });
    }
    #[cfg(not(feature = "snes"))]
    let (slice, start, end) = {
        let start = offset_of(idx);
        let end = start + OFFSET_DELTAS[idx] as usize;
        if start >= end {
            return Ok(Ability::default());
        }
        let decompressed = get_decompressed_bytecode();
        (&decompressed[start..end], start, end)
    };
    #[cfg(feature = "snes")]
    let (slice, start, end) = {
        let (ci, start, len) = ABILITY_LOCS[idx];
        if len == 0 {
            return Ok(Ability::default());
        }
        (
            bytecode_slice(ci, start as usize, len as usize),
            start as usize,
            (start + len) as usize,
        )
    };
    let mut bc = BcReader::with_idx(slice, idx);
    if let Some(ability) = decode_ability(&mut bc) {
        return Ok(ability);
    }
    log::error!("bytecode: direct decoder failed for ability {idx} (bytes {start}..{end})");
    Err(DecodeError::DecodeFailed {
        idx,
        byte_range: (start, end),
    })
}

// ── Low-level byte readers (used by BcReader) ──

fn read_u8(c: &mut &[u8]) -> Option<u8> {
    if c.is_empty() {
        return None;
    }
    let b = c[0];
    *c = &c[1..];
    Some(b)
}

fn read_u16(c: &mut &[u8]) -> Option<u16> {
    if c.len() < 2 {
        return None;
    }
    let v = u16::from_le_bytes([c[0], c[1]]);
    *c = &c[2..];
    Some(v)
}

fn read_u32(c: &mut &[u8]) -> Option<u32> {
    if c.len() < 4 {
        return None;
    }
    let v = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
    *c = &c[4..];
    Some(v)
}

fn read_i64(c: &mut &[u8]) -> Option<i64> {
    if c.len() < 8 {
        return None;
    }
    let v = i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
    *c = &c[8..];
    Some(v)
}

// ── BcReader ──

struct BcReader<'a> {
    cursor: &'a [u8],
    /// Ability index being decoded, for decode-audit diagnostics.
    idx: Option<usize>,
}

impl<'a> BcReader<'a> {
    fn with_idx(data: &'a [u8], idx: usize) -> Self {
        BcReader { cursor: data, idx: Some(idx) }
    }

    fn read_u8(&mut self) -> Option<u8> {
        read_u8(&mut self.cursor)
    }

    fn u16(&mut self) -> Option<u16> {
        read_u16(&mut self.cursor)
    }

    fn read_u32(&mut self) -> Option<u32> {
        read_u32(&mut self.cursor)
    }

    /// Read a container length (u8 with 0xFE escape).
    fn read_len(&mut self) -> Option<usize> {
        let b = self.read_u8()?;
        if b < 0xFE {
            Some(b as usize)
        } else {
            self.u16().map(|v| v as usize)
        }
    }

    fn i64(&mut self) -> Option<i64> {
        read_i64(&mut self.cursor)
    }

    /// Read a TAG_I64 value with variable-width payload:
    /// value ≤ 0xFD → 1 byte; 0xFE → +u16; 0xFF → +i32 (two's complement,
    /// signed so the compiler can encode negative ints such as cost_offset).
    fn read_int(&mut self) -> Option<i64> {
        let b = self.read_u8()?;
        if b <= 0xFD {
            Some(b as i64)
        } else if b == 0xFE {
            self.u16().map(|v| v as i64)
        } else if b == 0xFF {
            self.read_u32().map(|v| v as i32 as i64)
        } else {
            self.i64()
        }
    }

    fn key(&mut self) -> Option<&'a str> {
        let idx = self.read_idx()?;
        get_string(idx)
    }

    fn read_idx(&mut self) -> Option<usize> {
        let b = self.read_u8()?;
        if b == 0xFE {
            self.u16().map(|v| v as usize)
        } else {
            Some(b as usize)
        }
    }

    fn skip_value(&mut self) -> Option<()> {
        let tag = self.read_u8()?;
        skip_value_with_tag(self, tag)
    }

    fn read_string_value(&mut self) -> Option<String> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                Some(get_string(idx)?.to_string())
            }
            _ => None,
        }
    }

    fn read_arc_str_value(&mut self) -> Option<ArcStr> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                Some(ArcStr::from(get_string(idx)?))
            }
            _ => None,
        }
    }

    fn read_zone_value(&mut self) -> Option<Zone> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                let s = get_string(idx)?;
                match Zone::from_str(s) {
                    Some(z) => Some(z),
                    None => {
                        note_decode_fallback(self.idx, "zone", s);
                        Some(Zone::Unknown)
                    }
                }
            }
            _ => None,
        }
    }

    fn read_bool_value(&mut self) -> Option<bool> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_TRUE => Some(true),
            TAG_FALSE => Some(false),
            _ => None,
        }
    }

    fn read_u32_value(&mut self) -> Option<u8> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_I64 => u8::try_from(self.read_int()?).ok(),
            _ => None,
        }
    }

    fn read_u8_value(&mut self) -> Option<u8> {
        self.read_u32_value()
    }

    fn read_i8_value(&mut self) -> Option<i8> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_I64 => i8::try_from(self.read_int()?).ok(),
            _ => None,
        }
    }

    fn read_card_type_value(&mut self) -> Option<crate::card::CardType> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                crate::card::CardType::from_card_str(get_string(idx)?)
            }
            _ => None,
        }
    }

    fn read_operator_value(&mut self) -> Option<crate::card::Operator> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                let s = get_string(idx)?;
                match s {
                    ">=" => Some(crate::card::Operator::Gte),
                    "<=" => Some(crate::card::Operator::Lte),
                    ">" => Some(crate::card::Operator::Gt),
                    "<" => Some(crate::card::Operator::Lt),
                    "=" | "==" => Some(crate::card::Operator::Eq),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn read_operation_value(&mut self) -> Option<crate::card::Operation> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= (STRINGS_OFFSETS.len() - 1) {
                    return None;
                }
                crate::card::parse_operation(get_string(idx).unwrap_or(""))
            }
            _ => None,
        }
    }

    fn read_opt_str_vec_value(&mut self) -> Option<Box<Vec<String>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_len()?;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    v.push(self.read_string_value()?);
                }
                Some(Box::new(v))
            }
            _ => None,
        }
    }

    fn read_str_vec_value(&mut self) -> Box<Vec<String>> {
        let tag = self.read_u8().unwrap_or(TAG_NULL);
        match tag {
            TAG_ARRAY => {
                let len = self.read_len().unwrap_or(0);
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    if let Some(s) = self.read_string_value() {
                        v.push(s);
                    }
                }
                Box::new(v)
            }
            _ => Box::new(Vec::new()),
        }
    }

    fn read_condition_value(&mut self) -> Option<Box<Condition>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_OBJECT_VARIANT => {
                let variant = self.read_u8()?;
                Some(Box::new(decode_condition_direct(self, variant)?))
            }
            _ => None,
        }
    }

    fn read_condition_vec_value(&mut self) -> Option<Vec<Box<Condition>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_len()?;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    v.push(self.read_condition_value()?);
                }
                Some(v)
            }
            _ => None,
        }
    }

    fn read_opt_u8_vec_value(&mut self) -> Option<Box<Vec<u8>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_len()?;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    v.push(self.read_u8_value()?);
                }
                Some(Box::new(v))
            }
            _ => None,
        }
    }

    fn read_distinct_info_value(&mut self) -> Option<Box<crate::card::DistinctInfo>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_FALSE => Some(Box::new(crate::card::DistinctInfo::Boolean(false))),
            TAG_TRUE => Some(Box::new(crate::card::DistinctInfo::Boolean(true))),
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= (STRINGS_OFFSETS.len() - 1) {
                    return None;
                }
                Some(Box::new(crate::card::DistinctInfo::String(
                    get_string(idx).unwrap_or("").to_string(),
                )))
            }
            _ => None,
        }
    }

    fn read_positions_characters_value(
        &mut self,
    ) -> Option<Box<Vec<crate::card::PositionCharacter>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_len()?;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    let otag = self.read_u8()?;
                    if otag != TAG_OBJECT && otag != TAG_OBJECT_VARIANT {
                        skip_value_with_tag(self, otag)?;
                        continue;
                    }
                    if otag == TAG_OBJECT_VARIANT {
                        self.read_u8()?;
                    }
                    let count = self.read_len()?;
                    let mut position = String::new();
                    let mut character = String::new();
                    for _ in 0..count {
                        let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                            "position" => {
                                position = self.read_string_value().unwrap_or_default();
                            }
                            "character" => {
                                character = self.read_string_value().unwrap_or_default();
                            }
                            _ => {
                                self.skip_value()?;
                            }
                        }
                    }
                    v.push(crate::card::PositionCharacter {
                        position,
                        character,
                    });
                }
                Some(Box::new(v))
            }
            _ => None,
        }
    }

    #[allow(dead_code)] // deserialization mirror kept for symmetry with the serializer
    fn read_cost_comparison_value(&mut self) -> Option<crate::card::CostComparison> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_len()?;
                let mut operator = None;
                let mut relative_to = None;
                let mut cost_limit = None;
                let mut cost_limit_operator = None;
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                        "operator" => {
                            operator = self.read_operator_value();
                        }
                        "relative_to" => {
                            relative_to = self.read_arc_str_value();
                        }
                        "cost_limit" => {
                            cost_limit = self.read_u8_value();
                        }
                        "cost_limit_operator" => {
                            cost_limit_operator = self.read_operator_value();
                        }
                        _ => {
                            self.skip_value()?;
                        }
                    }
                }
                Some(crate::card::CostComparison {
                    operator,
                    relative_to,
                    cost_limit,
                    cost_limit_operator,
                })
            }
            _ => None,
        }
    }

    #[allow(dead_code)] // deserialization mirror kept for symmetry with the serializer
    fn read_trigger_event_value(&mut self) -> Option<Box<crate::card::TriggerEvent>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_len()?;
                let mut te = crate::card::TriggerEvent::default();
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                        "type" => {
                            te.event_type = self.read_arc_str_value();
                        }
                        "tense" => {
                            te.tense = self.read_arc_str_value();
                        }
                        "location" => {
                            te.location = self.read_arc_str_value();
                        }
                        "source_character" => {
                            te.source_character = self.read_arc_str_value();
                        }
                        "source_group" => {
                            te.source_group = self.read_arc_str_value();
                        }
                        "cost_comparison" => {
                            te.cost_comparison = self.read_cost_comparison_value();
                        }
                        "min_count" => {
                            te.min_count = self.read_u8_value();
                        }
                        "exclude_characters" => {
                            te.exclude_characters = self.read_opt_str_vec_value();
                        }
                        "ability_filter" => {
                            te.ability_filter = self.read_ability_filter_value();
                        }
                        "self_effect_only" => {
                            te.self_effect_only = self.read_bool_value();
                        }
                        "energy_placed" => {
                            te.energy_placed = self.read_bool_value();
                        }
                        "phase" => {
                            te.phase = self.read_arc_str_value();
                        }
                        "phase_target" => {
                            te.phase_target = self.read_arc_str_value();
                        }
                        "recurrence" => {
                            te.recurrence = self.read_arc_str_value();
                        }
                        "events" => {
                            let etag = self.read_u8()?;
                            if etag == TAG_ARRAY {
                                let elen = self.read_len()?;
                                let mut events = Vec::with_capacity(elen);
                                for _ in 0..elen {
                                    events.push(*self.read_trigger_event_value()?);
                                }
                                te.events = Some(events);
                            } else {
                                skip_value_with_tag(self, etag)?;
                            }
                        }
                        "source" => {
                            te.source = self.read_arc_str_value();
                        }
                        "destination" => {
                            te.destination = self.read_arc_str_value();
                        }
                        "from_state" => {
                            te.from_state = self.read_arc_str_value();
                        }
                        "to_state" => {
                            te.to_state = self.read_arc_str_value();
                        }
                        _ => {
                            self.skip_value()?;
                        }
                    }
                }
                Some(Box::new(te))
            }
            _ => None,
        }
    }

    fn read_location_sub_checks_value(&mut self) -> Option<Box<crate::card::LocationSubChecks>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_len()?;
                let mut lsc = crate::card::LocationSubChecks::default();
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                        "card_property" => {
                            let s = self.read_arc_str_value();
                            lsc.card_property = s.map(|s| crate::card::CardProperty::from_str(&s));
                        }
                        "baton_touch_trigger" => {
                            lsc.baton_touch_trigger = self.read_bool_value();
                        }
                        "baton_touch_source" => {
                            lsc.baton_touch_source = self.read_arc_str_value();
                        }
                        "min_baton_touch_count" => {
                            lsc.min_baton_touch_count = self.read_u8_value();
                        }
                        "ability_filter" => {
                            let s = self.read_arc_str_value();
                            lsc.ability_filter =
                                s.map(|s| crate::card::AbilityFilter::from_str(&s));
                        }
                        "ability_filter_triggers" => {
                            lsc.ability_filter_triggers = self
                                .read_opt_str_vec_value()
                                .map(|b| (*b).into_iter().collect());
                        }
                        "aggregate" => {
                            lsc.aggregate = self.read_arc_str_value();
                        }
                        "no_excess_heart" => {
                            lsc.no_excess_heart = self.read_bool_value();
                        }
                        "original_value" => {
                            lsc.original_value = self.read_bool_value();
                        }
                        "activation_position" => {
                            lsc.activation_position = self.read_arc_str_value();
                        }
                        "unit" => {
                            lsc.unit = self.read_arc_str_value();
                        }
                        "values" => {
                            lsc.values = self.read_opt_u8_vec_value().map(|b| *b);
                        }
                        "group_reference" => {
                            lsc.group_reference = self.read_arc_str_value();
                        }
                        "reference_card" => {
                            lsc.reference_card = self.read_arc_str_value();
                        }
                        _ => {
                            self.skip_value()?;
                        }
                    }
                }
                Some(Box::new(lsc))
            }
            _ => None,
        }
    }

    fn read_effect_value(&mut self) -> Option<Box<AbilityEffect>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_OBJECT_VARIANT => {
                let variant = self.read_u8()?;
                Some(Box::new(decode_ability_effect_direct(self, variant)?))
            }
            _ => None,
        }
    }

    fn read_effect_vec_value(&mut self) -> Option<Vec<Box<AbilityEffect>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_len()?;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    let sub_tag = self.read_u8()?;
                    match sub_tag {
                        TAG_OBJECT_VARIANT => {
                            let variant = self.read_u8()?;
                            v.push(Box::new(decode_ability_effect_direct(self, variant)?));
                        }
                        _ => {
                            // sub_tag was already consumed; skip the body without
                            // re-reading the tag byte.
                            skip_value_with_tag(self, sub_tag)?;
                        }
                    }
                }
                Some(v)
            }
            _ => None,
        }
    }

    fn read_effect_vec_boxed_value(&mut self) -> Option<Box<Vec<Box<AbilityEffect>>>> {
        self.read_effect_vec_value().map(Box::new)
    }

    fn read_position_value(&mut self) -> Option<Box<crate::card::PositionInfo>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= (STRINGS_OFFSETS.len() - 1) {
                    return None;
                }
                Some(Box::new(crate::card::PositionInfo::String(
                    get_string(idx).unwrap_or("").to_string(),
                )))
            }
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_len()?;
                let mut position = None;
                let mut target = None;
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                        "position" => {
                            position = self.read_arc_str_value();
                        }
                        "target" => {
                            target = self.read_arc_str_value();
                        }
                        _ => {
                            self.skip_value()?;
                        }
                    }
                }
                Some(Box::new(crate::card::PositionInfo::Struct {
                    position,
                    target,
                }))
            }
            _ => None,
        }
    }

    fn read_dynamic_count_value(&mut self) -> Option<Box<crate::card::DynamicCount>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_len()?;
                let mut count_type = String::new();
                let mut reference = None;
                let mut mode = None;
                let mut base_reference = None;
                let mut calculation = None;
                let mut calculation_value = None;
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                        "type" => {
                            count_type = self.read_string_value().unwrap_or_default();
                        }
                        "reference" => {
                            reference = self.read_arc_str_value();
                        }
                        "mode" => {
                            mode = self.read_arc_str_value();
                        }
                        "base_reference" => {
                            base_reference = self.read_arc_str_value();
                        }
                        "calculation" => {
                            calculation = self.read_arc_str_value();
                        }
                        "calculation_value" => {
                            calculation_value = self.read_u8_value();
                        }
                        _ => {
                            self.skip_value()?;
                        }
                    }
                }
                Some(Box::new(crate::card::DynamicCount {
                    count_type,
                    reference,
                    mode,
                    base_reference,
                    calculation,
                    calculation_value,
                }))
            }
            _ => None,
        }
    }

    fn read_effect_state_value(&mut self) -> Option<Box<super::enums::EffectState>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= (STRINGS_OFFSETS.len() - 1) {
                    return None;
                }
                Some(Box::new(super::enums::EffectState::from_str(get_string(idx).unwrap_or(""))))
            }
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_len()?;
                if count > 0 {
                    // A non-empty object was discarded — its fields carried
                    // state the default EffectState does not represent.
                    note_decode_fallback(self.idx, "effect_state_nonempty_object", "");
                }
                for _ in 0..count {
                    self.skip_value()?;
                }
                Some(Box::new(super::enums::EffectState::default()))
            }
            _ => None,
        }
    }

    fn read_distinct_value(&mut self) -> Option<Box<crate::card::DistinctType>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_FALSE => Some(Box::new(crate::card::DistinctType::CardName)),
            TAG_TRUE => Some(Box::new(crate::card::DistinctType::True)),
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= (STRINGS_OFFSETS.len() - 1) {
                    return None;
                }
                Some(Box::new(match get_string(idx).unwrap_or("") {
                    "card_name" => crate::card::DistinctType::CardName,
                    "true" | "distinct" => crate::card::DistinctType::True,
                    other => {
                        note_decode_fallback(self.idx, "distinct", other);
                        crate::card::DistinctType::CardName
                    }
                }))
            }
            _ => None,
        }
    }

    fn read_ability_filter_value(&mut self) -> Option<AbilityFilter> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => Some(AbilityFilter::NoAbility),
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= (STRINGS_OFFSETS.len() - 1) {
                    return None;
                }
                Some(match get_string(idx).unwrap_or("") {
                    "has_ability" => AbilityFilter::HasAbility,
                    "has_ability_type" => AbilityFilter::HasAbilityType,
                    "no_ability_type" => AbilityFilter::NoAbilityType,
                    "no_ability" => AbilityFilter::NoAbility,
                    other => {
                        note_decode_fallback(self.idx, "ability_filter", other);
                        AbilityFilter::NoAbility
                    }
                })
            }
            _ => {
                note_decode_fallback(self.idx, "ability_filter_tag", &format!("{tag:#04x}"));
                Some(AbilityFilter::NoAbility)
            }
        }
    }

    fn read_or_ability_filters_value(&mut self) -> Option<Box<Vec<AbilityFilterBranch>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_len()?;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    let tag2 = self.read_u8()?;
                    if tag2 == TAG_OBJECT || tag2 == TAG_OBJECT_VARIANT {
                        if tag2 == TAG_OBJECT_VARIANT {
                            self.read_u8()?;
                        }
                        let count = self.read_len()?;
                        let mut af = None;
                        let mut aft = None;
                        for _ in 0..count {
                            let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                                "ability_filter" => {
                                    af = Some(self.read_ability_filter_value()?);
                                }
                                "ability_filter_triggers" => {
                                    aft = self.read_opt_str_vec_value().map(|b| *b);
                                }
                                _ => {
                                    self.skip_value()?;
                                }
                            }
                        }
                        v.push(AbilityFilterBranch {
                            ability_filter: af,
                            ability_filter_triggers: aft,
                        });
                    } else {
                        self.skip_value()?;
                    }
                }
                Some(Box::new(v))
            }
            _ => None,
        }
    }

    fn read_placement_order_value(&mut self) -> Option<crate::card::PlacementOrder> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                let s = get_string(idx)?;
                match s {
                    "any_order" => Some(crate::card::PlacementOrder::AnyOrder),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn read_quoted_text_value(&mut self) -> Option<Box<crate::card::QuotedText>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_len()?;
                let mut text = String::new();
                let mut quoted_type = String::new();
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                        let kstr = get_string(kidx)?;
                        match kstr {
                        "text" => {
                            text = self.read_string_value().unwrap_or_default();
                        }
                        "quoted_type" => {
                            quoted_type = self.read_string_value().unwrap_or_default();
                        }
                        _ => {
                            self.skip_value()?;
                        }
                    }
                }
                Some(Box::new(crate::card::QuotedText { text, quoted_type }))
            }
            _ => None,
        }
    }
}

fn skip_value_with_tag(bc: &mut BcReader, tag: u8) -> Option<()> {
    match tag {
        TAG_NULL | TAG_FALSE | TAG_TRUE => Some(()),
        TAG_I64 => {
            let b = bc.read_u8()?;
            if b > 0xFD {
                // 0xFE → u16, 0xFF → u32
                let skip = if b == 0xFE {
                    2
                } else if b == 0xFF {
                    4
                } else {
                    8
                };
                if bc.cursor.len() < skip {
                    return None;
                }
                bc.cursor = &bc.cursor[skip..];
            }
            Some(())
        }
        TAG_F64 => {
            if bc.cursor.len() < 8 {
                return None;
            }
            bc.cursor = &bc.cursor[8..];
            Some(())
        }
        TAG_STR => {
            bc.read_idx()?;
            Some(())
        }
        TAG_ARRAY => {
            let len = bc.read_len()?;
            for _ in 0..len {
                bc.skip_value()?;
            }
            Some(())
        }
        TAG_OBJECT | TAG_OBJECT_VARIANT => {
            if tag == TAG_OBJECT_VARIANT {
                bc.read_u8()?;
            }
            let len = bc.read_len()?;
            for _ in 0..len {
                bc.read_idx()?;
                bc.skip_value()?;
            }
            Some(())
        }
        _ => None,
    }
}

/// Read a tagged value from a BcReader into a serde_json::Value.
// ── Direct Ability decoder ──

fn decode_ability(bc: &mut BcReader) -> Option<Ability> {
    let tag = bc.read_u8()?;
    if tag != TAG_OBJECT {
        return None;
    }
    let count = bc.read_len()?;

    let mut full_text = String::new();
    let mut triggerless_text: Option<String> = None;
    let mut triggers: Option<ArcStr> = None;
    let mut use_limit: Option<u8> = None;
    let mut is_null = false;
    let mut cost: Option<Box<AbilityCost>> = None;
    let mut effect: Option<Box<AbilityEffect>> = None;
    let mut keywords: Option<Vec<crate::card::Keyword>> = None;

    for _i in 0..count {
        let key = bc.key()?;
        match key {
            "full_text" => {
                full_text = bc.read_string_value()?;
            }
            "triggerless_text" => {
                triggerless_text = bc.read_string_value();
            }
            "triggers" => {
                triggers = bc.read_arc_str_value();
            }
            "use_limit" => {
                use_limit = bc.read_u32_value();
            }
            "is_null" => {
                is_null = bc.read_bool_value().unwrap_or(false);
            }
            "cost" => {
                cost = decode_ability_cost(bc)?;
            }
            "effect" => {
                effect = decode_ability_effect(bc)?.map(Box::new);
            }
            "keywords" => {
                keywords = decode_keywords(bc)?;
            }
            _ => {
                bc.skip_value()?;
            }
        }
    }

    Some(Ability {
        full_text,
        triggerless_text,
        triggers,
        use_limit,
        is_null,
        cost,
        effect,
        keywords,
    })
}

// ── AbilityEffect decoder ──

fn decode_ability_cost(bc: &mut BcReader) -> Option<Option<Box<AbilityCost>>> {
    let tag = bc.read_u8()?;
    match tag {
        TAG_NULL => Some(None),
        TAG_OBJECT_VARIANT => {
            // The compiler aliases cost `type`→`action` and `zone`→`source`, so
            // costs decode through the same direct effect decoder as effects.
            let variant = bc.read_u8()?;
            let inner = decode_ability_effect_direct(bc, variant)?;
            Some(Some(Box::new(AbilityCost(inner))))
        }
        _ => None,
    }
}

fn decode_ability_effect(bc: &mut BcReader) -> Option<Option<AbilityEffect>> {
    let tag = bc.read_u8()?;
    match tag {
        TAG_NULL => Some(None),
        TAG_OBJECT_VARIANT => {
            let variant = bc.read_u8()?;
            let inner = decode_ability_effect_direct(bc, variant)?;
            Some(Some(inner))
        }
        _ => None,
    }
}

/// Apply AbilityCost-specific key normalizations recursively.
/// The bytecode stores the exact JSON keys from abilities.json. AbilityCost
/// objects use "type" (not "action"), "zone" (not "source"), and
/// "options"/"costs" (not "actions") — because the legacy AbilityCost
/// custom Deserialize handles these aliases.
#[cfg(feature = "json_path_test")]
pub fn normalize_cost_keys(map: &mut serde_json::Map<String, serde_json::Value>) {
    if !map.contains_key("action") {
        if let Some(v) = map.remove("type").or_else(|| map.remove("cost_type")) {
            map.insert("action".into(), v);
        }
    }
    if !map.contains_key("source") {
        if let Some(v) = map.remove("zone") {
            map.insert("source".into(), v);
        }
    }
    if !map.contains_key("actions") {
        if let Some(v) = map.remove("options").or_else(|| map.remove("costs")) {
            map.insert("actions".into(), v);
        }
    }
    // Recurse into nested values (arrays, sub-objects).
    for (_k, v) in map.iter_mut() {
        recursive_normalize_cost_value(v);
    }
}

#[cfg(feature = "json_path_test")]
fn recursive_normalize_cost_value(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::Object(m) => {
            normalize_cost_keys(m);
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                recursive_normalize_cost_value(item);
            }
        }
        _ => {}
    }
}

/// Direct decoder for TAG_OBJECT_VARIANT effects using the generated field dispatch.
fn decode_ability_effect_direct(bc: &mut BcReader, _variant: u8) -> Option<AbilityEffect> {
    let count = bc.read_len()?;
    let mut text = ArcStr::from("");
    let mut action = ActionType::default();
    let mut source: Option<Zone> = None;
    let mut destination: Option<Zone> = None;
    let mut count_val: Option<u8> = None;
    let mut target: Option<ArcStr> = None;
    let mut condition: Option<Box<Condition>> = None;
    let mut non_stackable: Option<bool> = None;
    let mut conditional: Option<bool> = None;
    let mut is_further: Option<bool> = None;
    let mut optional: Option<bool> = None;
    let mut max: Option<bool> = None;
    let mut effect_steps: Option<Vec<Box<AbilityEffect>>> = None;
    let mut look_action: Option<Box<AbilityEffect>> = None;
    let mut select_action: Option<Box<AbilityEffect>> = None;
    let mut actions: Option<Vec<Box<AbilityEffect>>> = None;
    let mut primary_effect: Option<Box<AbilityEffect>> = None;
    let mut alternative_condition: Option<Box<Condition>> = None;
    let mut result_condition: Option<Box<Condition>> = None;
    let mut followup_action: Option<Box<AbilityEffect>> = None;
    let mut optional_action: Option<Box<AbilityEffect>> = None;
    let mut conditional_action: Option<Box<AbilityEffect>> = None;
    let mut conditional_negation: Option<bool> = None;
    let mut cost_reduction_per_group: Option<u8> = None;
    let mut ek = EffectKindLocals::default();

    for _ in 0..count {
        let key = bc.key()?;
        decode_effect_field(
            bc,
            key,
            &mut text,
            &mut action,
            &mut source,
            &mut destination,
            &mut count_val,
            &mut target,
            &mut condition,
            &mut non_stackable,
            &mut conditional,
            &mut is_further,
            &mut optional,
            &mut max,
            &mut effect_steps,
            &mut cost_reduction_per_group,
            &mut look_action,
            &mut select_action,
            &mut actions,
            &mut primary_effect,
            &mut alternative_condition,
            &mut result_condition,
            &mut followup_action,
            &mut optional_action,
            &mut conditional_action,
            &mut conditional_negation,
            &mut ek,
        )?;
    }

    // The old JSON path feeds the same JSON value into both AbilityEffect AND EffectKind.
    // Fields like source, target, destination, count exist on AbilityEffect (AE) and also on
    // many EffectKind variants. The AE dispatch consumes them first so ek never sees them.
    // Copy the overlapping AE fields into ek so build_* functions have them.
    ek.source = source.clone();
    ek.target = target.clone();
    ek.destination = destination.clone();
    ek.count = count_val;
    ek.optional = optional;
    ek.non_stackable = non_stackable;
    ek.alternative_condition = alternative_condition.clone();

    // Build the EffectKind variant from the action via the shared single
    // derivation (single source of truth shared with the JSON deep-compare path).
    let filter = build_filter(&ek);
    let kind = EffectKind::from_action(action.to_str(), filter).map(ek_box_new);
    let effect = AbilityEffect {
        text,
        action,
        source,
        destination,
        count: count_val,
        target,
        condition,
        non_stackable,
        conditional,
        is_further,
        optional,
        max,
        effect_steps,
        cost_reduction_per_group,
        compound: Box::new(crate::card::CompoundBranch {
            look_action,
            select_action,
            actions,
            primary_effect,
            alternative_condition,
            result_condition,
            followup_action,
            optional_action,
            conditional_action,
            conditional_negation,
        }),
        kind,
    };

    Some(effect)
}

// ── populate_from_json (JSON-path decode; used only by the deep-compare oracle) ──

#[cfg(feature = "json_path_test")]
impl AbilityEffect {
    /// Populate `kind` from this effect's JSON value. Recurses into sub-effects.
    pub fn populate_from_json(&mut self, json_val: &serde_json::Value) {
        if let Some(kind) = Self::kind_from_action(self.action.to_str(), json_val) {
            self.kind = Some(ek_box_new(kind));
        }
        if let Some(ref mut sub) = self.compound.look_action {
            if let Some(sub_json) = json_val.get("look_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.select_action {
            if let Some(sub_json) = json_val.get("select_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.followup_action {
            if let Some(sub_json) = json_val.get("followup_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.primary_effect {
            if let Some(sub_json) = json_val.get("primary_effect") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.optional_action {
            if let Some(sub_json) = json_val.get("optional_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.conditional_action {
            if let Some(sub_json) = json_val.get("conditional_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut actions) = self.compound.actions {
            if let Some(json_actions) = json_val.get("actions").and_then(|a| a.as_array()) {
                for (i, action) in actions.iter_mut().enumerate() {
                    if i < json_actions.len() {
                        action.populate_from_json(&json_actions[i]);
                    }
                }
            }
        }
        if let Some(ref mut steps) = self.effect_steps {
            if let Some(json_steps) = json_val.get("effect_steps").and_then(|a| a.as_array()) {
                for (i, step) in steps.iter_mut().enumerate() {
                    if i < json_steps.len() {
                        step.populate_from_json(&json_steps[i]);
                    }
                }
            }
        }
        if let Some(ref mut cond) = self.condition {
            if let Some(cond_json) = json_val.get("condition") {
                condition_populate_from_json(cond, cond_json);
            }
        }
        if let Some(opts) = self
            .kind
            .as_deref_mut()
            .and_then(|k| k.filter_mut())
            .and_then(|f| f.options.as_mut())
        {
            if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                for (i, opt) in opts.iter_mut().enumerate() {
                    if i < json_opts.len() {
                        opt.populate_from_json(&json_opts[i]);
                    }
                }
            }
        }
        // Nested sub-effects now live on the shared filter, so populate them
        // whenever the JSON provides them (no per-variant match needed).
        if let Some(f) = self.kind.as_deref_mut().and_then(|k| k.filter_mut()) {
            if let Some(ref mut ros) = f.resource_on_select {
                if let Some(ros_json) = json_val.get("resource_on_select") {
                    ros.populate_from_json(ros_json);
                }
            }
            if let Some(ref mut ae) = f.alternative_effect {
                if let Some(ae_json) = json_val.get("alternative_effect") {
                    ae.populate_from_json(ae_json);
                }
            }
            if let Some(ref mut ge) = f.gained_effect {
                if let Some(ge_json) = json_val.get("gained_effect") {
                    ge.populate_from_json(ge_json);
                }
            }
            if let Some(ref mut oa) = f.opponent_action {
                if let Some(oa_json) = json_val.get("opponent_action") {
                    oa.populate_from_json(oa_json);
                }
            }
        }
    }
}

#[cfg(feature = "json_path_test")]
fn condition_populate_from_json(cond: &mut Condition, cond_json: &serde_json::Value) {
    if let Condition::Choice {
        ref mut options, ..
    } = cond
    {
        if let Some(ref mut opts) = options {
            if let Some(json_opts) = cond_json.get("options").and_then(|a| a.as_array()) {
                for (i, opt) in opts.iter_mut().enumerate() {
                    if i < json_opts.len() {
                        opt.populate_from_json(&json_opts[i]);
                    }
                }
            }
        }
    }
    if let Condition::Complex { ref mut effect, .. } = cond {
        if let Some(ref mut eff) = effect {
            if let Some(eff_json) = cond_json.get("effect") {
                eff.populate_from_json(eff_json);
            }
        }
    }
    if let Condition::Compound {
        ref mut common,
        ref mut conditions,
    } = cond
    {
        if common.operator.is_none()
            && cond_json.get("type").and_then(|t| t.as_str()) == Some("or_condition")
        {
            common.operator = Some("or".into());
        }
        if let Some(ref mut conditions) = conditions {
            if let Some(json_conditions) = cond_json.get("conditions").and_then(|a| a.as_array()) {
                for (i, sub_cond) in conditions.iter_mut().enumerate() {
                    if i < json_conditions.len() {
                        condition_populate_from_json(sub_cond, &json_conditions[i]);
                    }
                }
            }
        }
    }
    if let Condition::Temporal {
        ref mut condition, ..
    } = cond
    {
        if let Some(ref mut sub_cond) = condition {
            if let Some(sub_cond_json) = cond_json.get("condition") {
                condition_populate_from_json(sub_cond, sub_cond_json);
            }
        }
    }
}

// ── Keyword decoder ──

fn decode_keywords(bc: &mut BcReader) -> Option<Option<Vec<crate::card::Keyword>>> {
    let tag = bc.read_u8()?;
    match tag {
        TAG_NULL => Some(None),
        TAG_ARRAY => {
            let len = bc.read_len()?;
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                let s = bc.read_string_value()?;
                match keyword_from_str(&s) {
                    Some(k) => v.push(k),
                    None => {
                        note_decode_fallback(bc.idx, "keyword", &s);
                    }
                }
            }
            Some(Some(v))
        }
        _ => None,
    }
}

fn keyword_from_str(s: &str) -> Option<crate::card::Keyword> {
    Some(match s {
        "Turn1" => crate::card::Keyword::Turn1,
        "Turn2" => crate::card::Keyword::Turn2,
        "Debut" => crate::card::Keyword::Debut,
        "LiveStart" => crate::card::Keyword::LiveStart,
        "LiveSuccess" => crate::card::Keyword::LiveSuccess,
        "Center" => crate::card::Keyword::Center,
        "LeftSide" => crate::card::Keyword::LeftSide,
        "RightSide" => crate::card::Keyword::RightSide,
        "PositionChange" => crate::card::Keyword::PositionChange,
        "FormationChange" => crate::card::Keyword::FormationChange,
        _ => return None,
    })
}
