/// Tests for Poppin' Up! (PL!N-bp1-026-L) — LiveSuccess ability.
/// Stage: 2× PL!-PR-012-PR (heart03=1, blade=2) → heart03=2, 1 surplus.
/// Cheer draws 4 cards from index 0. Ball_card (b_all=1) provides +1 wildcard.
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
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn deck_push(game: &mut TestGame, cards: &[i16]) -> i16 {
    let filler = game.id("PL!-sd1-010-SD");
    for &c in cards {
        game.state.player1.main_deck.cards.push(c);
    }
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    filler
}

/// Test 4: P1 score > P2 score → condition passes → 虹ヶ咲 added to hand
#[test]
fn poppin_q66_has_cards_beats_no_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-PR-012-PR");
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player2.stage.stage = [member, member, -1];

    let poppin = game.id("PL!N-bp1-026-L");
    let niji = game.id("PL!N-pb1-005-R");
    let ball = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(poppin);
    // Cheer draws from index 0. ball first (=b_all wildcard), niji second.
    game.state.player1.main_deck.cards.push(ball);
    game.state.player1.main_deck.cards.push(niji);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(poppin);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&poppin));
    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "虹ヶ咲 should be moved from revealed → hand"
    );
}

/// Test 5: P1 == P2 score → condition fails
#[test]
fn poppin_equal_score_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-PR-012-PR");
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player2.stage.stage = [member, member, -1];

    let poppin = game.id("PL!N-bp1-026-L");
    let niji = game.id("PL!N-pb1-005-R");
    let ball = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(poppin);
    game.state.player1.main_deck.cards.push(ball);
    game.state.player1.main_deck.cards.push(niji);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    let p2_live = game.id("PL!-sd1-019-SD");
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(p2_live);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(poppin);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert!(
        !game.state.player1.hand.cards.contains(&niji),
        "Equal score → condition should fail"
    );
}

/// Test 6: P1 score < P2 score → condition fails
#[test]
fn poppin_lower_score_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-PR-012-PR");
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player2.stage.stage = [member, member, -1];

    let poppin = game.id("PL!N-bp1-026-L");
    let niji = game.id("PL!N-pb1-005-R");
    let ball = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(poppin);
    game.state.player1.main_deck.cards.push(ball);
    game.state.player1.main_deck.cards.push(niji);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    let p2_a = game.id("PL!-sd1-019-SD");
    let p2_b = game.new_id("PL!-sd1-019-SD");
    game.state.player2.success_live_card_zone.cards.push(p2_a);
    game.state.player2.success_live_card_zone.cards.push(p2_b);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(poppin);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert!(
        !game.state.player1.hand.cards.contains(&niji),
        "P1 1 < P2 2 → condition should fail"
    );
}

/// Test 7: P1 multiple live cards (total=2) < P2 (total=3) → fails
#[test]
fn poppin_multiple_cards_lower_total_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-PR-012-PR");
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player2.stage.stage = [member, member, -1];

    let poppin = game.id("PL!N-bp1-026-L");
    let niji = game.id("PL!N-pb1-005-R");
    let ball = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(poppin);
    game.state.player1.main_deck.cards.push(ball);
    game.state.player1.main_deck.cards.push(niji);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    let p1_extra = game.id("PL!-sd1-019-SD");
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(p1_extra);

    let p2_a = game.id("PL!-sd1-019-SD");
    let p2_b = game.new_id("PL!-sd1-019-SD");
    let p2_c = game.new_id("PL!-sd1-019-SD");
    game.state.player2.success_live_card_zone.cards.push(p2_a);
    game.state.player2.success_live_card_zone.cards.push(p2_b);
    game.state.player2.success_live_card_zone.cards.push(p2_c);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(poppin);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert!(
        !game.state.player1.hand.cards.contains(&niji),
        "P1 2 < P2 3 → condition should fail"
    );
}

/// Test 8: Multiple 虹ヶ咲 revealed → picks exactly 1
#[test]
fn poppin_multiple_niji_revealed_picks_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-PR-012-PR");
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player2.stage.stage = [member, member, -1];

    let poppin = game.id("PL!N-bp1-026-L");
    let niji_a = game.id("PL!N-pb1-005-R");
    let niji_b = game.id("PL!N-pb1-002-R");
    let ball = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(poppin);
    game.state.player1.main_deck.cards.push(ball);
    game.state.player1.main_deck.cards.push(niji_a);
    game.state.player1.main_deck.cards.push(niji_b);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(poppin);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert_eq!(
        game.state.player1.hand.cards.len() - hand_before,
        1,
        "Exactly 1 card should be added to hand"
    );
}

/// Test 9: No 虹ヶ咲 revealed → effect has no valid target
#[test]
fn poppin_no_niji_revealed_effect_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-PR-012-PR");
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player2.stage.stage = [member, member, -1];

    let poppin = game.id("PL!N-bp1-026-L");
    let ball = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(poppin);
    game.state.player1.main_deck.cards.push(ball);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(poppin);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert!(
        game.state.player1.hand.cards.len() <= hand_before,
        "No 虹ヶ咲 → hand should not grow"
    );
}

/// Test 10: Mixed 虹ヶ咲 + non-虹ヶ咲 → picks only 虹ヶ咲
#[test]
fn poppin_mixed_revealed_picks_niji_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-PR-012-PR");
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player2.stage.stage = [member, member, -1];

    let poppin = game.id("PL!N-bp1-026-L");
    let niji = game.id("PL!N-pb1-005-R");
    let non_niji = game.id("PL!-PR-001-PR");
    let ball = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(poppin);
    game.state.player1.main_deck.cards.push(ball);
    game.state.player1.main_deck.cards.push(niji);
    game.state.player1.main_deck.cards.push(non_niji);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(poppin);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "虹ヶ咲 should be added to hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&non_niji),
        "Non-虹ヶ咲 should NOT be added"
    );
}
