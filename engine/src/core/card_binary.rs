use super::card::{BaseHeart, BladeHeart, Card, CardType, HeartColor, HeartMap, SpecialHeart};
#[cfg(not(feature = "external_card_data"))]
use super::cards_gen::{CARD_BLOB, CARD_STRINGS};

// PS1: the card blob is loaded from the CD at runtime (2MB RAM can't hold it
// baked). The platform fills these before decoding; the embedded const is not
// referenced on external-card-data builds so it is dead-stripped.
#[cfg(feature = "external_card_data")]
pub static mut EXTERN_CARD_BLOB: *const u8 = core::ptr::null();
#[cfg(feature = "external_card_data")]
pub static mut EXTERN_CARD_BLOB_LEN: usize = 0;

/// The active card blob: the embedded const, or the runtime-loaded buffer.
#[inline]
fn blob() -> &'static [u8] {
    #[cfg(feature = "external_card_data")]
    unsafe {
        if !EXTERN_CARD_BLOB.is_null() {
            return core::slice::from_raw_parts(EXTERN_CARD_BLOB, EXTERN_CARD_BLOB_LEN);
        }
        return &[];
    }
    #[cfg(not(feature = "external_card_data"))]
    {
        CARD_BLOB
    }
}
use crate::core::types::ArcStr;
use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};

const MAGIC: &[u8; 4] = b"CARD";

/// Parse the CARD_BLOB header and return (num_cards, strtab_len, strtab_start, length_start, data_start).
/// Header: magic(4) + num_cards(u16) + strtab_len(u32). Then strtab, then a u8
/// per-card length table, then card data. Card starts are prefix sums of lengths.
fn parse_header() -> Option<(u32, u32, usize, usize, usize)> {
    if blob().len() < 10 || &blob()[0..4] != MAGIC {
        return None;
    }
    let num_cards = u16::from_le_bytes(blob()[4..6].try_into().ok()?) as u32;
    let strtab_len = u32::from_le_bytes(blob()[6..10].try_into().ok()?);
    let strtab_start = 10;
    let length_start = strtab_start + strtab_len as usize;
    let data_start = length_start + num_cards as usize;
    Some((
        num_cards,
        strtab_len,
        strtab_start,
        length_start,
        data_start,
    ))
}

/// Get the byte offset of card `idx`'s data within CARD_BLOB.
/// O(n) prefix-sum over the u8 length table — cheap (cards are 25-41 bytes).
fn card_data_offset(idx: usize) -> Option<usize> {
    let (num_cards, _strtab_len, _strtab_start, length_start, data_start) = parse_header()?;
    if idx >= num_cards as usize {
        return None;
    }
    let lengths = &blob()[length_start..length_start + num_cards as usize];
    let mut start = 0usize;
    for &len in &lengths[..idx] {
        start += len as usize;
    }
    Some(data_start + start)
}

/// Find a string's index in the blob strtab (external mode).
#[cfg(feature = "external_card_data")]
fn find_string_index_by_no(card_no: &str) -> Option<usize> {
    let b = blob();
    let (_num, strtab_len, strtab_start, _, _) = parse_header()?;
    let strtab_end = strtab_start + strtab_len as usize;
    let mut pos = strtab_start;
    let mut idx = 0usize;
    while pos + 2 <= strtab_end {
        let len = u16::from_le_bytes([b[pos], b[pos + 1]]) as usize;
        pos += 2;
        let end = (pos + len).min(strtab_end);
        if core::str::from_utf8(&b[pos..end]).ok() == Some(card_no) {
            return Some(idx);
        }
        pos = end;
        idx += 1;
    }
    None
}

