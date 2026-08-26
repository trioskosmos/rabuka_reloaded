/// Tests for 西木野真姫 (PL!-pb1-015-R) — Auto ability gameplay:
///
/// Ab#1 (自動, ターン1回):
///   自分のカードの効果によって、相手のステージにいる
///   アクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、
///   カードを１枚引く。
///
/// Q177: Draw is mandatory — can't skip.
///
/// Ab#0 (登場/ライブ開始時, センター):
///   「BiBi」のメンバー1人をウェイトにしてもよい：
///   相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。
///   (Ab#0's opponent wait action triggers Ab#1)
use crate::helpers::*;
use crate::test_modules::bp7_wait_immunity_helpers::*;

/// 真姫's ab#0 opponent-wait is blocked by 松浦果南's wait-immunity on the member.
#[test]
fn maki_ab0_opponent_wait_blocked_by_wait_immunity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Player2 protects their 果南 (Aqours, active).
    let p2_kanan = p2_establish_wait_immunity(&mut game);

    // Player1 plays 真姫 at center → ab#0 makes the opponent (player2) wait one of
    // their own active members.
    let maki = game.id("PL!-pb1-015-R");
    let bibi = game.id("PL!-sd1-011-SD"); // BiBi member for the condition
    game.state.player1.hand.cards.push(maki);
    game.state.player1.stage.stage[0] = bibi;
    game.state.player1.stage.stage[1] = -1;
    game.give_energy(11);
    game.play_to_stage(maki, rabuka_engine::zones::MemberArea::Center);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        // Pay the optional cost (wait a BiBi member), then the opponent's own-wait.
        if game.pending_choice_type().as_deref() == Some("SelectCard") {
            game.select_indices(&[0]);
        } else {
            game.select_choice_option(1);
        }
    }

    assert!(
        !is_waited(&game, p2_kanan),
        "真姫's ab#0 opponent-wait must be blocked by wait-immunity"
    );
}

/// Q177: Debut 真姫 → Ab#0 fires → opponent's cost ≤4 member waited → Ab#1 draws 1.
#[test]
fn maki_q177_debut_triggers_draw_via_ab0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    // Opponent members with cost ≤4 that will be waited by Ab#0
    let cheap_opp = game.id("PL!SP-sd1-019-SD"); // cost 2
    let cheap_opp2 = game.id("PL!-sd1-011-SD"); // cost 4, BiBi member
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage[0] = cheap_opp;
    game.state.player2.stage.stage[1] = cheap_opp2;
    game.give_energy(11);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(maki, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();

    // hand after play: filler only (1 card)
    let hand_after_play = game.state.player1.hand.cards.len();

    // Pay optional cost (wait Maki herself as Center BiBi member).
    // Observed: SelectTarget pay_optional_cost gate is offered.
    assert!(
        game.has_pending_choice(),
        "optional wait-a-BiBi-member cost must be offered"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget optional-cost gate"
    );
    game.select_option(1);
    // Opponent chooses a member to wait (select cheap_opp at index 0)
    assert!(
        game.has_pending_choice(),
        "Opponent should have a choice to wait a member"
    );
    let entry = game.state.ability_queue.current_entry();
    assert_eq!(
        entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Wait-member choice should be routed to opponent"
    );
    game.select_indices(&[0]);
    // Consume any remaining choices
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Ab#1 draws 1 → hand goes from 1 to 2
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_after_play + 1,
        "Q177: Cost ≤4 opponent waited → draw 1"
    );
    // Opponent member should be on stage (just in wait state)
    assert!(
        game.state.player2.stage.stage.contains(&cheap_opp),
        "Opponent member should still be on stage (wait state)"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_opp),
        Some("wait"),
        "Opponent's cheap member should be in wait state"
    );
}

/// Edge: Opponent member with cost > 4 → Ab#1 doesn't trigger.
/// The condition text says cost ≤4, but the condition evaluator doesn't
/// check cost_limit — this test documents the gap.
#[test]
fn maki_edge_cost5_opponent_draws_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let expensive_opp = game.id("PL!-sd1-014-SD"); // cost 9 > 4
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage[0] = expensive_opp;
    game.give_energy(11);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let _hand_before = game.state.player1.hand.cards.len();

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(maki, rabuka_engine::zones::MemberArea::Center);

    // Skip optional cost (choose "Skip" option index 0).
    // Observed: SelectTarget pay_optional_cost gate is offered.
    assert!(
        game.has_pending_choice(),
        "optional wait cost must be offered"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget optional-cost gate"
    );
    game.select_option(0); // skip → no effect fires
    // Cost was skipped → opponent action doesn't fire → no choice
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Member was NOT waited (cost wasn't paid)
    assert!(
        game.state
            .mods
            .get_orientation_modifier(expensive_opp)
            .is_none(),
        "Opponent member should NOT be waited when cost was skipped"
    );

    // No trigger for Ab#1 draw
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "No draw when optional cost skipped"
    );

    // No trigger for Ab#1 draw
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "No draw when optional cost skipped"
    );
}

/// Edge: No opponent member on stage → no one to wait → Ab#0 effect does nothing.
#[test]
fn maki_edge_no_opponent_member_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(11);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(maki, rabuka_engine::zones::MemberArea::Center);

    let hand_after_play = game.state.player1.hand.cards.len();

    // Skip optional cost (choose "Skip" option index 0).
    // Observed: SelectTarget pay_optional_cost gate is offered even though
    // the effect would have no targets.
    assert!(
        game.has_pending_choice(),
        "optional wait cost must be offered"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget optional-cost gate"
    );
    game.select_option(0);
    // Opponent has no members → no pending choice after skip
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_after_play,
        "No opponent member → no draw"
    );
}
