use super::abilities_gen::{BYTECODE, NUM_ABILITIES, OFFSETS, STRINGS};
use crate::ability::enums::ActionType;
use crate::card::{Ability, AbilityCost, AbilityEffect};
use crate::core::types::ArcStr;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, string::ToString, vec, vec::Vec};

const TAG_NULL: u8 = 0x00;
const TAG_FALSE: u8 = 0x01;
const TAG_TRUE: u8 = 0x02;
const TAG_I64: u8 = 0x03;
const TAG_F64: u8 = 0x04;
const TAG_STR: u8 = 0x06;
const TAG_ARRAY: u8 = 0x07;
const TAG_OBJECT: u8 = 0x08;

pub fn ability_count() -> usize {
    NUM_ABILITIES
}

/// Decode a single ability from the bytecode blob.
///
/// This is called eagerly at load time for ALL 800 abilities by
/// `card_loader::build_abilities_map_inner`. Each call decodes the
/// bytecode slice into a full `Ability` struct (with nested AbilityEffect,
/// EffectKind, Condition, etc.) — ~3.5KB per ability.
///
/// # TODO: Lazy decode path (150KB target)
/// For console targets, this function should be called on-demand (only
/// when an ability is first triggered), not eagerly at load time. The
/// decoded Ability would be cached in a bounded HashMap<u16, Arc<Ability>>
/// with LRU eviction. The `decode_ability` path is already fast (~50μs)
/// so lazy decode adds negligible latency on first trigger.
///
/// # TODO: Bytecode interpreter (alternative to lazy decode)
/// Instead of decoding bytecode → Ability struct → execute, evaluate
/// bytecode directly via opcode dispatch. This avoids materializing
/// Ability/AbilityEffect/EffectKind structs entirely. Requires a ~500-line
/// new module that mirrors the existing handler dispatch but reads fields
/// from bytecode bytes instead of struct fields.
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
    let mut bc = BcReader::new(slice);
    decode_ability(&mut bc).or_else(|| {
        log::error!(
            "bytecode: direct decoder failed for ability {idx}, falling back to serde path"
        );
        let mut cursor = slice;
        let value = read_value(&mut cursor)?;
        decode_like_json(value)
    })
}

// ── BcReader ──

struct BcReader<'a> {
    cursor: &'a [u8],
}

impl<'a> BcReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BcReader { cursor: data }
    }

    fn u8(&mut self) -> Option<u8> {
        read_u8(&mut self.cursor)
    }

    fn u16(&mut self) -> Option<u16> {
        read_u16(&mut self.cursor)
    }

    fn u32(&mut self) -> Option<u32> {
        read_u32(&mut self.cursor)
    }

    fn i64(&mut self) -> Option<i64> {
        read_i64(&mut self.cursor)
    }

    fn key(&mut self) -> Option<&'a str> {
        let idx = self.u16()? as usize;
        if idx >= STRINGS.len() {
            return None;
        }
        Some(STRINGS[idx])
    }

    fn skip_value(&mut self) -> Option<()> {
        let tag = self.u8()?;
        skip_value_with_tag(self, tag)
    }

    fn read_string_value(&mut self) -> Option<String> {
        let tag = self.u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.u16()? as usize;
                if idx >= STRINGS.len() {
                    return None;
                }
                Some(STRINGS[idx].to_string())
            }
            _ => None,
        }
    }

    fn read_arc_str_value(&mut self) -> Option<ArcStr> {
        let tag = self.u8()?;
        match tag {
            TAG_NULL => None,
            TAG_STR => {
                let idx = self.u16()? as usize;
                if idx >= STRINGS.len() {
                    return None;
                }
                Some(ArcStr::from(STRINGS[idx]))
            }
            _ => None,
        }
    }

    fn read_bool_value(&mut self) -> Option<bool> {
        let tag = self.u8()?;
        match tag {
            TAG_NULL => None,
            TAG_TRUE => Some(true),
            TAG_FALSE => Some(false),
            _ => None,
        }
    }

    fn read_u32_value(&mut self) -> Option<u32> {
        let tag = self.u8()?;
        match tag {
            TAG_NULL => None,
            TAG_I64 => Some(self.i64()? as u32),
            _ => None,
        }
    }

    fn read_json_value(&mut self) -> Option<serde_json::Value> {
        read_value_from_bc(self)
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
            bc.u16()?;
            Some(())
        }
        TAG_ARRAY => {
            let len = bc.u32()? as usize;
            for _ in 0..len {
                bc.skip_value()?;
            }
            Some(())
        }
        TAG_OBJECT => {
            let len = bc.u32()? as usize;
            for _ in 0..len {
                bc.u16()?; // key index
                bc.skip_value()?;
            }
            Some(())
        }
        _ => None,
    }
}

