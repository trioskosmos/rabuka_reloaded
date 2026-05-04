/// Tests for LL-bp3-001-R+ (園田海未&津島善子&天王寺璃奈) — Multi-name joint card
///
/// Ab#0 (起動/ターン1回): Move 6 cards from discard to deck bottom
///   → activate up to 6 energy.
///
/// Ab#1 (ライブ開始時): Optional pay 6E → gain 3 blade until live end.
///
/// Q165: Don't need all three characters — any combination totaling 6 works.
/// Q62:  Multi-name card — has individual names.
//=====================================================================

mod helpers;
use helpers::*;

fn card_id_by_no(db: &std::sync::Arc<rabuka_engine::card::CardDatabase>, no: &str) -> i16 {
    db.get_card_id(no).unwrap_or_else(|| panic!("Card {no} not found"))
}

/// Verify both abilities are parsed correctly
#[test]
fn ll_joint_ability_parsed() {
    let db = load_real_database();
    let card = db.get_card_by_no("LL-bp3-001-R\u{ff0b}")
        .expect("Joint card should exist");

    assert_eq!(card.abilities.len(), 2, "Should have 2 abilities");

    // Ab#0
    let ab0 = &card.abilities[0];
    assert_eq!(ab0.triggers.as_deref(), Some("起動"), "Ab#0 trigger should be 起動");
    assert_eq!(ab0.use_limit, Some(1), "Ab#0 should be once per turn");

    let cost = ab0.cost.as_ref().expect("Ab#0 should have cost");
    assert_eq!(cost.cost_type.as_deref(), Some("move_cards"), "Ab#0 cost should be move_cards");
    assert_eq!(cost.source.as_deref(), Some("discard"), "Ab#0 cost source should be discard");
    assert_eq!(cost.destination.as_deref(), Some("deck_bottom"), "Ab#0 cost destination should be deck_bottom");
    assert_eq!(cost.characters.as_ref().map(|c| c.len()), Some(3), "Ab#0 cost should have 3 character filters");

    // Count parsed from "合計6枚" in the ability text
    assert_eq!(cost.count, Some(6), "Ab#0 cost count should be 6");

    // Ab#1
    let ab1 = &card.abilities[1];
    assert_eq!(ab1.triggers.as_deref(), Some("ライブ開始時"), "Ab#1 trigger should be ライブ開始時");

    let cost1 = ab1.cost.as_ref().expect("Ab#1 should have cost");
    assert_eq!(cost1.cost_type.as_deref(), Some("pay_energy"), "Ab#1 cost should be pay_energy");
    assert!(cost1.optional.unwrap_or(false), "Ab#1 cost should be optional");
    assert_eq!(cost1.energy, Some(6), "Ab#1 cost should be 6 energy");
}

/// Q62: Multi-name card has individual names
#[test]
fn ll_joint_has_individual_names() {
    let db = load_real_database();
    let card_id = card_id_by_no(&db, "LL-bp3-001-R\u{ff0b}");
    let names = db.get_card_names(card_id);
    assert!(names.contains(&"園田海未".to_string()));
    assert!(names.contains(&"津島善子".to_string()));
    assert!(names.contains(&"天王寺璃奈".to_string()));
}

/// Q165: Don't need all three characters — any combination of the 3 totaling 6 works.
#[test]
fn ll_joint_q165_any_combination_of_characters() {
    let db = load_real_database();
    let card = db.get_card_by_no("LL-bp3-001-R\u{ff0b}").unwrap();
    let cost = card.abilities[0].cost.as_ref().unwrap();

    let chars = cost.characters.as_ref().expect("Cost should have characters");
    assert_eq!(chars.len(), 3, "Should have 3 character filters");
    assert!(chars.contains(&"園田海未".to_string()));
    assert!(chars.contains(&"津島善子".to_string()));
    assert!(chars.contains(&"天王寺璃奈".to_string()));
}

/// Without 6+ cards in discard, the ability creates no choice prompt
#[test]
fn ll_joint_requires_6_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let joint = game.id("LL-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage[1] = joint;
    game.give_energy(5);

    // Put only 3 cards in discard
    for _ in 0..3 {
        game.state.player1.waitroom.cards.push(game.id("PL!-sd1-010-SD"));
    }

    game.activate_ability(joint);

    // With < 6 cards, cost validation fails (no matching_indices can't find 6),
    // and no pending choice is created
    assert!(!game.has_pending_choice(),
        "No choice prompt with < 6 matching cards");
    assert_eq!(game.state.player1.waitroom.cards.len(), 3,
        "Cards should remain in discard");
}

/// With 6+ cards in discard, cost creates a choice
#[test]
fn ll_joint_creates_choice_with_6_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let joint = game.id("LL-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage[1] = joint;
    game.give_energy(5);

    // Put 6 cards in discard (character filtering not yet applied in engine)
    for _ in 0..6 {
        game.state.player1.waitroom.cards.push(game.id("PL!-sd1-010-SD"));
    }

    game.activate_ability(joint);

    // Should create a pending choice (SelectCard from discard)
    assert!(game.has_pending_choice(),
        "Should create a choice to select 6 cards from discard");
}
