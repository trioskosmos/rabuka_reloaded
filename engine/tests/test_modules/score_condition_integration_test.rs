use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Helper to calculate total score in P1's success_live_card_zone
fn total_success_score(game: &TestGame) -> u32 {
    let db = game.state.card_database.clone();
    game.state
        .player1
        .success_live_card_zone
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|c| c.score)
        .sum()
}

/// Build a live card ID by name (for readability)
fn live_named(game: &TestGame, name: &str) -> i16 {
    game.id(name)
}

/// RIN (PL!-bp5-005-R): 登場, if success_zone total score >= 6 → gain 1 energy from energy_deck.
///
/// Test: score starts below threshold (2), then rises above (8), and condition responds correctly.
#[test]
fn rin_score_condition_below_then_above() {
    let mut game = TestGame::new(load_real_database());

    let filler = game.id("PL!-sd1-010-SD");
    let rin_a = game.id("PL!-bp5-005-R");
    let rin_b = game.id("PL!-bp5-005-R");
    let low_live = live_named(&game, "PL!-bp3-024-L"); // score 2
    let high_live = live_named(&game, "PL!-bp3-022-L"); // score 5

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Populate energy deck (RIN moves from energy_deck to energy_zone)
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..5 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(20);

    // Start with score = 2 (below threshold)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(low_live);
    assert_eq!(total_success_score(&game), 2, "Initial score should be 2");

    // Play first RIN → condition should FAIL (2 < 6)
    let deck_before = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin_a);
    game.play_to_stage(rin_a, MemberArea::Center);
    let deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(
        deck_after, deck_before,
        "RIN should NOT consume energy deck when score=2 (< 6)"
    );

    // Add score-5 card → total = 7 (now >= 6)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(high_live);
    assert_eq!(total_success_score(&game), 7);

    // Play another RIN → condition should PASS
    let deck_before2 = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin_b);
    game.play_to_stage(rin_b, MemberArea::LeftSide);
    let deck_after2 = game.state.player1.energy_deck.cards.len();
    assert!(
        deck_after2 < deck_before2,
        "RIN should consume 1 energy deck card when score=7 (>= 6), deck {} -> {}",
        deck_before2,
        deck_after2
    );
    assert_eq!(
        deck_before2 - deck_after2,
        1,
        "Exactly 1 energy should be moved"
    );
}

/// Boundary: exactly score=6
#[test]
fn rin_score_condition_at_exactly_6() {
    let mut game = TestGame::new(load_real_database());

    let filler = game.id("PL!-sd1-010-SD");
    let rin = game.id("PL!-bp5-005-R");
    let live_a = live_named(&game, "PL!-bp3-024-L"); // score 2
    let live_b = live_named(&game, "PL!-bp4-019-L"); // score 4

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(live_a);
    game.state.player1.success_live_card_zone.cards.push(live_b);
    assert_eq!(total_success_score(&game), 6);

    let deck_before = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin);
    game.play_to_stage(rin, MemberArea::Center);
    let deck_after = game.state.player1.energy_deck.cards.len();
    assert!(
        deck_after < deck_before,
        "RIN should activate at exactly score=6 (>= 6)"
    );
}

/// No success zone cards at all
#[test]
fn rin_score_condition_no_cards() {
    let mut game = TestGame::new(load_real_database());

    let filler = game.id("PL!-sd1-010-SD");
    let rin = game.id("PL!-bp5-005-R");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(10);

    assert_eq!(total_success_score(&game), 0);

    let deck_before = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin);
    game.play_to_stage(rin, MemberArea::Center);
    let deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(
        deck_after, deck_before,
        "RIN should NOT activate when score=0"
    );
}
