use crate::helpers::*;

/// Edge: ド！ド！ド！ (live card, unit=みらくらぱーく！) among top 5 → selectable.
/// Must pay the hand-discard cost, then select the card to reveal.
#[test]
fn optional_discard_look_five_selects_unit_live_q82_bp1_023() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    let dodo = game.id("PL!HS-bp1-023-L");
    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    game.state.player1.main_deck.cards.insert(0, dodo);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    // Pay cost
    assert!(
        game.has_pending_choice(),
        "Should have optional cost choice"
    );
    game.select_indices(&[0]);

    // Look_and_select: select the matching card from looked_at
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    game.select_indices(&[0]);

    // Resolve any remaining sub-choices (reveal, move to hand, discard rest)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        game.state.player1.hand.cards.contains(&dodo),
        "Q82: ド！ド！ド！ (みらくらぱーく！) is selectable"
    );
}

/// Edge: アイデンティティ (live card, unit=みらくらぱーく！) selectable.
#[test]
fn optional_discard_look_five_selects_unit_live_q82_pr_012() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    let identity = game.id("PL!HS-PR-012-PR");
    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    game.state.player1.main_deck.cards.insert(0, identity);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    assert!(
        game.has_pending_choice(),
        "Should have optional cost choice"
    );
    game.select_indices(&[0]); // pay cost
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    game.select_indices(&[0]); // select identity
    while game.has_pending_choice() {
        game.select_indices(&[]);
    } // resolve remaining

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        game.state.player1.hand.cards.contains(&identity),
        "Q82: アイデンティティ (みらくらぱーく！) is selectable"
    );
}

/// Edge: No みらくらぱーく！ card among top 5 → nothing to reveal.
#[test]
fn optional_discard_look_five_no_unit_match_skips_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    assert!(
        game.has_pending_choice(),
        "optional hand-discard cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the cost"
    );
    game.select_indices(&[0]); // pay cost
      // Look_and_select shows 5 cards (group filter not applied at select level)
      // No みらくらぱーく！ among the top 5 → no looked_at reveal prompt appears.
    assert!(
        !game.has_pending_choice(),
        "no mirakurapark match -> looked_at selection must be auto-skipped"
    );

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "No cards in hand: himeno played, filler paid"
    );
}
