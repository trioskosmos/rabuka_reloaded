use crate::helpers::*;

/// PL!-bp6-006-R+ 西木野真姫 (Nishikino Maki, μ's)
/// 起動 turn1: discard 1 → choose heart color → reveal 5 from deck →
///   if all 5 match chosen heart, pick 1 μ's from revealed to hand,
///   gain 3 blades until live_end, send rest to discard.

fn drain_cost_and_color(game: &mut TestGame) {
    // Drain cost (hand discard) and heart color selection.
    // Do NOT drain subsequent choices (revealed_cards selection, etc.).
    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectTarget") {
        game.select_option(1);
    }
}

#[test]
fn maki_bp6_cost_discard_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id("PL!-bp6-006-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);
    game.state.turn_number = 1;

    let hand_before = game.state.player1.hand.cards.len();

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), hand_before - 1);
    assert_eq!(game.state.player1.energy_zone.active_energy_count, 10);
}

/// Maki full flow with actual μ's cards in deck.
/// Cost: discard 1 from hand → specify heart color → reveal 5 from deck →
/// select μ's card from revealed → remaining go to discard.
#[test]
fn maki_bp6_full_flow_pick_mus_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id("PL!-bp6-006-R+");
    let filler = game.id("PL!-sd1-010-SD");
    let mus_member = game.id("PL!-bp6-002-R"); // 絢瀬絵里 (μ's, cost 2)

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    // Put 1 μ's member + 4 filler in the top 5 of the deck
    game.state.player1.main_deck.cards.push(mus_member);
    for _ in 0..4 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);
    game.state.turn_number = 1;

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    assert!(
        game.has_pending_choice(),
        "SelectCard from revealed expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "revealed cards selection"
    );

    let hand_before = game.state.player1.hand.cards.len();
    game.select_indices(&[0]);

    // Selected μ's card was added to hand
    assert_eq!(game.state.player1.hand.cards.len(), hand_before + 1);
    assert!(
        game.state.player1.hand.cards.contains(&mus_member),
        "μ's member should be in hand"
    );
}

#[test]
fn maki_bp6_deck_lt5_reveals_partial() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id("PL!-bp6-006-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);
    game.state.turn_number = 1;

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[0]);
    }

    assert!(!game.has_pending_choice(), "partial deck flow completed");
    assert_eq!(game.state.player1.main_deck.cards.len(), 0, "all 3 used");
}

#[test]
fn maki_bp6_no_mus_in_revealed_skips_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id("PL!-bp6-006-R+");
    let non_mus = game.id("PL!S-bp2-009-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(non_mus);
    }
    for _ in 0..25 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);
    game.state.turn_number = 1;

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    // Non-μ's revealed cards filtered to empty → prompt with allow_skip
    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[]);
    }

    assert!(!game.has_pending_choice(), "no remaining prompts");
}

#[test]
fn maki_bp6_use_limit_turn1_enforces() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id("PL!-bp6-006-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);
    game.state.turn_number = 1;

    // First activation — full drain
    game.activate_ability(maki);
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectTarget") => {
                game.select_option(1);
            }
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            _ => break,
        }
    }

    // Second activation fails
    let result = game.try_activate_ability(maki);
    assert!(result.is_err(), "use_limit=1 blocks second activation");
}
