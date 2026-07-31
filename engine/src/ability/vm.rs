use super::abilities_gen::{BYTECODE, NUM_ABILITIES, OFFSETS, STRINGS};
use super::enums::EffectState;
use crate::ability::enums::ActionType;
use crate::card::{
    ek_box_new, Ability, AbilityCost, AbilityEffect, AbilityFilter, AbilityFilterBranch, CardType,
    Condition, DistinctType, DynamicCount, EffectKind, Operator, PlacementOrder, PositionInfo,
    QuotedText,
};
use crate::core::types::ArcStr;

include!("effect_decoder_gen.rs");

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};

/// Errors that can occur when decoding an ability from bytecode.
#[derive(Debug, Clone)]
pub enum DecodeError {
    /// Ability index is out of range.
    IndexOutOfRange { idx: usize, max: usize },
    /// Bytecode slice is empty (offset start >= end).
    EmptySlice { idx: usize },
    /// Direct decoder failed and serde fallback also failed.
    DecodeFailed {
        idx: usize,
        byte_range: (usize, usize),
    },
    /// Serde deserialization failed after successful JSON reconstruction.
    SerdeFailed { idx: usize, detail: String },
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
            DecodeError::SerdeFailed { idx, detail } => {
                write!(f, "ability {idx} serde failed: {detail}")
            }
        }
    }
}