/// Decode a single card from a raw record using a string resolver.
/// The record is a CARD-format card record; `get_str` resolves strtab indices.
fn decode_card_from_record(rec: &[u8], strtab: &[u8]) -> Option<Card> {
    let data = rec;
    let get_str = |idx: u16| get_str_from_strtab(strtab, idx);

    if data.len() < 20 {
        return None;
    }

    let card_no_idx = u16::from_le_bytes(data[0..2].try_into().ok()?);
    let name_idx = u16::from_le_bytes(data[2..4].try_into().ok()?);
    let series_idx = u16::from_le_bytes(data[4..6].try_into().ok()?);
    let group_idx = u16::from_le_bytes(data[6..8].try_into().ok()?);
    let unit_idx = u16::from_le_bytes(data[8..10].try_into().ok()?);
    #[cfg(not(feature = "compact_cards"))]
    let img_idx = u16::from_le_bytes(data[10..12].try_into().ok()?);
    #[cfg(feature = "compact_cards")]
    let _img_idx = u16::from_le_bytes(data[10..12].try_into().ok()?);
    #[cfg(not(feature = "compact_cards"))]
    let product_idx = u16::from_le_bytes(data[12..14].try_into().ok()?);
    #[cfg(feature = "compact_cards")]
    let _product_idx = u16::from_le_bytes(data[12..14].try_into().ok()?);
    #[cfg(not(feature = "compact_cards"))]
    let rare_idx = u16::from_le_bytes(data[14..16].try_into().ok()?);
    #[cfg(feature = "compact_cards")]
    let _rare_idx = u16::from_le_bytes(data[14..16].try_into().ok()?);
    #[cfg(not(feature = "compact_cards"))]
    let ability_idx = u16::from_le_bytes(data[16..18].try_into().ok()?);
    #[cfg(feature = "compact_cards")]
    let _ability_idx = u16::from_le_bytes(data[16..18].try_into().ok()?);
    let type_flags = data[18];
    let cost_val = data[19];
    let blade_val = data[20];
    let score_val = data[21];
    let num_base = data[22] as usize;
    let num_blade = data[23] as usize;
    let num_need = data[24] as usize;
    let ctype = type_flags & 0x03;
    let has_special = (type_flags >> 2) & 0x01;
    // Presence bits: cost and score may legitimately be 0, so a separate flag
    // distinguishes Some(0) from None.
    let has_cost = (type_flags >> 3) & 0x01;
    let has_score = (type_flags >> 4) & 0x01;

    let card_type = match ctype {
        0 => CardType::Member,
        1 => CardType::Live,
        2 => CardType::Energy,
        _ => CardType::Member,
    };

    let card_no: ArcStr = get_str(card_no_idx).into();
    let name: ArcStr = get_str(name_idx).into();

    let series: Box<str> = get_str(series_idx).into();
    let group: Box<str> = {
        let g = get_str(group_idx);
        if g.is_empty() {
            // Mirrors Card::deserialize's map_series_to_group: a multi-line series
            // does not map to a single group, so group stays empty.
            match series.as_ref() {
                "ラブライブ！" => "μ's".into(),
                "ラブライブ！サンシャイン!!" => "Aqours".into(),
                "ラブライブ！虹ヶ咲学園スクールアイドル同好会" => {
                    "虹ヶ咲".into()
                }
                "ラブライブ！スーパースター!!" => "Liella!".into(),
                "蓮ノ空女学院スクールアイドルクラブ"
                | "ラブライブ！蓮ノ空女学院スクールアイドルクラブ" => {
                    "蓮ノ空".into()
                }
                _ => Box::from(""),
            }
        } else {
            g.into()
        }
    };
    let unit: Option<ArcStr> = {
        let u = get_str(unit_idx);
        if u.is_empty() {
            None
        } else {
            Some(u.into())
        }
    };

    // Parse hearts
    let mut pos = 25;
    let base_heart = parse_hearts(&data[pos..], num_base);
    pos += num_base * 2;
    let blade_heart = parse_hearts(&data[pos..], num_blade);
    pos += num_blade * 2;
    let need_heart = parse_hearts(&data[pos..], num_need);
    pos += num_need * 2;

    let special_heart = if has_special != 0 && pos + 2 <= data.len() {
        let sc = data[pos];
        let scount = data[pos + 1];
        if scount > 0 {
            let mut hearts = HeartMap::new();
            hearts.insert(color_from_u8(sc), scount as u8);
            Some(SpecialHeart { hearts })
        } else {
            None
        }
    } else {
        None
    };

    Some(Card {
        card_no,
        name,
        #[cfg(not(feature = "compact_cards"))]
        img: if img_idx != 0 {
            Some(get_str(img_idx).into())
        } else {
            None
        },
        series,
        group,
        card_type,
        unit,
        cost: if has_cost != 0 {
            Some(cost_val as u8)
        } else {
            None
        },
        blade: blade_val as u8,
        score: if has_score != 0 {
            Some(score_val as u8)
        } else {
            None
        },
        base_heart: if base_heart.is_empty() {
            None
        } else {
            Some(BaseHeart { hearts: base_heart })
        },
        blade_heart: if blade_heart.is_empty() {
            None
        } else {
            Some(BladeHeart {
                hearts: blade_heart,
            })
        },
        need_heart: if need_heart.is_empty() {
            None
        } else {
            Some(BaseHeart { hearts: need_heart })
        },
        special_heart,
        #[cfg(not(feature = "compact_cards"))]
        product: get_str(product_idx).into(),
        #[cfg(not(feature = "compact_cards"))]
        rare: get_str(rare_idx).into(),
        #[cfg(not(feature = "compact_cards"))]
        ability: if ability_idx != 0xFFFF {
            get_str(ability_idx).into()
        } else {
            Box::from("")
        },
        #[cfg(not(feature = "compact_cards"))]
        faq: Vec::new(),
        abilities: Vec::new(),
    })
}

