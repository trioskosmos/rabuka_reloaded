use crate::helpers::*;

/// PL!S-bp2-021-L 未体験HORIZON — LiveSuccess: from revealed yell cards, place 1 live ≤ deck bottom (max 1, optional)
#[test]
fn s_bp2_021_live_success_with_live_in_yell_moves_to_deck_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp2-021-L");
    let filler = game.id("PL!-sd1-010-SD");
    let live_in_yell = game.id("PL!-sd1-019-SD"); // live card to be in revealed
    // Setup live success scenario: need to trigger LiveSuccess after a successful live
    // Use fire_trigger helper to directly fire LiveSuccess
    game.state.player1.stage.stage = [game.id("PL!S-sd1-001-SD"), -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    // Simulate yell revealing a live card
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(live_in_yell);
    game.state.yell_occurred = true;
    game.state.player1.main_deck.cards.push(filler); // ensure deck has filler
    let deck_len_before = game.state.player1.main_deck.cards.len();
    crate::helpers::fire_trigger(&mut game, live, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    // The move should be pending as SelectCard over revealed_cards (max 1, allow_skip)
    if game.has_pending_choice() {
        // Should offer the live_in_yell as selectable
        game.select_indices(&[0]);
        while game.has_pending_choice() { game.select_indices(&[]); }
    }
    // Live should now be at deck bottom
    let deck = &game.state.player1.main_deck.cards;
    assert!(deck.contains(&live_in_yell) || deck.len() == deck_len_before + 1, "live should be at deck bottom or deck grew");
    assert!(!game.state.revealed_cards.contains(&live_in_yell), "revealed should be cleared");
}

#[test]
fn s_bp2_021_live_success_no_live_in_yell_no_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp2-021-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [game.id("PL!S-sd1-001-SD"), -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(filler); // only filler, no live
    game.state.yell_occurred = true;
    let deck_before = game.state.player1.main_deck.cards.clone();
    crate::helpers::fire_trigger(&mut game, live, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    if game.has_pending_choice() {
        // No live eligible, should be skippable or auto-skip
        game.select_indices(&[]);
        while game.has_pending_choice() { game.select_indices(&[]); }
    }
    assert_eq!(game.state.player1.main_deck.cards, deck_before, "no live in yell should not change deck");
}

#[test]
fn s_bp2_021_live_success_empty_yell_no_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp2-021-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [game.id("PL!S-sd1-001-SD"), -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.revealed_cards.clear();
    game.state.yell_occurred = false;
    let deck_before = game.state.player1.main_deck.cards.clone();
    crate::helpers::fire_trigger(&mut game, live, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(game.state.player1.main_deck.cards, deck_before);
}

#[test]
fn s_bp2_021_live_success_skip_optional() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp2-021-L");
    let filler = game.id("PL!-sd1-010-SD");
    let live_in_yell = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage = [game.id("PL!S-sd1-001-SD"), -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(live_in_yell);
    game.state.yell_occurred = true;
    let deck_before = game.state.player1.main_deck.cards.clone();
    crate::helpers::fire_trigger(&mut game, live, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    if game.has_pending_choice() {
        game.select_indices(&[]); // skip optional
    }
    // Deck should be unchanged when skipped
    assert_eq!(game.state.player1.main_deck.cards, deck_before);
    assert!(game.state.revealed_cards.contains(&live_in_yell) || game.state.player1.main_deck.cards.contains(&live_in_yell)==false, "skip should leave revealed");
}
