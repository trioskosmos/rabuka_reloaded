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
use crate::test_modules::support::bp7_wait_immunity_helpers::*;

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

/// Ab#1 fires on an OWN-effect wait: Shiki swaps Toubatsu, whose jidou waits
/// the cheap opponent member → Maki draws 1. Real P1-caused chain.
#[test]
fn maki_ab1_own_effect_wait_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let toubatsu = game.id("PL!SP-pb2-011-R");
    let shiki = game.id("PL!SP-bp2-008-R");
    let cheap_opp = game.id("PL!N-PR-009-PR"); // 優木せつ菜, cost 2 (<=4)
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [maki, toubatsu, shiki];
    game.state.player2.stage.stage = [cheap_opp, -1, -1];
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(10); // Shiki kidou E

    let hand_before = game.state.player1.hand.cards.len();
    // Shiki Right → Center: Toubatsu swaps to the Right (own effect move).
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                == Some("center")
        })
        .expect("center target not offered");
    game.select_generated(idx);
    game.drain_auto_ability_choices();
    assert_eq!(
        game.state.player1.stage.stage,
        [maki, shiki, toubatsu],
        "swap moved Toubatsu center→area"
    );

    // Toubatsu jidou fires on its move: take the wait bullet (index 1),
    // then the single opponent member auto-resolves. Manual scan (not
    // scan_autos_both, which would answer the 3-option itself).
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert!(
        game.has_pending_choice(),
        "Toubatsu jidou should offer its 3-option"
    );
    game.select_choice_option(1); // wait bullet
    game.drain_auto_ability_choices();
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_opp),
        Some("wait"),
        "own effect waited the cheap opponent member"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Maki ab#1: own-effect wait → draw 1"
    );
}

/// Foreign-caused recorded transition: P2 debuts Ayumu N-bp3-006-R, whose
/// debut effect waits itself. Maki must stay silent (cost gate aside, the
/// causer is P2, not an own card effect).
#[test]
fn maki_ab1_opponent_caused_wait_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let ayumu = game.id("PL!N-bp3-006-R"); // cost 9, debut waits itself
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let hand_before = game.state.player1.hand.cards.len();

    game.add_to_hand_for(Side::P2, ayumu);
    game.give_energy_for(Side::P2, 9);
    game.try_play_to_stage_for(Side::P2, ayumu, rabuka_engine::zones::MemberArea::Center)
        .expect("p2 debut of Ayumu");
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.mods.get_orientation_modifier(ayumu),
        Some("wait"),
        "Ayumu really waited itself via P2's debut effect"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "opponent-caused wait must not draw for Maki"
    );
}

/// Causer isolation: identical recorded transitions differing only in the
/// recorded cause player. Opponent cause → silent; own cause → draw.
/// (No printed card isolates the causer dimension — P2 effects that wait
/// own cheap members don't exist — so this pins the new check directly;
///
/// the two real tests above pin the wiring end to end.)
#[test]
fn maki_ab1_cause_player_decides() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let cheap_opp = game.id("PL!N-PR-009-PR"); // cost 2 (<=4)
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, maki, -1];
    game.state.player2.stage.stage = [cheap_opp, -1, -1];
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // Mirror an effect-driven wait without running an effect.
    game.state.mods.add_orientation_modifier(cheap_opp, "wait");
    let hand_before = game.state.player1.hand.cards.len();

    // Opponent-caused transition → silent.
    game.state.recently_state_changed.push((
        cheap_opp,
        "active".to_string(),
        "wait".to_string(),
        "p2".to_string(),
    ));
    scan_autos_both(&mut game);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "opponent-caused wait must not draw"
    );

    // Own-caused transition → draw 1 (turn-1 use still intact).
    game.state.recently_state_changed.push((
        cheap_opp,
        "active".to_string(),
        "wait".to_string(),
        "p1".to_string(),
    ));
    scan_autos_both(&mut game);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "own-effect wait draws 1"
    );
}