/// Decode a single card from the active blob (embedded const or runtime buffer).
pub fn decode_card_from_blob(idx: usize) -> Option<Card> {
    let offset = card_data_offset(idx)?;
    let data = &blob()[offset..];
    let (_num, strtab_len, strtab_start, _, _) = parse_header()?;
    let strtab_end = strtab_start + strtab_len as usize;
    decode_card_from_record(data, &blob()[strtab_start..strtab_end])
}

/// Walk a length-prefixed (u16) strtab slice and return string `idx` (0 = "").
fn get_str_from_strtab(strtab: &[u8], idx: u16) -> &str {
    let mut pos = 0usize;
    for _ in 0..idx {
        if pos + 2 > strtab.len() {
            return "";
        }
        let len = u16::from_le_bytes([strtab[pos], strtab[pos + 1]]) as usize;
        pos += 2 + len;
    }
    if pos + 2 > strtab.len() {
        return "";
    }
    let len = u16::from_le_bytes([strtab[pos], strtab[pos + 1]]) as usize;
    pos += 2;
    if pos + len > strtab.len() {
        return "";
    }
    core::str::from_utf8(&strtab[pos..pos + len]).unwrap_or("")
}

/// Decode every card from a self-contained CARD-format blob slice. Does not use
/// the active blob/global, so `load_two_decks` can decode just the two selected
/// decks' cards from the engine-baked per-deck blobs.
pub fn decode_all_cards_from_slice(blob_slice: &[u8]) -> Vec<Card> {
    if blob_slice.len() < 10 || &blob_slice[0..4] != MAGIC {
        return Vec::new();
    }
    let num_cards = u16::from_le_bytes([blob_slice[4], blob_slice[5]]) as usize;
    let strtab_len =
        u32::from_le_bytes(blob_slice[6..10].try_into().unwrap_or([0, 0, 0, 0])) as usize;
    let length_start = 10 + strtab_len;
    let data_start = length_start + num_cards;
    if blob_slice.len() < data_start {
        return Vec::new();
    }
    let strtab = &blob_slice[10..length_start];

    let mut out = Vec::with_capacity(num_cards);
    let mut start = data_start;
    for idx in 0..num_cards {
        let len = blob_slice[length_start + idx] as usize;
        let end = (start + len).min(blob_slice.len());
        if let Some(card) = decode_card_from_record(&blob_slice[start..end], strtab) {
            out.push(card);
        }
        start = end;
    }
    out
}

fn parse_hearts(data: &[u8], count: usize) -> HeartMap {
    let mut map = HeartMap::new();
    for i in 0..count {
        let base = i * 2;
        if base + 1 >= data.len() {
            break;
        }
        let color = color_from_u8(data[base]);
        let count_val = data[base + 1] as u8;
        if count_val > 0 {
            map.insert(color, count_val);
        }
    }
    map
}

fn color_from_u8(v: u8) -> HeartColor {
    match v {
        0 => HeartColor::Heart00,
        1 => HeartColor::Heart01,
        2 => HeartColor::Heart02,
        3 => HeartColor::Heart03,
        4 => HeartColor::Heart04,
        5 => HeartColor::Heart05,
        6 => HeartColor::Heart06,
        7 => HeartColor::BAll,
        8 => HeartColor::Draw,
        9 => HeartColor::Score,
        10 => HeartColor::All,
        _ => HeartColor::Heart00,
    }
}

/// Build a CardDatabase containing only the specified subset of cards.
/// Reads each card from the CARD_BLOB by index, decodes it, and assigns a sequential ID.
pub fn load_cards_from_blob(indices: &[usize]) -> super::card::CardDatabase {
    let mut cards: HashMap<i16, super::card::Card> = HashMap::default();
    let mut card_no_to_id: HashMap<String, i16> = HashMap::default();
    let mut next_id: i16 = 0;

    for &idx in indices {
        if let Some(card) = decode_card_from_blob(idx) {
            let card_no = card.card_no.to_string();
            if !card_no_to_id.contains_key(&card_no) {
                card_no_to_id.insert(card_no.clone(), next_id);
                cards.insert(next_id, card);
                next_id += 1;
            }
        }
    }

    super::card::CardDatabase {
        cards,
        card_no_to_id,
        next_id,
    }
}