// DS debug screen print
#[cfg(feature = "ds_debug")]
extern "C" {
    fn nds_println(text: *const u8);
}
#[cfg(feature = "ds_debug")]
fn ds_print(s: &str) {
    use alloc::string::ToString;
    let mut msg = s.to_string();
    msg.push('\0');
    unsafe {
        nds_println(msg.as_ptr());
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

/// Decode a single ability from the bytecode blob.
///
/// Returns `Ok(Ability)` on success, or `Err(DecodeError)` with the ability
/// index and byte range so callers can log precise diagnostics.
///
/// # Lazy decoding flow
/// 1. `card_loader` stores only u16 indices (no decode at load time)
/// 2. On first trigger/access, `AbilityRef::resolve()` calls `get_ability(idx)`
/// 3. Decode from bytecode, falling back to serde path if direct decode fails
/// 4. Caller wraps in Arc and drops when done — no memory leak
///
/// # RAM savings
/// Before: All 800 abilities decoded eagerly at load = ~2.8MB
/// After: Only ~30-45 abilities triggered per game decoded = ~120KB
pub fn get_ability(idx: usize) -> Result<Ability, DecodeError> {
    if idx >= NUM_ABILITIES {
        return Err(DecodeError::IndexOutOfRange {
            idx,
            max: NUM_ABILITIES,
        });
    }
    let start = OFFSETS[idx] as usize;
    let end = OFFSETS[idx + 1] as usize;
    if start >= end {
        return Ok(Ability::default());
    }
    let slice = &BYTECODE[start..end];
    #[cfg(feature = "ds_debug")]
    {
        extern "C" {
            fn nds_println(t: *const u8);
        }
        let mut m = alloc::string::String::new();
        m.push_str("IDX:");
        m.push_str(&alloc::string::ToString::to_string(&idx));
        m.push_str(" sz:");
        m.push_str(&alloc::string::ToString::to_string(&(end - start)));
        m.push('\0');
        unsafe {
            nds_println(m.as_ptr());
        }
    }
    let mut bc = BcReader::new(slice);
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
}

impl<'a> BcReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BcReader { cursor: data }
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

    fn i64(&mut self) -> Option<i64> {
        read_i64(&mut self.cursor)
    }

    fn key(&mut self) -> Option<&'a str> {
        let idx = self.read_idx()?;
        if idx >= STRINGS.len() {
            return None;
        }
        Some(STRINGS[idx])
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
                if idx >= STRINGS.len() {
                    return None;
                }
                Some(STRINGS[idx].to_string())
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
                if idx >= STRINGS.len() {
                    return None;
                }
                Some(ArcStr::from(STRINGS[idx]))
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
            TAG_I64 => Some(self.i64()? as u8),
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
            TAG_I64 => Some(self.i64()? as i8),
            _ => None,
        }
    }

    fn read_card_type_value(&mut self) -> Option<crate::card::CardType> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= STRINGS.len() {
                    return None;
                }
                crate::card::CardType::from_card_str(STRINGS[idx])
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
                if idx >= STRINGS.len() {
                    return None;
                }
                match STRINGS[idx] {
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

    fn read_opt_str_vec_value(&mut self) -> Option<Box<Vec<String>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_u32()? as usize;
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
                let len = self.read_u32().unwrap_or(0) as usize;
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
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_u32()? as usize;
                let mut map = serde_json::Map::with_capacity(count);
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                    if kidx >= STRINGS.len() {
                        return None;
                    }
                    map.insert(STRINGS[kidx].to_string(), self.read_json_value()?);
                }
                let map_val = serde_json::Value::Object(map);
                let cond: Condition = serde_json::from_value(map_val.clone()).ok()?;
                let mut boxed = Box::new(cond);
                condition_populate_from_json(&mut boxed, &map_val);
                Some(boxed)
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
            TAG_OBJECT => Some(Box::new(decode_ability_effect_from_object(self)?)),
            _ => None,
        }
    }

    fn read_effect_vec_value(&mut self) -> Option<Vec<Box<AbilityEffect>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_u32()? as usize;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    let sub_tag = self.read_u8()?;
                    match sub_tag {
                        TAG_OBJECT_VARIANT => {
                            let variant = self.read_u8()?;
                            v.push(Box::new(decode_ability_effect_direct(self, variant)?));
                        }
                        TAG_OBJECT => v.push(Box::new(decode_ability_effect_from_object(self)?)),
                        _ => {
                            self.skip_value()?;
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
                if idx >= STRINGS.len() {
                    return None;
                }
                let s = STRINGS[idx];
                let val = serde_json::Value::String(s.to_string());
                serde_json::from_value(val).ok().map(Box::new)
            }
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_u32()? as usize;
                let mut map = serde_json::Map::with_capacity(count);
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                    if kidx >= STRINGS.len() {
                        return None;
                    }
                    map.insert(STRINGS[kidx].to_string(), self.read_json_value()?);
                }
                let pi: crate::card::PositionInfo =
                    serde_json::from_value(serde_json::Value::Object(map)).ok()?;
                Some(Box::new(pi))
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
                let count = self.read_u32()? as usize;
                let mut map = serde_json::Map::with_capacity(count);
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                    if kidx >= STRINGS.len() {
                        return None;
                    }
                    map.insert(STRINGS[kidx].to_string(), self.read_json_value()?);
                }
                let dc: crate::card::DynamicCount =
                    serde_json::from_value(serde_json::Value::Object(map)).ok()?;
                Some(Box::new(dc))
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
                if idx >= STRINGS.len() {
                    return None;
                }
                // EffectState can be a plain string like "wait", "active", etc.
                // Try to deserialize from the string value.
                let s = STRINGS[idx];
                let val = serde_json::Value::String(s.to_string());
                serde_json::from_value(val).ok().map(Box::new)
            }
            TAG_OBJECT | TAG_OBJECT_VARIANT => {
                if tag == TAG_OBJECT_VARIANT {
                    self.read_u8()?;
                }
                let count = self.read_u32()? as usize;
                let mut map = serde_json::Map::with_capacity(count);
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                    if kidx >= STRINGS.len() {
                        return None;
                    }
                    map.insert(STRINGS[kidx].to_string(), self.read_json_value()?);
                }
                let st: super::enums::EffectState =
                    serde_json::from_value(serde_json::Value::Object(map)).ok()?;
                Some(Box::new(st))
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
                if idx >= STRINGS.len() {
                    return None;
                }
                Some(Box::new(match STRINGS[idx] {
                    "card_name" => crate::card::DistinctType::CardName,
                    "true" | "distinct" => crate::card::DistinctType::True,
                    _ => crate::card::DistinctType::CardName,
                }))
            }
            _ => None,
        }
    }

    fn read_json_value(&mut self) -> Option<serde_json::Value> {
        read_value_from_bc(self)
    }

    fn read_ability_filter_value(&mut self) -> Option<AbilityFilter> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => Some(AbilityFilter::NoAbility),
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= STRINGS.len() {
                    return None;
                }
                Some(match STRINGS[idx] {
                    "has_ability" => AbilityFilter::HasAbility,
                    "has_ability_type" => AbilityFilter::HasAbilityType,
                    "no_ability_type" => AbilityFilter::NoAbilityType,
                    _ => AbilityFilter::NoAbility,
                })
            }
            _ => Some(AbilityFilter::NoAbility),
        }
    }

    fn read_or_ability_filters_value(&mut self) -> Option<Box<Vec<AbilityFilterBranch>>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_ARRAY => {
                let len = self.read_u32()? as usize;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    let tag2 = self.read_u8()?;
                    if tag2 == TAG_OBJECT || tag2 == TAG_OBJECT_VARIANT {
                        if tag2 == TAG_OBJECT_VARIANT {
                            self.read_u8()?;
                        }
                        let count = self.read_u32()? as usize;
                        let mut af = None;
                        let mut aft = None;
                        for _ in 0..count {
                            let kidx = self.read_idx()?;
                            if kidx >= STRINGS.len() {
                                self.skip_value()?;
                                continue;
                            }
                            match STRINGS[kidx] {
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

    fn read_boxed_arcstr_value(&mut self) -> Option<Box<ArcStr>> {
        let tag = self.read_u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.read_idx()?;
                if idx >= STRINGS.len() {
                    return None;
                }
                Some(Box::new(ArcStr::from(STRINGS[idx])))
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
                if idx >= STRINGS.len() {
                    return None;
                }
                match STRINGS[idx] {
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
                let count = self.read_u32()? as usize;
                let mut map = serde_json::Map::with_capacity(count);
                for _ in 0..count {
                    let kidx = self.read_idx()?;
                    if kidx >= STRINGS.len() {
                        return None;
                    }
                    map.insert(STRINGS[kidx].to_string(), self.read_json_value()?);
                }
                let qt: crate::card::QuotedText =
                    serde_json::from_value(serde_json::Value::Object(map)).ok()?;
                Some(Box::new(qt))
            }
            _ => None,
        }
    }
}

fn skip_value_with_tag(bc: &mut BcReader, tag: u8) -> Option<()> {
    match tag {
        TAG_NULL | TAG_FALSE | TAG_TRUE => Some(()),
        TAG_I64 | TAG_F64 => {
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
            let len = bc.read_u32()? as usize;
            for _ in 0..len {
                bc.skip_value()?;
            }
            Some(())
        }
        TAG_OBJECT | TAG_OBJECT_VARIANT => {
            if tag == TAG_OBJECT_VARIANT {
                bc.read_u8()?;
            }
            let len = bc.read_u32()? as usize;
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
fn read_value_from_bc(bc: &mut BcReader) -> Option<serde_json::Value> {
    let tag = bc.read_u8()?;
    match tag {
        TAG_NULL => Some(serde_json::Value::Null),
        TAG_FALSE => Some(serde_json::Value::Bool(false)),
        TAG_TRUE => Some(serde_json::Value::Bool(true)),
        TAG_I64 => {
            let v = bc.i64()?;
            Some(serde_json::Value::from(v))
        }
        TAG_F64 => {
            if bc.cursor.len() < 8 {
                return None;
            }
            let v = f64::from_le_bytes(bc.cursor[..8].try_into().ok()?);
            bc.cursor = &bc.cursor[8..];
            Some(serde_json::Value::from(v))
        }
        TAG_STR => {
            let idx = bc.read_idx()?;
            if idx >= STRINGS.len() {
                return None;
            }
            Some(serde_json::Value::String(STRINGS[idx].to_string()))
        }
        TAG_ARRAY => {
            let len = bc.read_u32()? as usize;
            let mut arr = Vec::with_capacity(len);
            for _ in 0..len {
                arr.push(read_value_from_bc(bc)?);
            }
            Some(serde_json::Value::Array(arr))
        }
        TAG_OBJECT | TAG_OBJECT_VARIANT => {
            if tag == TAG_OBJECT_VARIANT {
                bc.read_u8()?; // skip variant tag byte
            }
            let len = bc.read_u32()? as usize;
            let mut obj = serde_json::Map::with_capacity(len);
            for _ in 0..len {
                let kidx = bc.read_idx()?;
                if kidx >= STRINGS.len() {
                    return None;
                }
                let key = STRINGS[kidx].to_string();
                let val = read_value_from_bc(bc)?;
                obj.insert(key, val);
            }
            Some(serde_json::Value::Object(obj))
        }
        _ => None,
    }
}

// ── Direct Ability decoder ──

fn decode_ability(bc: &mut BcReader) -> Option<Ability> {
    let tag = bc.read_u8()?;
    if tag != TAG_OBJECT {
        return None;
    }
    let count = bc.read_u32()? as usize;
    #[cfg(feature = "ds_debug")]
    ds_print(&alloc::format!("DA:c={}", count));

    let mut full_text = String::new();
    let mut triggerless_text: Option<String> = None;
    let mut triggers: Option<ArcStr> = None;
    let mut use_limit: Option<u8> = None;
    let mut is_null = false;
    let mut cost: Option<Box<AbilityCost>> = None;
    let mut effect: Option<Box<AbilityEffect>> = None;
    let mut keywords: Option<Vec<crate::card::Keyword>> = None;

    #[cfg(feature = "ds_debug")]
    ds_print("DA:LOOP");
    for _i in 0..count {
        #[cfg(feature = "ds_debug")]
        if i % 10 == 0 {
            ds_print(&alloc::format!("DA:i={}", i));
        }
        let key = bc.key()?;
        match key {
            "full_text" => {
                full_text = bc.read_string_value()?;
                #[cfg(feature = "ds_debug")]
                ds_print("DA:ft");
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
                #[cfg(feature = "ds_debug")]
                ds_print("DA:cost");
            }
            "effect" => {
                effect = decode_ability_effect(bc)?.map(Box::new);
                #[cfg(feature = "ds_debug")]
                ds_print("DA:eff");
            }
            "keywords" => {
                keywords = decode_keywords(bc)?;
            }
            _ => {
                bc.skip_value()?;
            }
        }
    }
    #[cfg(feature = "ds_debug")]
    ds_print("DA:OK");

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
        TAG_OBJECT => {
            let count = bc.read_u32()? as usize;
            // Collect into map, then apply AbilityCost-specific normalizations.
            let mut map = collect_json_map(bc, count)?;
            normalize_cost_keys(&mut map);
            let map_val = serde_json::Value::Object(map);
            let map_clone = map_val.clone();
            let mut inner: AbilityEffect = serde_json::from_value(map_val).ok()?;
            inner.populate_from_json(&map_clone);
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
        TAG_OBJECT => {
            let inner = decode_ability_effect_from_object(bc)?;
            Some(Some(inner))
        }
        _ => None,
    }
}

/// Collect `count` key-value pairs from a BcReader into a serde_json::Map.
fn collect_json_map(
    bc: &mut BcReader,
    count: usize,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let mut map = serde_json::Map::with_capacity(count);
    for _ in 0..count {
        let key_idx = bc.read_idx()?;
        if key_idx >= STRINGS.len() {
            return None;
        }
        let key = STRINGS[key_idx].to_string();
        let val = bc.read_json_value()?;
        map.insert(key, val);
    }
    Some(map)
}

/// Apply AbilityCost-specific key normalizations recursively.
/// The bytecode stores the exact JSON keys from abilities.json. AbilityCost
/// objects use "type" (not "action"), "zone" (not "source"), and
/// "options"/"costs" (not "actions") — because the legacy AbilityCost
/// custom Deserialize handles these aliases.
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

/// Decode an AbilityEffect from an object whose TAG_OBJECT + count have been
/// consumed. Strategy:
///   1. Collect every key→value pair into a serde_json::Map
///   2. Deserialize AbilityEffect from the Map via serde (handles all fields
///      including `#[serde(flatten)]` CompoundBranch)
///   3. Call `populate_from_json` to build EffectKind and recurse into sub-effects
fn decode_ability_effect_from_object(bc: &mut BcReader) -> Option<AbilityEffect> {
    let count = bc.read_u32()? as usize;

    // Phase 1: collect all key-value pairs into a JSON map.
    let mut map = collect_json_map(bc, count)?;

    // Normalize legacy / alias keys so AbilityEffect::Deserialize picks them up:
    //   "type" / "cost_type" → "action"   (AbilityCost uses "type")
    //   "zone"               → "source"   (AbilityCost uses "zone")
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

    let map_val = serde_json::Value::Object(map);
    let map_clone = map_val.clone();

    // Phase 2: deserialize AbilityEffect via serde.
    // This handles text, action, source, destination, count, target, condition,
    // compound (look_action, select_action, actions, etc.), and all other fields
    // including #[serde(flatten)] CompoundBranch.
    let mut effect: AbilityEffect = serde_json::from_value(map_val).ok()?;

    // Phase 3: populate EffectKind and recurse into sub-effects.
    effect.populate_from_json(&map_clone);

    // Phase 4: draw-count fix (mirror decode_like_json)
    if let Some(ref actions) = effect.compound.actions {
        let fixed: Vec<Box<AbilityEffect>> = actions
            .iter()
            .map(|a| {
                let mut f = (**a).clone();
                if (f.action == ActionType::DrawCard)
                    && f.count.is_none()
                    && f.dynamic_count_any().is_none()
                {
                    f.count = Some(1);
                }
                Box::new(f)
            })
            .collect();
        effect.compound.actions = Some(fixed);
    }

    Some(effect)
}

/// Direct decoder for TAG_OBJECT_VARIANT effects using the generated field dispatch.
fn decode_ability_effect_direct(bc: &mut BcReader, variant: u8) -> Option<AbilityEffect> {
    let count = bc.read_u32()? as usize;
    let mut text = ArcStr::from("");
    let mut action = ActionType::default();
    let mut source: Option<ArcStr> = None;
    let mut destination: Option<ArcStr> = None;
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

    let kind = Some(ek_box_new(match variant {
        1 => build_movecards(&ek),
        2 => build_drawcards(&ek),
        3 => build_selecttarget(&ek),
        4 => build_lookreveal(&ek),
        5 => build_modifyscore(&ek),
        6 => build_modifyhearts(&ek),
        7 => build_gainresource(&ek),
        8 => build_changestate(&ek),
        9 => build_abilityop(&ek),
        10 => build_compoundeffect(&ek),
        11 => build_restrictionop(&ek),
        12 => build_positionop(&ek),
        13 => build_miscop(&ek),
        14 => build_customop(&ek),
        _ => return None,
    }));

    let mut effect = AbilityEffect {
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

    // Draw-count fix
    if let Some(ref actions) = effect.compound.actions {
        let fixed: Vec<Box<AbilityEffect>> = actions
            .iter()
            .map(|a| {
                let mut f = (**a).clone();
                if f.action == ActionType::DrawCard
                    && f.count.is_none()
                    && f.dynamic_count_any().is_none()
                {
                    f.count = Some(1);
                }
                Box::new(f)
            })
            .collect();
        effect.compound.actions = Some(fixed);
    }

    Some(effect)
}

// ── populate_from_json (kept for JSON fallback / tests) ──

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
        match self.kind.as_deref_mut() {
            Some(EffectKind::LookReveal {
                ref mut options,
                ref mut resource_on_select,
                ..
            }) => {
                if let Some(ref mut opts) = options {
                    if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                        for (i, opt) in opts.iter_mut().enumerate() {
                            if i < json_opts.len() {
                                opt.populate_from_json(&json_opts[i]);
                            }
                        }
                    }
                }
                if let Some(ref mut ros) = resource_on_select {
                    if let Some(ros_json) = json_val.get("resource_on_select") {
                        ros.populate_from_json(ros_json);
                    }
                }
            }
            Some(EffectKind::CompoundEffect {
                ref mut options,
                ref mut alternative_effect,
                ..
            }) => {
                if let Some(ref mut opts) = options {
                    if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                        for (i, opt) in opts.iter_mut().enumerate() {
                            if i < json_opts.len() {
                                opt.populate_from_json(&json_opts[i]);
                            }
                        }
                    }
                }
                if let Some(ref mut ae) = alternative_effect {
                    if let Some(ae_json) = json_val.get("alternative_effect") {
                        ae.populate_from_json(ae_json);
                    }
                }
            }
            Some(EffectKind::AbilityOp {
                ref mut gained_effect,
                ..
            }) => {
                if let Some(ref mut ge) = gained_effect {
                    if let Some(ge_json) = json_val.get("gained_effect") {
                        ge.populate_from_json(ge_json);
                    }
                }
            }
            Some(EffectKind::CustomOp {
                ref mut opponent_action,
                ..
            }) => {
                if let Some(ref mut oa) = opponent_action {
                    if let Some(oa_json) = json_val.get("opponent_action") {
                        oa.populate_from_json(oa_json);
                    }
                }
            }
            Some(EffectKind::MiscOp {
                ref mut options, ..
            })
            | Some(EffectKind::SelectTarget {
                ref mut options, ..
            }) => {
                if let Some(ref mut opts) = options {
                    if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                        for (i, opt) in opts.iter_mut().enumerate() {
                            if i < json_opts.len() {
                                opt.populate_from_json(&json_opts[i]);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

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
        ref mut conditions,
        ref mut operator,
        ..
    } = cond
    {
        if operator.is_none()
            && cond_json.get("type").and_then(|t| t.as_str()) == Some("or_condition")
        {
            *operator = Some("or".into());
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
            let len = bc.read_u32()? as usize;
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                let s = bc.read_string_value()?;
                v.push(keyword_from_str(&s));
            }
            Some(Some(v))
        }
        _ => None,
    }
}

fn keyword_from_str(s: &str) -> crate::card::Keyword {
    match s {
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
        _ => crate::card::Keyword::Turn1,
    }
}
