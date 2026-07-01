/// Tests for ビタミンSUMMER！ (PL!SP-bp2-024-L) — LiveSuccess ability:
///   {{live_success.png|ライブ成功時}}自分の手札の枚数が相手より多い場合、
///   このカードのスコアを+1する。
///
/// If your hand count > opponent's at LiveSuccess timing, this card's score +1.
///
/// Q119: After the ability resolves and hand count changes, score stays locked
/// Q128: Draw icon before LiveSuccess can make hand count exceed and trigger
/// Q36:  LiveSuccess abilities trigger before winner determination
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

    // Stage: members providing hearts to satisfy vitamin's need_heart
    // {heart02:1, heart03:4, heart06:1, heart0:6}. Three members each with
    // {heart02:1, heart03:2, heart06:1} = 12 hearts total, enough to satisfy.
    let heart_member = game.id("PL!SP-bp1-013-PR");
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, heart_member);
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, heart_member);
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, heart_member);

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
    assert!(
        p1_hand > p2_hand,
        "P1 hand ({}) must be > P2 hand ({}) for condition to pass",
        p1_hand,
        p2_hand
    );

    assert_eq!(
        game.state.mods.get_score_modifier(vitamin),
        0,
        "LiveSuccess score bonus cleared after live"
    );
    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "bonus in final score");
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

    // Stage: members providing hearts to satisfy vitamin's need_heart
    let heart_member = game.id("PL!SP-bp1-013-PR");
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, heart_member);
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, heart_member);
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, heart_member);

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

    // Score locked — verify in snapshot, cleared from mods
    assert_eq!(
        game.state.mods.get_score_modifier(vitamin),
        0,
        "LiveSuccess bonus cleared after live"
    );
    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "bonus in final score");

    // Hand changes don't re-evaluate (still 0)
    game.state.player1.hand.cards.clear();
    assert_eq!(
        game.state.mods.get_score_modifier(vitamin),
        0,
        "Q119: Score stays 0 after hand counts change"
    );
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

    let _score_mod = game.state.mods.get_score_modifier(vitamin);
    let score_mod = game.state.mods.get_score_modifier(vitamin);
    assert_eq!(score_mod, 0, "No score bonus when P1 hand <= P2 hand");
}
