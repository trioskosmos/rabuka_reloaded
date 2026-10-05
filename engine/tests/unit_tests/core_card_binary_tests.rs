// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::game_setup;
use rabuka_engine::game_setup::*;
use rabuka_engine::game::match_runner::*;
use rabuka_engine::game::sav::*;
use rabuka_engine::ability::abilities_gen::*;
use rabuka_engine::ability::types::*;
use rabuka_engine::ability::util::duration::*;
use rabuka_engine::bot::v8_live::*;
use rabuka_engine::bot::v8_model::*;
use rabuka_engine::bot::strategy_common::*;
use rabuka_engine::card::*;
use rabuka_engine::card_loader::*;
use rabuka_engine::constants::*;
use rabuka_engine::game_state::*;
use rabuka_engine::player::*;
use rabuka_engine::types::*;
use rabuka_engine::zones::*;
use rabuka_engine::HashMap;
use smallvec::SmallVec;


use rabuka_engine::core::card_binary::*;


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
    fn test_normalize_card_no_fold() {
        assert_eq!(normalize_card_no("LL-bp2-001-R＋"), "LL-BP2-001-R+");
        assert_eq!(normalize_card_no("PL!S-pb1-003-P＋"), "PL!S-PB1-003-P+");
        assert_eq!(normalize_card_no("BP1-005-R"), "BP1-005-R");
        // Mirrors the DS/bake fold exactly: fullwidth uppercase is NOT
        // folded (only fullwidth lowercase + digits are).
        assert_eq!(normalize_card_no("０１２３４５６７８９"), "0123456789");
    }

    #[test]
    fn test_folded_index_resolves_all_blob_cards() {
        let index = blob_index_by_folded_no();
        assert!(!index.is_empty(), "index covers the blob");
        assert!(
            index.windows(2).all(|w| w[0].0 <= w[1].0),
            "index is sorted"
        );
        // Every blob card resolves through the index (exact or folded).
        for i in 0..blob_card_count() {
            let card = decode_card_from_blob(i).expect("card decodes");
            let got = resolve_card_no(&index, card.card_no.as_ref());
            assert!(got.is_some(), "card {} resolves", card.card_no);
        }
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

            // Load from cards.json (shared process-wide parse, not per test)
            let mut all_cards = rabuka_engine::core::card_loader::test_cards().to_vec();
            // Sort by card_no (same order as blob)
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