/// Number of cards stored in the embedded CARD_BLOB.
pub fn blob_card_count() -> usize {
    parse_header().map(|(n, ..)| n as usize).unwrap_or(0)
}

/// Find the blob index of a card by its `card_no`.
/// Linear scan — use sparingly. For GBA, pre-resolve deck card indices at boot.
pub fn find_card_index_by_no(card_no: &str) -> Option<usize> {
    let (_num_cards, _strtab_len, _strtab_start, _length_start, _data_start) = parse_header()?;
    #[cfg(feature = "external_card_data")]
    let idx = find_string_index_by_no(card_no)?;
    #[cfg(not(feature = "external_card_data"))]
    let idx = CARD_STRINGS.iter().position(|s| *s == card_no)?;
    // idx 0 is the empty string, skip it
    if idx == 0 {
        return None;
    }
    // String found, now find which card references it
    let (num_cards, _strtab_len, _strtab_start, length_start, data_start) = parse_header()?;
    let lengths = &blob()[length_start..length_start + num_cards as usize];
    let mut start = 0usize;
    for (i, &len) in lengths.iter().enumerate() {
        let card_data = &blob()[data_start + start..];
        if card_data.len() >= 2 {
            let card_no_idx = u16::from_le_bytes(card_data[0..2].try_into().ok()?);
            if card_no_idx as usize == idx {
                return Some(i);
            }
        }
        start += len as usize;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_loader::CardLoader;
    use std::path::Path;

    #[test]
    fn test_parse_header() {
        let h = parse_header().expect("header should parse");
        assert!(h.0 > 0, "num_cards > 0");
    }

    #[test]
    fn test_decode_first_card() {
        let card = decode_card_from_blob(0).expect("card 0 should decode");
        assert!(!card.card_no.is_empty(), "card_no not empty");
        assert!(!card.name.is_empty(), "name not empty");
        println!("Card 0: {} - {}", card.card_no, card.name);
    }

    #[test]
    fn test_decode_member_card() {
        // Find a member card
        for i in 0..100 {
            if let Some(card) = decode_card_from_blob(i) {
                if card.is_member() {
                    println!(
                        "Member {}: {} cost={} blade={}",
                        i,
                        card.name,
                        card.cost.unwrap_or(0),
                        card.blade
                    );
                    return;
                }
            }
        }
        panic!("No member card found in first 100");
    }

    #[test]
    fn test_find_card_by_no() {
        let card0 = decode_card_from_blob(0).expect("card 0");
        let idx = find_card_index_by_no(&card0.card_no);
        assert_eq!(idx, Some(0), "card 0 should be found by its card_no");
    }

    #[test]
    fn test_load_subset() {
        let db = load_cards_from_blob(&[0, 1, 2, 3, 4]);
        assert_eq!(db.cards.len(), 5, "should have 5 cards");
        assert_eq!(db.card_no_to_id.len(), 5, "should have 5 mapping entries");
    }

    #[test]
    fn test_blob_matches_json() {
        // Load ALL cards from blob and JSON, compare by card_no.
        let (num_cards, ..) = parse_header().unwrap();
        let num = num_cards as usize;
        let mut blob_cards: Vec<Card> = Vec::new();
        for i in 0..num {
            if let Some(c) = decode_card_from_blob(i) {
                blob_cards.push(c);
            }
        }

        // Load from cards.json
        let json_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/cards.json"));
        let all_cards =
            CardLoader::load_cards_from_file(json_path).expect("Failed to load cards.json");
        // Sort by card_no (same order as blob)
        let mut all_cards = all_cards;
        all_cards.sort_by(|a, b| a.card_no.cmp(&b.card_no));

        let count = blob_cards.len().min(all_cards.len().min(num));

        // Compare numeric fields directly
        for i in 0..count {
            let b = &blob_cards[i];
            let j = &all_cards[i];
            assert_eq!(b.card_no, j.card_no, "card_no mismatch at blob index {}", i);
            assert_eq!(b.name, j.name, "name mismatch for {}", b.card_no);
            assert_eq!(b.card_type, j.card_type, "type mismatch for {}", b.card_no);
            assert_eq!(b.series, j.series, "series mismatch for {}", b.card_no);
            assert_eq!(b.group, j.group, "group mismatch for {}", b.card_no);
            assert_eq!(b.unit, j.unit, "unit mismatch for {}", b.card_no);
            assert_eq!(b.cost, j.cost, "cost mismatch for {}", b.card_no);
            assert_eq!(b.blade, j.blade, "blade mismatch for {}", b.card_no);
            assert_eq!(b.score, j.score, "score mismatch for {}", b.card_no);
            // Hearts: compare by value (order-independent)
            assert_hearts_eq(&b.base_heart, &j.base_heart, &b.card_no, "base_heart");
            assert_blade_hearts_eq(&b.blade_heart, &j.blade_heart, &b.card_no, "blade_heart");
            assert_hearts_eq(&b.need_heart, &j.need_heart, &b.card_no, "need_heart");
            assert_special_heart_eq(&b.special_heart, &j.special_heart, &b.card_no);
            assert_eq!(b.img, j.img, "img mismatch for {}", b.card_no);
            assert_eq!(b.product, j.product, "product mismatch for {}", b.card_no);
            assert_eq!(b.rare, j.rare, "rare mismatch for {}", b.card_no);
            assert_eq!(b.ability, j.ability, "ability mismatch for {}", b.card_no);
        }
        assert!(count > 0, "at least one card matched");
        println!("Verified {} cards match between blob and JSON", count);
    }

    fn assert_hearts_eq(a: &Option<BaseHeart>, b: &Option<BaseHeart>, card_no: &str, label: &str) {
        match (a, b) {
            (None, None) => {}
            (None, Some(bh)) if bh.hearts.is_empty() => {}
            (Some(ah), None) if ah.hearts.is_empty() => {}
            (Some(ah), Some(bh)) => {
                for (color, count) in ah.hearts.iter() {
                    assert_eq!(
                        bh.hearts.get(color).copied().unwrap_or(0),
                        *count,
                        "{} {} color {:?} mismatch for {}",
                        label,
                        "blob",
                        color,
                        card_no
                    );
                }
                for (color, count) in bh.hearts.iter() {
                    assert_eq!(
                        ah.hearts.get(color).copied().unwrap_or(0),
                        *count,
                        "{} {} color {:?} mismatch for {}",
                        label,
                        "json",
                        color,
                        card_no
                    );
                }
            }
            _ => panic!(
                "{} mismatch for {}: blob={:?} json={:?}",
                label, card_no, a, b
            ),
        }
    }

    fn assert_blade_hearts_eq(
        a: &Option<BladeHeart>,
        b: &Option<BladeHeart>,
        card_no: &str,
        label: &str,
    ) {
        match (a, b) {
            (None, None) => {}
            (None, Some(bh)) if bh.hearts.is_empty() => {}
            (Some(ah), None) if ah.hearts.is_empty() => {}
            (Some(ah), Some(bh)) => {
                for (color, count) in ah.hearts.iter() {
                    assert_eq!(
                        bh.hearts.get(color).copied().unwrap_or(0),
                        *count,
                        "{} {} color {:?} mismatch for {}",
                        label,
                        "blob",
                        color,
                        card_no
                    );
                }
                for (color, count) in bh.hearts.iter() {
                    assert_eq!(
                        ah.hearts.get(color).copied().unwrap_or(0),
                        *count,
                        "{} {} color {:?} mismatch for {}",
                        label,
                        "json",
                        color,
                        card_no
                    );
                }
            }
            _ => panic!(
                "{} mismatch for {}: blob={:?} json={:?}",
                label, card_no, a, b
            ),
        }
    }

    fn assert_special_heart_eq(a: &Option<SpecialHeart>, b: &Option<SpecialHeart>, card_no: &str) {
        match (a, b) {
            (None, None) => {}
            (Some(ah), Some(bh)) => {
                for (color, count) in ah.hearts.iter() {
                    assert_eq!(
                        bh.hearts.get(color).copied().unwrap_or(0),
                        *count,
                        "special_heart blob color {:?} mismatch for {}",
                        color,
                        card_no
                    );
                }
                for (color, count) in bh.hearts.iter() {
                    assert_eq!(
                        ah.hearts.get(color).copied().unwrap_or(0),
                        *count,
                        "special_heart json color {:?} mismatch for {}",
                        color,
                        card_no
                    );
                }
            }
            (None, Some(bh)) if bh.hearts.is_empty() => {} // JSON may have {} = empty special
            (Some(ah), None) if ah.hearts.is_empty() => {} // blob may treat empty as None
            _ => panic!(
                "special_heart mismatch for {}: blob={:?} json={:?}",
                card_no, a, b
            ),
        }
    }
}
