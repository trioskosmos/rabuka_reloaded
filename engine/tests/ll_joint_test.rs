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

    // Put 7 cards matching the character filter (more than count=6 -> creates a choice)
    let sonoda_umi = game.id("PL!-PR-014-PR");
    for _ in 0..7 {
        game.state.player1.waitroom.cards.push(sonoda_umi);
    }

    game.activate_ability(joint);

    // Should create a pending choice (SelectCard from discard)
    assert!(game.has_pending_choice(),
        "Should create a choice to select 6 cards from discard");
}
