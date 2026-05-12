/// Tests for PL!-bp3-014-PR/N 星空凛 (and sharing cards) — Debut look+select ability:
///
/// Ab#0 (登場):
///   Cost (optional): put this member to wait state
///   Effect: look at top 2 cards of deck, select any number to place on
///           deck top in any order, discard the rest.
///
/// Shared by: PL!-bp3-014-PR, PL!-bp3-014-N, PL!-bp3-017-N,
///            PL!-bp3-018-N, PL!N-bp3-022-N, PL!N-bp4-016-N
///
/// Deck representation: MainDeck.cards[0] = top of deck (first drawn).
///   draw() removes from index 0. insert(0, x) places on top.
///   push(x) adds to bottom.

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

// ===== COST: Optional wait state =====

/// Q1: When optional cost is paid, the member should change to wait state.
#[test]
fn cost_paid_applies_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-bp3-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(rin);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    game.give_energy(5);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, MemberArea::LeftSide);

    // [Choice 1] Optional cost: pay (select option 1 = "pay_optional_cost")
    assert!(game.has_pending_choice(), "Should have optional cost choice");
    game.select_option(1);

    // [Choice 2] look_and_select: skip
    if game.has_pending_choice() { game.select_indices(&[]); }

    // Verify Rin is in wait state
    let orientation = game.state.get_orientation_modifier(rin).cloned();
    assert_eq!(orientation, Some("wait".to_string()),
        "Rin should be in wait state after paying optional cost");
}

/// Q2: When optional cost is skipped, the member should NOT be in wait state.
#[test]
fn cost_skipped_does_not_apply_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-bp3-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(rin);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    game.give_energy(5);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip (select option 0 = "skip_optional_cost")
    assert!(game.has_pending_choice(), "Should have optional cost choice");
    game.select_option(0);

    // [Choice 2] look_and_select: skip
    if game.has_pending_choice() { game.select_indices(&[]); }

    // Verify Rin is NOT in wait state
    let orientation = game.state.get_orientation_modifier(rin).cloned();
    assert!(orientation.is_none() || orientation == Some("active".to_string()),
        "Rin should NOT be in wait state after skipping optional cost");
}

// ===== EFFECT: Look at top 2, select any number, discard rest =====

/// Q3: Select 0 cards — both looked-at cards go to discard.
#[test]
fn select_zero_cards_both_discarded() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-bp3-014-PR");
    let card_a = game.id("PL!-bp3-017-N");
    let card_b = game.id("PL!-bp3-018-N");

    game.state.player1.hand.cards.push(rin);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(card_a);
    game.state.player1.main_deck.cards.push(card_b);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.give_energy(5);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    assert!(game.has_pending_choice(), "Should have optional cost choice");
    game.select_option(0);

    // [Choice 2] look_and_select: select 0 cards (skip)
    assert!(game.has_pending_choice(), "Should have look_and_select choice");
    game.select_indices(&[]);

    assert!(!game.has_pending_choice(), "Ability should have ended");

    // Both cards should be in discard
    assert!(game.state.player1.waitroom.cards.contains(&card_a),
        "card_a should be in discard");
    assert!(game.state.player1.waitroom.cards.contains(&card_b),
        "card_b should be in discard");
}

/// Q4: Select 1 card to put on deck top, the other goes to discard.
#[test]
fn select_one_card_other_discarded() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-bp3-014-PR");
    let card_a = game.id("PL!-bp3-017-N");
    let card_b = game.id("PL!-bp3-018-N");

    game.state.player1.hand.cards.push(rin);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(card_a);
    game.state.player1.main_deck.cards.push(card_b);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let discard_before = game.state.player1.waitroom.cards.len();

    game.give_energy(5);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    game.select_option(0);

    // [Choice 2] look_and_select: select 1 card (index 0 = card_a)
    game.select_indices(&[0]);

    // Consume any remaining pending choices from auto-re-trigger
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // card_a should be on top of deck (index 0 = top)
    assert_eq!(game.state.player1.main_deck.cards[0], card_a,
        "card_a should be on top of deck");
    assert!(game.state.player1.main_deck.cards.contains(&card_a),
        "card_a should be in deck");

    // card_b should be in discard
    assert!(game.state.player1.waitroom.cards.contains(&card_b),
        "card_b should be in discard");
    assert_eq!(game.state.player1.waitroom.cards.len(), discard_before + 1,
        "1 card should have been discarded");
}

