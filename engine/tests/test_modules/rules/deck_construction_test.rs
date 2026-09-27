use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::deck_builder::{validate_deck_construction, DeckBuilder};
use serde_json::json;
use std::sync::Arc;

fn test_card(card_no: &str, card_type: &str) -> Card {
    serde_json::from_value(json!({
        "card_no": card_no,
        "name": card_no,
        "type": card_type,
        "series": "Test",
        "group": "Test",
        "base_heart": {},
        "blade": 0,
        "ability": "",
        "faq": []
    }))
    .unwrap()
}

fn test_deck() -> (CardDatabase, Vec<String>, [String; 3]) {
    let mut cards = Vec::new();
    let mut card_numbers = Vec::new();

    for index in 0..12 {
        let card_no = format!("TEST-M-{:03}-N", index);
        cards.push(test_card(&card_no, "メンバー"));
        card_numbers.extend(std::iter::repeat_n(card_no, 4));
    }
    for index in 0..3 {
        let card_no = format!("TEST-L-{:03}-N", index);
        cards.push(test_card(&card_no, "ライブ"));
        card_numbers.extend(std::iter::repeat_n(card_no, 4));
    }
    for index in 0..3 {
        let card_no = format!("TEST-E-{:03}-N", index);
        cards.push(test_card(&card_no, "エネルギー"));
        card_numbers.extend(std::iter::repeat_n(card_no, 4));
    }

    let spare_members = format!("TEST-M-{:03}-N", 12);
    let spare_lives = format!("TEST-L-{:03}-N", 3);
    let spare_energy = format!("TEST-E-{:03}-N", 3);
    cards.push(test_card(&spare_members, "メンバー"));
    cards.push(test_card(&spare_lives, "ライブ"));
    cards.push(test_card(&spare_energy, "エネルギー"));

    (
        CardDatabase::load_or_create(cards),
        card_numbers,
        [spare_members, spare_lives, spare_energy],
    )
}

fn first_index_with_prefix(card_numbers: &[String], prefix: &str) -> usize {
    card_numbers
        .iter()
        .position(|card_no| card_no.starts_with(prefix))
        .unwrap()
}

#[test]
fn accepts_exact_rule_6_1_1_deck() {
    let (db, card_numbers, _) = test_deck();
    assert_eq!(validate_deck_construction(&db, &card_numbers), Ok(()));
}

#[test]
fn rejects_under_and_over_category_totals() {
    let (db, exact, spares) = test_deck();
    for (index, expected, spare_index) in [
        (0, "Member", 0),
        (48, "Deck must contain exactly 12 live", 1),
        (60, "Energy", 2),
    ] {
        let mut card_numbers = exact.clone();
        card_numbers.remove(index);
        assert!(validate_deck_construction(&db, &card_numbers)
            .unwrap_err()
            .starts_with(expected));
        let mut card_numbers = exact.clone();
        card_numbers.push(spares[spare_index].clone());
        assert!(validate_deck_construction(&db, &card_numbers)
            .unwrap_err()
            .starts_with(expected));
    }
}

#[test]
fn rejects_mixed_totals_with_correct_grand_total() {
    let (db, mut card_numbers, spares) = test_deck();
    let member_index = first_index_with_prefix(&card_numbers, "TEST-M-");
    card_numbers[member_index] = spares[1].clone();
    assert!(validate_deck_construction(&db, &card_numbers)
        .unwrap_err()
        .contains("Member deck must contain exactly 48"));
}

#[test]
fn counts_aliases_toward_canonical_copy_limit() {
    let (db, mut card_numbers, _) = test_deck();
    let canonical = "TEST-M-000-N";
    let indexes: Vec<_> = card_numbers
        .iter()
        .enumerate()
        .filter(|(_, card_no)| card_no.as_str() == canonical)
        .map(|(index, _)| index)
        .collect();
    card_numbers[indexes[0]] = "test-m-000-n".to_string();
    card_numbers[indexes[1]] = canonical.to_string();
    assert_eq!(validate_deck_construction(&db, &card_numbers), Ok(()));

    let other_member = first_index_with_prefix(&card_numbers, "TEST-M-001-N");
    card_numbers[other_member] = "test-m-000-n".to_string();
    assert!(validate_deck_construction(&db, &card_numbers)
        .unwrap_err()
        .contains("maximum 4"));
}

#[test]
fn legacy_builder_still_accepts_starter_sized_lists() {
    let (db, _, _) = test_deck();
    let mut db = Arc::new(db);
    let deck = DeckBuilder::build_deck_from_database(&mut db, vec!["TEST-M-000-N".to_string()])
        .expect("starter-sized deck");
    assert_eq!(deck.main_deck.len(), 1);
    assert!(deck.energy_deck.is_empty());
}
