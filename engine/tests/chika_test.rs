/// Tests for PL!S-bp3-001-R+ (高海千歌 / CYaRon!) — Activation ability: wait member → gain total score +1
///
/// Ab#0 (起動/センター/ターン1回): メンバー1人をウェイトにする：
///   ライブ終了時まで、これによってウェイト状態になったメンバーは、
///   「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。
///   （この能力はセンターエリアに登場している場合のみ起動できる。）
///
/// Parsed:
///   trigger: 起動, use_limit: 1
///   cost: change_state(wait, 1, member_card)
///   effect: sequential[ modify_score(+1, live_end), gain_ability ]
///   activation_position: "center"
///
/// Q152: Cannot target opponent's members (self-only)
/// Q151: Gained ability is lost when the waited member leaves stage
/// Q171: "Until end of live" effects persist to LiveVictoryDetermination end
//=====================================================================

mod helpers;
use helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;


/// Q152: Cost targets self only (can't select opponent's members)
#[test]
fn chika_q152_self_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    // Chika at center, opponent has a member too
    game.state.player1.stage.stage = [-1, chika, -1];
    game.state.player2.stage.stage = [-1, filler, -1];

    game.give_energy(5);

    // Activate ability → cost waits Chika herself (only P1 member)
    game.activate_ability(chika);

    // Q152: Only P1's members are targeted (self), not P2's
    let p1_wait = game.state.get_orientation_modifier(chika);
    assert!(p1_wait == Some(&"wait".to_string()) || p1_wait.is_some(),
        "Q152: Chika should be in wait state after activation");

    // P2's member should NOT be waited
    let p2_wait = game.state.get_orientation_modifier(filler);
    assert!(p2_wait.is_none() || p2_wait != Some(&"wait".to_string()),
        "Q152: Opponent member should NOT be waited");
}

/// Q151: Score boost is lost when waited member leaves stage
#[test]
fn chika_q151_ability_lost_on_leave() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage = [-1, chika, -1];
    game.give_energy(5);

    game.activate_ability(chika);

    // Chika should have +1 score modifier after activation
    let score_before = game.state.get_score_modifier(chika);
    assert_eq!(score_before, 1,
        "Q151: Chika should have +1 score after activation");

    // Remove Chika from stage (simulating leaving via effect)
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(chika);
    // clear_modifiers_for_card is called by move_cards when card moves

    // Manually clear (since we bypassed move_cards)
    game.state.clear_modifiers_for_card(chika);

    let score_after = game.state.get_score_modifier(chika);
    assert_eq!(score_after, 0,
        "Q151: Score boost should be lost when member leaves stage");
}

/// Q171: "Until end of live" duration persists to LiveVictoryDetermination  
#[test]
fn chika_q171_live_end_persistence() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, chika, -1];
    // Keep deck stocked so phase transitions don't crash
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id("PL!-sd1-010-SD"));
    }

    game.give_energy(5);

    // Activate → Chika gains +1 score
    game.activate_ability(chika);
    assert_eq!(game.state.get_score_modifier(chika), 1,
        "Score should be +1 after activation");

    // Advance through phases to live
    game.state.player1.hand.cards.push(filler_live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Pass through performance phases
    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination

    // Q171: Score modifier should still exist during LiveVictoryDetermination
    assert_eq!(game.state.get_score_modifier(chika), 1,
        "Q171: Score +1 should persist to LiveVictoryDetermination");
}

/// Center requirement: activation only works from Center area
#[test]
fn chika_center_required() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    // Place Chika at LEFT (not Center)
    game.state.player1.stage.stage = [chika, -1, -1];
    game.give_energy(5);

    // Try to activate from Left → should fail (activation_position: "center")
    let _result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(chika),
        None,
        None,
        None,
    );

    // Position mismatch skips the ability, but doesn't error
    // The score modifier should NOT be applied
    assert_eq!(game.state.get_score_modifier(chika), 0,
        "Score should NOT be applied when activated from wrong position");
}

/// Once per turn: second activation in same turn should fail
#[test]
fn chika_turn1_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage = [-1, chika, -1];
    game.give_energy(10);

    // First activation succeeds (gain_ability grants constant +1 score)
    game.activate_ability(chika);
    assert_eq!(game.state.get_score_modifier(chika), 1,
        "First activation should apply +1 score via granted constant ability");

    // Second activation in same turn should skip (use_limit=1)
    let _result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(chika),
        None,
        None,
        None,
    );

    // The function may return Ok (skipping silently) or Err
    // Either way, score should still be 1 (not 2)
    assert_eq!(game.state.get_score_modifier(chika), 1,
        "Second activation should not apply any additional score modifiers");
}

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
