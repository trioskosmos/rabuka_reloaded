use crate::helpers::*;

fn maki_bp6_setup(game: &mut TestGame, deck_top: &[i16]) -> i16 {
    let maki = game.id("PL!-bp6-006-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for &cid in deck_top {
        game.state.player1.main_deck.cards.push(cid);
    }
    for _ in 0..(30 - deck_top.len()) {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);
    game.state.turn_number = 1;
    maki
}

fn drain_cost_and_color(game: &mut TestGame) {
    // Drain cost (hand discard) and heart color selection.
    // Do NOT drain subsequent choices (revealed_cards selection, etc.).
    assert!(
        game.has_pending_choice(),
        "hand discard cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the hand discard cost"
    );
    game.select_indices(&[0]);

    assert!(
        game.has_pending_choice(),
        "heart color selection prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectHeartColor"),
        "expected SelectHeartColor prompt"
    );
    game.select_option(1); // heart02
}

/// All 5 revealed cards have heart02 → condition met → blade+3, selection enabled.
#[test]
fn maki_bp6_all_match_grants_blade_and_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp2-009-R"); // member heart02, non-μ's
    let mus_maki = game.id("PL!-bp5-015-N"); // member heart02, μ's

    let maki = maki_bp6_setup(&mut game, &[ruby, ruby, ruby, ruby, mus_maki]);

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    // Condition met → selection should appear with mus_maki as pickable
    assert!(
        game.has_pending_choice(),
        "Expected selection prompt when condition met"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "Expected SelectCard from revealed cards"
    );

    let hand_before = game.state.player1.hand.cards.len();
    game.select_indices(&[0]); // select the only μ's card (after filter)

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "μ's card should be added to hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mus_maki),
        "μ's Maki should be in hand"
    );

    // Blade should be +3
    let blade = game.state.mods.get_blade_modifier(maki);
    assert_eq!(
        blade, 3,
        "All 5 match heart02 → expected blade+3, got {}",
        blade
    );
}

/// Only 4 of 5 revealed cards have heart02 → condition fails → no blade, no selection.
#[test]
fn maki_bp6_one_mismatch_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp2-009-R"); // heart02 ✓
    let filler = game.id("PL!-sd1-010-SD"); // heart01+heart03, NO heart02 ✗

    let maki = maki_bp6_setup(&mut game, &[ruby, ruby, ruby, ruby, filler]);

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    // Condition fails → no selection should appear, no pending SelectCard
    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectCard") {
        panic!("Selection should NOT appear when only 4/5 cards match heart color");
    }

    // Blade should be 0
    let blade = game.state.mods.get_blade_modifier(maki);
    assert_eq!(
        blade, 0,
        "Only 4 of 5 match heart02 → expected blade=0, got {}",
        blade
    );
}

/// Mix of member + live cards, all matching heart02 → condition met.
#[test]
fn maki_bp6_live_and_member_all_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp2-009-R"); // member heart02
    let live = game.id("PL!S-bp6-019-L"); // live need_heart=heart02
    let mus_maki = game.id("PL!-bp5-015-N"); // member heart02, μ's

    let maki = maki_bp6_setup(&mut game, &[ruby, ruby, live, live, mus_maki]);

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    // Condition met → selection should appear
    assert!(
        game.has_pending_choice(),
        "Expected selection prompt when all 5 match"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "Expected SelectCard"
    );

    let hand_before = game.state.player1.hand.cards.len();
    game.select_indices(&[0]); // select the only μ's card (after filter)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "μ's card should be in hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mus_maki),
        "μ's Maki should be in hand"
    );

    let blade = game.state.mods.get_blade_modifier(maki);
    assert_eq!(
        blade, 3,
        "All 5 match (member+live) → expected blade+3, got {}",
        blade
    );
}

/// None of the 5 have heart02 → condition fails.
#[test]
fn maki_bp6_none_match_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD"); // heart01+heart03, NO heart02

    let maki = maki_bp6_setup(&mut game, &[filler, filler, filler, filler, filler]);

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    // No selection
    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectCard") {
        panic!("Selection should NOT appear when no cards match heart color");
    }

    let blade = game.state.mods.get_blade_modifier(maki);
    assert_eq!(
        blade, 0,
        "0 of 5 match heart02 → expected blade=0, got {}",
        blade
    );
}

/// All 5 match heart02 but none are μ's → blade still granted, selection skipped.
#[test]
fn maki_bp6_all_match_no_mus_skips_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp2-009-R"); // heart02, non-μ's

    let maki = maki_bp6_setup(&mut game, &[ruby, ruby, ruby, ruby, ruby]);

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    // No μ's cards → SelectCard prompt with allow_skip=true, count=0
    // Skip it to let the remaining pending actions (gain_resource) run
    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[]); // skip
    }

    // All 5 match → blade should be +3 after all pending actions resolve
    let blade = game.state.mods.get_blade_modifier(maki);
    assert_eq!(
        blade, 3,
        "All 5 match but no μ's → expected blade+3 still, got {}",
        blade
    );
}

/// Verify blade clears at live end.
#[test]
fn maki_bp6_blade_expires_at_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp2-009-R");
    let mus_maki = game.id("PL!-bp5-015-N");

    let maki = maki_bp6_setup(&mut game, &[ruby, ruby, ruby, ruby, mus_maki]);

    game.activate_ability(maki);
    drain_cost_and_color(&mut game);

    // Selection + drain
    assert!(
        game.has_pending_choice(),
        "revealed_cards selection prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the μ's-card selection"
    );
    game.select_indices(&[0]); // select the only μ's card (after filter)

    // Blade should be +3 during live
    assert_eq!(
        game.state.mods.get_blade_modifier(maki),
        3,
        "Blade+3 during live"
    );

    // Advance past live end so live_end effects expire
    // Keep passing until we're past the Live turn phase
    for _ in 0..20 {
        game.pass();
    }

    // After live end, blade should be 0
    let blade = game.state.mods.get_blade_modifier(maki);
    assert_eq!(
        blade, 0,
        "Blade should expire after live end, got {}",
        blade
    );
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
    assert_eq!(game.state.player1.energy_zone.active_count(), 10);
}

/// Maki full flow with actual μ's cards in deck.
/// Cost: discard 1 from hand → specify heart color → reveal 5 from deck →
/// select μ's card from revealed → remaining go to discard.
#[test]
fn maki_bp6_full_flow_pick_mus_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id("PL!-bp6-006-R+");
    let ruby = game.id("PL!S-bp2-009-R"); // heart02, non-μ's
    let mus_maki = game.id("PL!-bp5-015-N"); // heart02, μ's
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    // All 5 must match heart02 for the condition to pass
    game.state.player1.main_deck.cards.push(ruby);
    game.state.player1.main_deck.cards.push(ruby);
    game.state.player1.main_deck.cards.push(ruby);
    game.state.player1.main_deck.cards.push(ruby);
    game.state.player1.main_deck.cards.push(mus_maki);
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
        game.state.player1.hand.cards.contains(&mus_maki),
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
            Some("SelectHeartColor") | Some("SelectTarget") => {
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
