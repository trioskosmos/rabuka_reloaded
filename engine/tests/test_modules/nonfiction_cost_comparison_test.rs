/// Tests for PL!SP-bp4-024 (ノンフィクション!!) ab#0:
/// LiveStart: If your center Liella! member's cost > opponent's center member's cost, this card's score +1.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
}

fn fill_both_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn drain_auto_choices(game: &mut TestGame) {
    while game.state.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// P1: high-cost Liella! center (cost 11) > P2: low-cost center (cost 4)
/// → condition passes → score +1
#[test]
fn nonfiction_cost_p1_high_vs_p2_low_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let p1_center = game.id("PL!SP-pb1-001-R"); // Kanon, cost=11, Liella!
    let filler = game.id("PL!-sd1-010-SD"); // cost=4, not Liella!

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage[1] = p1_center;
    game.state.player2.stage.stage[1] = filler;
    fill_both_decks(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);

    let score_mod = game.state.mods.get_score_modifier(nonfiction);
    assert_eq!(
        score_mod, 1,
        "P1 cost=11 > P2 cost=4 → score should be +1, got {}",
        score_mod
    );
}

/// P1: high-cost Liella! (cost 11) > P2: empty center (cost 0)
/// → condition passes → score +1
#[test]
fn nonfiction_cost_p1_high_vs_p2_empty_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let p1_center = game.id("PL!SP-pb1-001-R"); // cost=11, Liella!
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage[1] = p1_center;
    game.state.player2.stage.stage[1] = -1; // empty center
    fill_both_decks(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);

    let score_mod = game.state.mods.get_score_modifier(nonfiction);
    assert_eq!(
        score_mod, 1,
        "P1 cost=11 > P2 empty (cost=0) → score should be +1, got {}",
        score_mod
    );
}

/// P1: low-cost center (cost 4) vs P2: high-cost (cost 11)
/// → condition fails (P1 cost NOT higher) → score unchanged
#[test]
fn nonfiction_cost_p1_low_vs_p2_high_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let p2_center = game.id("PL!SP-pb1-001-R"); // cost=11, Liella!
    let filler = game.id("PL!-sd1-010-SD"); // cost=4

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage[1] = filler; // cost=4
    game.state.player2.stage.stage[1] = p2_center; // cost=11
    fill_both_decks(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);

    let score_mod = game.state.mods.get_score_modifier(nonfiction);
    assert_eq!(
        score_mod, 0,
        "P1 cost=4 < P2 cost=11 → score should be 0, got {}",
        score_mod
    );
}

/// P1 center is NOT Liella! → condition fails even if cost is higher
#[test]
fn nonfiction_cost_p1_non_liella_center_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let filler = game.id("PL!-sd1-010-SD"); // cost=4, NOT Liella!

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage[1] = filler; // not Liella!
    game.state.player2.stage.stage[1] = filler;
    fill_both_decks(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);

    let score_mod = game.state.mods.get_score_modifier(nonfiction);
    assert_eq!(
        score_mod, 0,
        "P1 center not Liella! → score should be 0, got {}",
        score_mod
    );
}

/// P1: Liella! center cost 11, P2: Liella! center cost 11 (same cost)
/// → condition fails (11 > 11 is false) → score unchanged
#[test]
fn nonfiction_cost_equal_costs_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let p1_center = game.id("PL!SP-pb1-001-R"); // cost=11, Liella!
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage[1] = p1_center;
    game.state.player2.stage.stage[1] = game.new_id("PL!SP-pb1-001-R"); // also cost=11
    fill_both_decks(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);

    let score_mod = game.state.mods.get_score_modifier(nonfiction);
    assert_eq!(
        score_mod, 0,
        "P1 cost=11 == P2 cost=11 → score should be 0, got {}",
        score_mod
    );
}

/// P1: Liella! center cost 11, P2: Liella! center cost 22
/// → condition fails (11 > 22 is false) → score unchanged
#[test]
fn nonfiction_cost_p1_lower_than_p2_liella_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let p1_center = game.id("PL!SP-pb1-001-R"); // cost=11, Liella!
    let p2_center = game.id("PL!SP-bp4-004-R\u{ff0b}"); // cost=22, Liella!
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage[1] = p1_center;
    game.state.player2.stage.stage[1] = p2_center;
    fill_both_decks(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);

    let score_mod = game.state.mods.get_score_modifier(nonfiction);
    assert_eq!(
        score_mod, 0,
        "P1 cost=11 < P2 cost=22 → score should be 0, got {}",
        score_mod
    );
}
