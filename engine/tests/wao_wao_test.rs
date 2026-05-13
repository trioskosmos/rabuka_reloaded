/// Tests for WAO-WAO Powerful day! (PL!-pb1-028-L) — LiveStart ability:
///
/// WAO-WAO is a live card (type=ライブ) with a LiveStart trigger.
/// It cannot be placed on stage — it's used as the live card during performance.
///
/// {{live_start.png|ライブ開始時}}自分のステージにいる『Printemps』のメンバーをアクティブにする。
/// これによりウェイト状態のメンバーが3人以上アクティブ状態になったとき、このカードのスコアを＋１する。
///
/// Q178: Can you activate multiple members with the first effect? A: Yes.
/// Q179: If you already have 3+ wait members on stage, can you skip the
///       activation and still get +1 score? A: No — you must actually change
///       3+ wait members to active via this effect.
///
/// Stage has 3 slots: [left, center, right]. All 3 must be Printemps members
/// for the condition to be met, since WAO-WAO is a live card not on stage.

mod helpers;
use helpers::*;

fn advance_to_live_start(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

/// Q178: 3 Printemps members in wait on stage → all become active → score +1.
#[test]
fn wao_wao_q178_activate_3_printemps_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wao_wao = game.id("PL!-pb1-028-L");
    let printemps_a = game.id("PL!-sd1-001-SD");
    let printemps_b = game.id("PL!-sd1-003-SD");
    let printemps_c = game.id("PL!-sd1-008-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 3 Printemps members in wait (left, center, right)
    game.state.player1.stage.stage = [printemps_a, printemps_b, printemps_c];
    game.state.mods.add_orientation_modifier(printemps_a, "wait");
    game.state.mods.add_orientation_modifier(printemps_b, "wait");
    game.state.mods.add_orientation_modifier(printemps_c, "wait");

    // Hand: WAO-WAO as the live card for this performance
    game.state.player1.hand.cards.push(wao_wao);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_start(&mut game);
    game.set_live_card(wao_wao);
    game.pass();
    game.pass();

    // All 3 Printemps should now be active ("active" string modifier = active state)
    let o_a = game.state.mods.get_orientation_modifier(printemps_a);
    assert!(o_a.map(|s| s.as_str()) == Some("active"),
        "printemps_a should be active, got {:?}", o_a);
    let o_b = game.state.mods.get_orientation_modifier(printemps_b);
    assert!(o_b.map(|s| s.as_str()) == Some("active"),
        "printemps_b should be active, got {:?}", o_b);
    let o_c = game.state.mods.get_orientation_modifier(printemps_c);
    assert!(o_c.map(|s| s.as_str()) == Some("active"),
        "printemps_c should be active, got {:?}", o_c);

    // 3 wait→active → score +1
    let score_mod = game.state.mods.get_score_modifier(wao_wao);
    assert_eq!(score_mod, 1,
        "3 wait members activated → score +1");
}

/// Q178 variant: 3 Printemps all already active → 0 changed → no score.
#[test]
fn wao_wao_q178_already_active_no_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wao_wao = game.id("PL!-pb1-028-L");
    let printemps_a = game.id("PL!-sd1-001-SD");
    let printemps_b = game.id("PL!-sd1-003-SD");
    let printemps_c = game.id("PL!-sd1-008-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // All 3 active (no orientation modifiers)
    game.state.player1.stage.stage = [printemps_a, printemps_b, printemps_c];

    game.state.player1.hand.cards.push(wao_wao);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_start(&mut game);
    game.set_live_card(wao_wao);
    game.pass();
    game.pass();

    let score_mod = game.state.mods.get_score_modifier(wao_wao);
    assert_eq!(score_mod, 0,
        "0 wait members changed → no score boost");
}

/// Q179: 2 wait + 1 active on stage → only 2 changed → no score.
#[test]
fn wao_wao_q179_only_2_wait_to_active_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wao_wao = game.id("PL!-pb1-028-L");
    let printemps_a = game.id("PL!-sd1-001-SD");
    let printemps_b = game.id("PL!-sd1-003-SD");
    let printemps_c = game.id("PL!-sd1-008-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 2 wait + 1 already active
    game.state.player1.stage.stage = [printemps_a, printemps_b, printemps_c];
    game.state.mods.add_orientation_modifier(printemps_a, "wait");
    game.state.mods.add_orientation_modifier(printemps_b, "wait");
    // printemps_c stays active (no modifier)

    game.state.player1.hand.cards.push(wao_wao);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_start(&mut game);
    game.set_live_card(wao_wao);
    game.pass();
    game.pass();

    // Only 2 changed from wait→active → condition requires 3+ → no score
    let score_mod = game.state.mods.get_score_modifier(wao_wao);
    assert_eq!(score_mod, 0,
        "Only 2 wait members changed, condition requires 3+");
}