/// Read a tagged value from a BcReader into a serde_json::Value.
fn read_value_from_bc(bc: &mut BcReader) -> Option<serde_json::Value> {
    let tag = bc.u8()?;
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
            let idx = bc.u16()? as usize;
            if idx >= STRINGS.len() {
                return None;
            }
            Some(serde_json::Value::String(STRINGS[idx].to_string()))
        }
        TAG_ARRAY => {
            let len = bc.u32()? as usize;
            let mut arr = Vec::with_capacity(len);
            for _ in 0..len {
                arr.push(read_value_from_bc(bc)?);
            }
            Some(serde_json::Value::Array(arr))
        }
        TAG_OBJECT => {
            let len = bc.u32()? as usize;
            let mut obj = serde_json::Map::with_capacity(len);
            for _ in 0..len {
                let kidx = bc.u16()? as usize;
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
    let tag = bc.u8()?;
    if tag != TAG_OBJECT {
        return None;
    }
    let count = bc.u32()? as usize;

    let mut full_text = String::new();
    let mut triggerless_text: Option<String> = None;
    let mut triggers: Option<ArcStr> = None;
    let mut use_limit: Option<u32> = None;
    let mut is_null = false;
    let mut cost: Option<Box<AbilityCost>> = None;
    let mut effect: Option<Box<AbilityEffect>> = None;
    let mut keywords: Option<Vec<crate::card::Keyword>> = None;

    for _ in 0..count {
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
    let tag = bc.u8()?;
    match tag {
        TAG_NULL => Some(None),
        TAG_OBJECT => {
            let count = bc.u32()? as usize;
            // Collect into map, then apply AbilityCost-specific normalizations.
            let mut map = collect_json_map(bc, count)?;
            normalize_cost_keys(&mut map);
            let map_val = serde_json::Value::Object(map);
            let mut inner: AbilityEffect = serde_json::from_value(map_val.clone()).ok()?;
            inner.populate_from_json(&map_val);
            Some(Some(Box::new(AbilityCost(inner))))
        }
        _ => None,
    }
}

fn decode_ability_effect(bc: &mut BcReader) -> Option<Option<AbilityEffect>> {
    let tag = bc.u8()?;
    match tag {
        TAG_NULL => Some(None),
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
        let key_idx = bc.u16()? as usize;
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
fn normalize_cost_keys(map: &mut serde_json::Map<String, serde_json::Value>) {
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
    let count = bc.u32()? as usize;

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

    // Phase 2: deserialize AbilityEffect via serde.
    // This handles text, action, source, destination, count, target, condition,
    // compound (look_action, select_action, actions, etc.), and all other fields
    // including #[serde(flatten)] CompoundBranch.
    let mut effect: AbilityEffect = serde_json::from_value(map_val.clone()).ok()?;

    // Phase 3: populate EffectKind and recurse into sub-effects.
    effect.populate_from_json(&map_val);

    // Phase 4: draw-count fix (mirror decode_like_json)
    if let Some(ref actions) = effect.compound.actions {
        let fixed: Vec<Box<AbilityEffect>> = actions
            .iter()
            .map(|a| {
                let mut f = (**a).clone();
                if (f.action == ActionType::Draw || f.action == ActionType::DrawCard)
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

// ── Keyword decoder ──

fn decode_keywords(bc: &mut BcReader) -> Option<Option<Vec<crate::card::Keyword>>> {
    let tag = bc.u8()?;
    match tag {
        TAG_NULL => Some(None),
        TAG_ARRAY => {
            let len = bc.u32()? as usize;
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

// ── Legacy JSON-path decoder (kept for bytecode_deep_compare_test) ──

pub(crate) fn decode_like_json(entry: serde_json::Value) -> Option<Ability> {
    let effect_json = entry.get("effect").cloned();
    let mut ab: Ability = serde_json::from_value::<Ability>(entry).ok()?;
    if let Some(ref mut effect) = ab.effect {
        if let Some(ref json_effect) = effect_json {
            effect.populate_from_json(json_effect);
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

// ── Legacy read_value / read_* helpers (kept for decode_like_json) ──

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
