use super::abilities_gen::{BYTECODE, NUM_ABILITIES, OFFSETS, STRINGS};
use crate::ability::enums::ActionType;
use crate::card::{Ability, AbilityCost, AbilityEffect};
use crate::core::types::ArcStr;
use serde::de::IntoDeserializer;

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

    fn remaining(&self) -> &'a [u8] {
        self.cursor
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

// ── Serde binary deserializer ──

#[derive(Debug)]
struct BcError(String);

impl serde::de::Error for BcError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        BcError(msg.to_string())
    }
}

impl std::fmt::Display for BcError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BcError {}

struct BcDeserializer<'de> {
    slice: &'de [u8],
}

impl<'de, 'a> serde::Deserializer<'de> for &'a mut BcDeserializer<'de> {
    type Error = BcError;

    fn deserialize_any<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, BcError> {
        if self.slice.is_empty() {
            return Err(BcError("unexpected end of input".into()));
        }
        let tag = self.slice[0];
        self.slice = &self.slice[1..];
        match tag {
            TAG_NULL => visitor.visit_unit(),
            TAG_FALSE => visitor.visit_bool(false),
            TAG_TRUE => visitor.visit_bool(true),
            TAG_I64 => {
                if self.slice.len() < 8 {
                    return Err(BcError("unexpected end of input for i64".into()));
                }
                let v = i64::from_le_bytes(self.slice[..8].try_into().unwrap());
                self.slice = &self.slice[8..];
                visitor.visit_i64(v)
            }
            TAG_F64 => {
                if self.slice.len() < 8 {
                    return Err(BcError("unexpected end of input for f64".into()));
                }
                let v = f64::from_le_bytes(self.slice[..8].try_into().unwrap());
                self.slice = &self.slice[8..];
                visitor.visit_f64(v)
            }
            TAG_STR => {
                if self.slice.len() < 2 {
                    return Err(BcError("unexpected end of input for string index".into()));
                }
                let idx = u16::from_le_bytes([self.slice[0], self.slice[1]]) as usize;
                self.slice = &self.slice[2..];
                if idx >= STRINGS.len() {
                    return Err(BcError(format!("string index {idx} out of range")));
                }
                visitor.visit_borrowed_str(STRINGS[idx])
            }
            TAG_ARRAY => {
                if self.slice.len() < 4 {
                    return Err(BcError("unexpected end of input for array length".into()));
                }
                let len = u32::from_le_bytes(self.slice[..4].try_into().unwrap()) as usize;
                self.slice = &self.slice[4..];
                visitor.visit_seq(BcSeqAccess {
                    deser: self,
                    remaining: len,
                })
            }
            TAG_OBJECT => {
                if self.slice.len() < 4 {
                    return Err(BcError("unexpected end of input for object length".into()));
                }
                let len = u32::from_le_bytes(self.slice[..4].try_into().unwrap()) as usize;
                self.slice = &self.slice[4..];
                visitor.visit_map(BcMapAccess {
                    deser: self,
                    remaining: len,
                })
            }
            _ => Err(BcError(format!("unknown tag: {tag:#04x}"))),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any struct
    }
}

struct BcSeqAccess<'de, 'a> {
    deser: &'a mut BcDeserializer<'de>,
    remaining: usize,
}

impl<'de, 'a> serde::de::SeqAccess<'de> for BcSeqAccess<'de, 'a> {
    type Error = BcError;

    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, BcError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.deser).map(Some)
    }
}

struct BcMapAccess<'de, 'a> {
    deser: &'a mut BcDeserializer<'de>,
    remaining: usize,
}

impl<'de, 'a> serde::de::MapAccess<'de> for BcMapAccess<'de, 'a> {
    type Error = BcError;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, BcError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        if self.deser.slice.len() < 2 {
            return Err(BcError("unexpected end of input for key index".into()));
        }
        let idx = u16::from_le_bytes([self.deser.slice[0], self.deser.slice[1]]) as usize;
        self.deser.slice = &self.deser.slice[2..];
        if idx >= STRINGS.len() {
            return Err(BcError(format!("key string index {idx} out of range")));
        }
        seed.deserialize(STRINGS[idx].into_deserializer()).map(Some)
    }

    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, BcError> {
        seed.deserialize(&mut *self.deser)
    }
}

fn from_bytes<'de, T: serde::Deserialize<'de>>(data: &'de [u8]) -> Result<T, BcError> {
    let mut deser = BcDeserializer { slice: data };
    T::deserialize(&mut deser)
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

            let start = bc.remaining();
            for _ in 0..count {
                bc.u16()?;
                bc.skip_value()?;
            }
            let consumed = start.len() - bc.remaining().len();
            let obj_bytes = &start[..consumed];

            let mut map_val: serde_json::Value = from_bytes(obj_bytes).ok()?;
            if let Some(obj) = map_val.as_object_mut() {
                normalize_cost_keys(obj);
            }

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
///   1. Capture the byte range of the object body
///   2. Deserialize serde_json::Value directly from binary via BcDeserializer
///   3. Normalize legacy / alias keys
///   4. Deserialize AbilityEffect from the normalized JSON via serde
///   5. Call `populate_from_json` to build EffectKind and recurse into sub-effects
fn decode_ability_effect_from_object(bc: &mut BcReader) -> Option<AbilityEffect> {
    let count = bc.u32()? as usize;

    let start = bc.remaining();
    for _ in 0..count {
        bc.u16()?;
        bc.skip_value()?;
    }
    let consumed = start.len() - bc.remaining().len();
    let obj_bytes = &start[..consumed];

    let mut map_val: serde_json::Value = from_bytes(obj_bytes).ok()?;

    if let Some(obj) = map_val.as_object_mut() {
        if !obj.contains_key("action") {
            if let Some(v) = obj.remove("type").or_else(|| obj.remove("cost_type")) {
                obj.insert("action".into(), v);
            }
        }
        if !obj.contains_key("source") {
            if let Some(v) = obj.remove("zone") {
                obj.insert("source".into(), v);
            }
        }
    }

    let mut effect: AbilityEffect = serde_json::from_value(map_val.clone()).ok()?;

    effect.populate_from_json(&map_val);

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
