use super::card::{BaseHeart, BladeHeart, Card, CardType, HeartColor, HeartMap, SpecialHeart};
use super::cards_gen::{CARD_BLOB, CARD_STRINGS};
use crate::core::types::ArcStr;
use crate::{HashMap, HashSet};

const MAGIC: &[u8; 4] = b"CARD";

/// Parse the CARD_BLOB header and return (num_cards, strtab_len, strtab_start, offset_start, data_start).
fn parse_header() -> Option<(u32, u32, usize, usize, usize)> {
    if CARD_BLOB.len() < 12 || &CARD_BLOB[0..4] != MAGIC {
        return None;
    }
    let num_cards = u32::from_le_bytes(CARD_BLOB[4..8].try_into().ok()?);
    let strtab_len = u32::from_le_bytes(CARD_BLOB[8..12].try_into().ok()?);
    let strtab_start = 12;
    let offset_start = strtab_start + strtab_len as usize;
    let data_start = offset_start + (num_cards as usize + 1) * 4;
    Some((
        num_cards,
        strtab_len,
        strtab_start,
        offset_start,
        data_start,
    ))
}

/// Get the byte offset of card `idx`'s data within CARD_BLOB.
fn card_data_offset(idx: usize) -> Option<usize> {
    let (num_cards, _strtab_len, _strtab_start, offset_start, data_start) = parse_header()?;
    if idx >= num_cards as usize {
        return None;
    }
    let off_start = offset_start + idx * 4;
    let off_next = offset_start + (idx + 1) * 4;
    let start = u32::from_le_bytes(CARD_BLOB[off_start..off_start + 4].try_into().ok()?) as usize;
    let _end = u32::from_le_bytes(CARD_BLOB[off_next..off_next + 4].try_into().ok()?) as usize;
    Some(data_start + start)
}

/// Get string from CARD_STRINGS by index (0 = empty string).
fn get_str(idx: u16) -> &'static str {
    if (idx as usize) < CARD_STRINGS.len() {
        CARD_STRINGS[idx as usize]
    } else {
        ""
    }
}

pub fn decode_card_from_blob(idx: usize) -> Option<Card> {
    let offset = card_data_offset(idx)?;
    let data = &CARD_BLOB[offset..];

    if data.len() < 20 {
        return None;
    }

    let card_no_idx = u16::from_le_bytes(data[0..2].try_into().ok()?);
    let name_idx = u16::from_le_bytes(data[2..4].try_into().ok()?);
    let series_idx = u16::from_le_bytes(data[4..6].try_into().ok()?);
    let group_idx = u16::from_le_bytes(data[6..8].try_into().ok()?);
    let unit_idx = u16::from_le_bytes(data[8..10].try_into().ok()?);
    let img_idx = u16::from_le_bytes(data[10..12].try_into().ok()?);
    let product_idx = u16::from_le_bytes(data[12..14].try_into().ok()?);
    let rare_idx = u16::from_le_bytes(data[14..16].try_into().ok()?);
    let ability_idx = u16::from_le_bytes(data[16..18].try_into().ok()?);
    let type_flags = data[18];
    let cost_val = data[19];
    let blade_val = data[20];
    let score_val = data[21];
    let num_base = data[22] as usize;
    let num_blade = data[23] as usize;
    let num_need = data[24] as usize;
    let ctype = type_flags & 0x03;
    let has_special = (type_flags >> 2) & 0x01;

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
            series.clone()
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
            hearts.insert(color_from_u8(sc), scount as u32);
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
        series,
        group,
        card_type,
        unit,
        cost: if cost_val > 0 {
            Some(cost_val as u32)
        } else {
            None
        },
        blade: blade_val as u32,
        score: if score_val > 0 {
            Some(score_val as u32)
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
        img: if img_idx != 0 {
            Some(get_str(img_idx).into())
        } else {
            None
        },
        product: get_str(product_idx).into(),
        rare: get_str(rare_idx).into(),
        ability: if ability_idx != 0xFFFF {
            get_str(ability_idx).into()
        } else {
            Box::from("")
        },
        faq: Vec::new(),
        abilities: Vec::new(),
    })
}

fn parse_hearts(data: &[u8], count: usize) -> HeartMap {
    let mut map = HeartMap::new();
    for i in 0..count {
        let base = i * 2;
        if base + 1 >= data.len() {
            break;
        }
        let color = color_from_u8(data[base]);
        let count_val = data[base + 1] as u32;
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

/// Find the blob index of a card by its `card_no`.
/// Linear scan — use sparingly. For GBA, pre-resolve deck card indices at boot.
pub fn find_card_index_by_no(card_no: &str) -> Option<usize> {
    let (_num_cards, _strtab_len, _strtab_start, _offset_start, _data_start) = parse_header()?;
    let idx = CARD_STRINGS.iter().position(|s| *s == card_no)?;
    // idx 0 is the empty string, skip it
    if idx == 0 {
        return None;
    }
    // String found, now find which card references it
    let (_num_cards, _strtab_len, _strtab_start, offset_start, data_start) = parse_header()?;
    let num_cards = u32::from_le_bytes(CARD_BLOB[4..8].try_into().ok()?);
    for i in 0..num_cards as usize {
        let off_start = offset_start + i * 4;
        if off_start + 4 >= CARD_BLOB.len() {
            break;
        }
        let start =
            u32::from_le_bytes(CARD_BLOB[off_start..off_start + 4].try_into().ok()?) as usize;
        let card_data = &CARD_BLOB[data_start + start..];
        if card_data.len() >= 2 {
            let card_no_idx = u16::from_le_bytes(card_data[0..2].try_into().ok()?);
            if card_no_idx as usize == idx {
                return Some(i);
            }
        }
    }
    None
}

/// Resolve deck card indices from a list of card_no strings.
pub fn resolve_deck_indices(card_nos: &[&str]) -> Vec<usize> {
    let mut indices = Vec::with_capacity(card_nos.len());
    let mut seen: HashSet<String> = HashSet::default();
    for cn in card_nos {
        let s = cn.to_string();
        if !seen.contains(&s) {
            seen.insert(s);
            if let Some(idx) = find_card_index_by_no(cn) {
                indices.push(idx);
            }
        }
    }
    indices
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
        let num = (num_cards as usize).min(100); // first 100 cards
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
            assert_eq!(b.unit, j.unit, "unit mismatch for {}", b.card_no);
            assert_eq!(
                b.cost.unwrap_or(0),
                j.cost.unwrap_or(0),
                "cost mismatch for {}",
                b.card_no
            );
            assert_eq!(b.blade, j.blade, "blade mismatch for {}", b.card_no);
            assert_eq!(
                b.score.unwrap_or(0),
                j.score.unwrap_or(0),
                "score mismatch for {}",
                b.card_no
            );
            // Hearts: compare by value (order-independent)
            assert_hearts_eq(&b.base_heart, &j.base_heart, &b.card_no, "base_heart");
            assert_blade_hearts_eq(&b.blade_heart, &j.blade_heart, &b.card_no, "blade_heart");
            assert_hearts_eq(&b.need_heart, &j.need_heart, &b.card_no, "need_heart");
            assert_special_heart_eq(&b.special_heart, &j.special_heart, &b.card_no);
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