/// Q5: Select both cards — both stay on deck top (no discard).
#[test]
fn select_both_cards_stay_on_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-bp3-014-PR");
    let card_a = game.id("PL!-bp3-017-N");
    let card_b = game.id("PL!-bp3-018-N");

    game.state.player1.hand.cards.push(rin);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(card_a);
    game.state.player1.main_deck.cards.push(card_b);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let discard_before = game.state.player1.waitroom.cards.len();

    game.give_energy(5);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    game.select_option(0);

    // [Choice 2] look_and_select: select both cards (indices 0 and 1)
    game.select_indices(&[0, 1]);

    // [Choice 3] Order choice: pick first card (index 0) to be on top
    if game.has_pending_choice() {
        game.select_option(0);
    }

    assert!(!game.has_pending_choice(), "Ability should have ended");

    // Both cards should be in deck (none discarded)
    assert!(game.state.player1.main_deck.cards.contains(&card_a),
        "card_a should be in deck");
    assert!(game.state.player1.main_deck.cards.contains(&card_b),
        "card_b should be in deck");
    assert_eq!(game.state.player1.waitroom.cards.len(), discard_before,
        "No cards should have been discarded");

    // Both should be on top (first two in deck)
    assert_eq!(game.state.player1.main_deck.cards[0], card_a,
        "card_a should be top of deck (default order)");
    assert_eq!(game.state.player1.main_deck.cards[1], card_b,
        "card_b should be second from top");
}

/// Q6: Select both cards and verify any_order placement — pick card_b on top.
#[test]
fn select_both_cards_any_order_card_b_on_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-bp3-014-PR");
    let card_a = game.id("PL!-bp3-017-N");
    let card_b = game.id("PL!-bp3-018-N");

    game.state.player1.hand.cards.push(rin);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(card_a);
    game.state.player1.main_deck.cards.push(card_b);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.give_energy(5);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    game.select_option(0);

    // [Choice 2] look_and_select: select both cards
    game.select_indices(&[0, 1]);

    // [Choice 3] Order choice: pick card_b (index 1) to be on top
    if game.has_pending_choice() {
        game.select_option(1);
    }

    assert!(!game.has_pending_choice(), "Ability should have ended");

    // card_b should be on top (index 0 in deck)
    assert_eq!(game.state.player1.main_deck.cards[0], card_b,
        "card_b should be on top of deck (any_order)");
    assert_eq!(game.state.player1.main_deck.cards[1], card_a,
        "card_a should be second from top");
}

/// Q7: Card count integrity — total cards remain unchanged after ability.
#[test]
fn card_count_integrity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-bp3-014-PR");
    let card_a = game.id("PL!-bp3-017-N");
    let card_b = game.id("PL!-bp3-018-N");

    game.state.player1.hand.cards.push(rin);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(card_a);
    game.state.player1.main_deck.cards.push(card_b);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Give energy before counting initial total (energy is a resource, not part of the 40-card deck)
    game.give_energy(5);
    let total_initial = total_cards(&mut game);

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    game.select_option(0);

    // [Choice 2] look_and_select: select 1 card
    game.select_indices(&[0]);

    let total_final = total_cards(&mut game);
    assert_eq!(total_final, total_initial, "Total card count should be preserved");
}

/// Count all cards across all zones for player1.
fn total_cards(game: &mut TestGame) -> usize {
    let p = &game.state.player1;
    p.hand.cards.len()
        + p.main_deck.cards.len()
        + p.waitroom.cards.len()
        + p.stage.stage.iter().filter(|&&id| id != -1).count()
        + p.energy_zone.cards.len()
        + p.live_card_zone.cards.len()
        + p.success_live_card_zone.cards.len()
}
