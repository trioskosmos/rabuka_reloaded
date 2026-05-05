/// Tests for ビタミンSUMMER！ (PL!SP-bp2-024-L) — LiveSuccess ability:
///   {{live_success.png|ライブ成功時}}自分の手札の枚数が相手より多い場合、
///   このカードのスコアを+1する。
///
/// If your hand count > opponent's at LiveSuccess timing, this card's score +1.
///
/// Q119: After the ability resolves and hand count changes, score stays locked
/// Q128: Draw icon before LiveSuccess can make hand count exceed and trigger
/// Q36:  LiveSuccess abilities trigger before winner determination

mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Q128: When P1 hand > P2 hand at LiveSuccess timing, score +1 is applied.
/// The condition is checked at the LiveSuccess trigger moment, not at card's debut.
/// Set up P1 with more cards in hand than P2 so the comparison_condition passes.
#[test]
fn vitamin_q128_hand_greater_triggers_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let vitamin = game.id("PL!SP-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    // P1 hand: 2 cards (vitamin + filler). P2 hand: empty. P1 > P2.
    game.state.player1.hand.cards.push(vitamin);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.clear();

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(vitamin);
    advance_to_live_start(&mut game);

    // Pass through FirstAttacker → SecondAttacker → LiveVictoryDetermination → Active
    game.pass();
    game.pass();
    game.pass();

    let p1_hand = game.state.player1.hand.cards.len();
    let p2_hand = game.state.player2.hand.cards.len();
    assert!(p1_hand > p2_hand,
        "P1 hand ({}) must be > P2 hand ({}) for condition to pass", p1_hand, p2_hand);

    let score_mod = game.state.get_score_modifier(vitamin);
    assert_eq!(score_mod, 1,
        "Q128: Score +1 when P1 hand > P2 hand at LiveSuccess");
}

/// Q119: Score increase is locked in at LiveSuccess resolution time.
/// Even if hand counts change after the ability resolves, the already-applied
/// score modifier does not change. This test verifies the score stays +1
/// after the ability has resolved and the card moves to success zone.
#[test]
fn vitamin_q119_score_locked_after_resolution() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let vitamin = game.id("PL!SP-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(vitamin);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.clear();

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(vitamin);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    // Score should be +1 after LiveSuccess resolution
    assert_eq!(game.state.get_score_modifier(vitamin), 1,
        "Q119: Score should be +1 after LiveSuccess ability resolves");

    // Now change hand counts dramatically — the score modifier should NOT change
    // because the condition was already evaluated and won't re-evaluate
    game.state.player1.hand.cards.clear();

    // The score modifier is locked in from the LiveSuccess evaluation
    // Even if P1 hand < P2 hand now, the already-applied score stays
    assert_eq!(game.state.get_score_modifier(vitamin), 1,
        "Q119: Score stays +1 even after hand counts change post-resolution");
}

/// Negative: P1 hand <= P2 hand → comparison fails → no score bonus.
#[test]
fn vitamin_hand_less_or_equal_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let vitamin = game.id("PL!SP-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    // P1 hand: 1 card (vitamin only)
    game.state.player1.hand.cards.push(vitamin);

    // P2 hand: 2 cards (P2 > P1)
    game.state.player2.hand.cards.push(filler);
    game.state.player2.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(vitamin);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    let score_mod = game.state.get_score_modifier(vitamin);
    let score_mod = game.state.get_score_modifier(vitamin);
    assert_eq!(score_mod, 0,
        "No score bonus when P1 hand <= P2 hand");
}

/// Verify the ability is parsed correctly.
#[test]
fn vitamin_ability_parsed() {
    let db = load_real_database();
    let vitamin = db.get_card_by_no("PL!SP-bp2-024-L")
        .expect("ビタミンSUMMER！ should exist");
    let ab0 = &vitamin.abilities[0];

    assert_eq!(ab0.triggers.as_deref(), Some("ライブ成功時"),
        "Trigger should be ライブ成功時 (LiveSuccess)");

    let effect = ab0.effect.as_ref().expect("Should have effect");
    assert_eq!(effect.action, "modify_score");
    assert_eq!(effect.operation.as_deref(), Some("add"));
    assert_eq!(effect.value, Some(1));
    assert_eq!(effect.self_target, Some(true));

    let condition = effect.condition.as_ref().expect("Should have condition");
    assert_eq!(condition.condition_type.as_deref(), Some("comparison_condition"),
        "Should be comparison_condition");
    assert_eq!(condition.comparison_target.as_deref(), Some("opponent"),
        "Target should be opponent");
    assert_eq!(condition.location.as_deref(), Some("hand"),
        "Should compare hand counts");
    assert_eq!(condition.operator.as_deref(), Some(">"),
        "Should use > operator");
}
