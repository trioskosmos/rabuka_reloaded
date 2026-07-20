use super::abilities_gen::{BYTECODE, NUM_ABILITIES, OFFSETS, STRINGS};
use crate::ability::enums::ActionType;
use crate::card::{Ability, AbilityEffect};

/// Decode a single `unique_abilities[idx]` entry from the bundled bytecode.
///
/// The bytecode stores each ability as a compact *binary JSON* slice (tagged
/// tree with interned strings — see `compile_abilities.py`). `get_ability`
/// reconstructs the **exact same** `serde_json::Value` the text loader would
/// produce, then runs the identical post-processing the default JSON loader
/// uses (`from_value::<Ability>` + `populate_from_json` + draw-count fix).
///
/// Because the decode is generic over JSON shape and feeds the SAME serde +
/// `populate_from_json` path as the default loader, adding a new action type or
/// field requires **zero** decoder changes — the bytecode path is fully
/// data-driven and guaranteed to stay in lock-step with the JSON path (guarded
/// by `bytecode_deep_compare_test`).
pub fn ability_count() -> usize {
    NUM_ABILITIES
}

pub fn get_ability(idx: usize) -> Option<Ability> {
    if idx >= NUM_ABILITIES {
        return None;
    }
    let start = OFFSETS[idx] as usize;
    let end = OFFSETS[idx + 1] as usize;
    if start >= end {
        return Some(Ability::default());
    }
    let slice = &BYTECODE[start..end];

    let mut cursor = slice;
    let value = match read_value(&mut cursor) {
        Some(v) => v,
        None => {
            log::error!("bytecode: failed to decode ability {idx}");
            return None;
        }
    };

    // Mirror `CardLoader::build_abilities_map_inner` (default path) exactly.
    // `value` is owned (no clone of the whole tree) — we consume it directly.
    decode_like_json(value)
}

/// Reconstruct an `Ability` from a `serde_json::Value` using the same logic the
/// default JSON loader applies. `entry` is consumed by value (no clone of the
/// whole tree) to keep peak heap at load low.
pub(crate) fn decode_like_json(entry: serde_json::Value) -> Option<Ability> {
    let effect_json = entry.get("effect").cloned();
    let mut ab: Ability = serde_json::from_value::<Ability>(entry).ok()?;
    if let Some(ref mut effect) = ab.effect {
        if let Some(ref json_effect) = effect_json {
            effect.populate_from_json(&json_effect);
        }
    }
    if let Some(ref mut effect) = ab.effect {
        if let Some(ref actions) = effect.compound.actions.clone() {
            let fixed_actions: Vec<Box<AbilityEffect>> = actions
                .iter()
                .map(|action| {
                    let mut fixed_action = action.clone();
                    if (fixed_action.action == ActionType::Draw
                        || fixed_action.action == ActionType::DrawCard)
                        && fixed_action.count.is_none()
                        && fixed_action.dynamic_count_any().is_none()
                    {
                        fixed_action.count = Some(1);
                    }
                    fixed_action
                })
                .collect();
            effect.compound.actions = Some(fixed_actions);
        }
    }
    Some(ab)
}

// ── Binary JSON decoder ──
// Tag byte followed by payload. Mirrors the encoder in compile_abilities.py.
const TAG_NULL: u8 = 0x00;
const TAG_FALSE: u8 = 0x01;
const TAG_TRUE: u8 = 0x02;
const TAG_I64: u8 = 0x03;
const TAG_F64: u8 = 0x04;
const TAG_STR: u8 = 0x06;
const TAG_ARRAY: u8 = 0x07;
const TAG_OBJECT: u8 = 0x08;

fn read_value(c: &mut &[u8]) -> Option<serde_json::Value> {
    if c.is_empty() {
        return None;
    }
    let tag = read_u8(c)?;
    match tag {
        TAG_NULL => Some(serde_json::Value::Null),
        TAG_FALSE => Some(serde_json::Value::Bool(false)),
        TAG_TRUE => Some(serde_json::Value::Bool(true)),
        TAG_I64 => {
            let v = read_i64(c)?;
            Some(serde_json::Value::from(v))
        }
        TAG_F64 => {
            let v = read_f64(c)?;
            Some(serde_json::Value::from(v))
        }
        TAG_STR => {
            let idx = read_u16(c)? as usize;
            if idx >= STRINGS.len() {
                return None;
            }
            Some(serde_json::Value::String(STRINGS[idx].to_string()))
        }
        TAG_ARRAY => {
            let len = read_u32(c)? as usize;
            let mut arr = Vec::with_capacity(len);
            for _ in 0..len {
                arr.push(read_value(c)?);
            }
            Some(serde_json::Value::Array(arr))
        }
        TAG_OBJECT => {
            let len = read_u32(c)? as usize;
            let mut obj = serde_json::Map::with_capacity(len);
            for _ in 0..len {
                let kidx = read_u16(c)? as usize;
                if kidx >= STRINGS.len() {
                    return None;
                }
                let key = STRINGS[kidx].to_string();
                let val = read_value(c)?;
                obj.insert(key, val);
            }
            Some(serde_json::Value::Object(obj))
        }
        _ => None,
    }
}

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

fn read_f64(c: &mut &[u8]) -> Option<f64> {
    if c.len() < 8 {
        return None;
    }
    let v = f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
    *c = &c[8..];
    Some(v)
}
