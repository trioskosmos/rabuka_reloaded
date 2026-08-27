//! Integration blind spots — behaviors the per-ability inventory cannot see.
//!
//! Three classes:
//! 1. LiveEnd modifier expiry is characterized by calling
//!    `check_expired_effects()` DIRECTLY (modifier_layer_characterization_test)
//!    — nothing proved the REAL victory→Active rollover (phases.rs) invokes
//!    it. T1 drives the actual phase machine.
//! 2. Dual-trigger cards (登場/ライブ開始時) whose second window is never
//!    fired anywhere: PL!N-bp5-004-R 朝香果林 (only Debut tested) and
//!    PL!HS-bp6-004-R 百生吟子 (only ライブ開始時 tested).
//! 3. Gate boundaries on those waits: original-blade==4 (exact match) and
//!    cost<=9.

use crate::helpers::*;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

fn orientation(game: &TestGame, cid: i16) -> Option<CardOrientation> {
    game.state.mods.orientation_modifiers.get(&cid).copied()
}

/// Main → LiveCardSet(1st): 5 passes, no draws in between on P1's seeded deck.
fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// LiveCardSet(2nd) → performances: LiveStart abilities fire here.
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// T1 — a live_end grant made during a REAL live must be gone when the turn
/// rolls over to Active via genuine pass()es (victory determination included),
/// not via a hand-called check_expired_effects().
#[test]
fn live_end_grant_expires_through_real_phase_rollover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-cl1-006-CL"); // 登場: +3 blades until live end
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage[0] = me;
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(live);

    // Grant INSIDE the live phase (real ability, established fire_trigger idiom).
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        3,
        "positive control: grant registered"
    );

    // Real rollover: LiveCardSet → performances → victory determination → Active.
    advance_to_live_start(&mut game);
    let mut guard = 0;
    while game.state.current_turn_phase == rabuka_engine::game_state::TurnPhase::Live && guard < 12 {
        guard += 1;
        game.pass();
        game.drain_auto_ability_choices();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }

    assert_ne!(
        game.state.current_turn_phase,
        rabuka_engine::game_state::TurnPhase::Live,
        "flow must have crossed the victory determination rollover"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "the REAL rollover (phases.rs) must expire live_end grants, not just direct calls"
    );
    assert!(game.state.temporary_effects.is_empty());
}

/// T2 — 果林 PL!N-bp5-004-R (登場/ライブ開始時): 「相手のステージにいる元々…
/// ブレードの数がちょうど4つのメンバー1人をウェイトにする」.
/// Her ライブ開始時 window is never exercised anywhere. Drive BOTH windows in
/// one real flow: debut whiffs (no blade-4 target yet), the member arrives,
/// then the LIVE START window must do the rest.
///
/// Gate edge: 「ちょうど4つ」 is an EXACT match — blade-1 filler is immune.
#[test]
fn karin_live_start_window_rests_exactly_blade4_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp5-004-R");
    let target = game.id("PL!N-PR-008-PR"); // 近江彼方: original blade exactly 4, no live-start triggers of her own
    let filler = game.id("PL!-sd1-010-SD"); // blade 1 — gate excludes
    let live = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game, filler);
    game.give_energy(20);
    game.state.player1.hand.cards.push(karin);
    game.state.player1.hand.cards.push(live);

    // Debut happens NOW, while P2 has no stage members — window 1 whiffs.
    game.play_to_stage(karin, MemberArea::Center);
    game.drain_auto_ability_choices();

    // Mid-turn main-phase actions shift which seat the fixed 5-pass walk
    // lands on — drive until P1's LiveCardSet window explicitly.
    let mut guard = 0;
    while game.state.current_phase
        != rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
        && guard < 10
    {
        guard += 1;
        game.pass();
    }
    game.set_live_card(live);

    // The blade-4 member arrives AFTER the debut window, BEFORE live start.
    // Direct placement: she is scenery for the trigger-timing test, not a
    // participant of any debut pipeline.
    game.state.player2.stage.stage[0] = target;
    game.state.player2.stage.stage[1] = filler;

    advance_to_live_start(&mut game);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        use rabuka_engine::ability::types::Choice;
        match game.get_pending_choice() {
            // SELECT the offered auto abilities — an empty answer would
            // decline the very live-start window under test.
            Choice::SelectAutoAbility { .. } => {
                let n = game.pending_choice_count();
                let idxs: Vec<usize> = (0..n.max(1) as usize).collect();
                game.select_indices(&idxs);
            }
            Choice::SelectTarget { .. } => game.select_option(1),
            Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        orientation(&game, target),
        Some(CardOrientation::Wait),
        "the ライブ開始時 window must rest the exactly-blade-4 member"
    );
    assert_ne!(
        orientation(&game, filler),
        Some(CardOrientation::Wait),
        "「ちょうど4つ」: blade-1 member is not a legal target"
    );
}

/// T3 — 吟子 PL!HS-bp6-004-R ab#0 (登場/ライブ開始時): 「相手のステージにいる
/// コスト9以下のメンバー1人をウェイトにする」. Only her ライブ開始時 window is
/// tested anywhere; here the DEBUT window runs through a real play_to_stage,
/// with a cost gate negative (cost-15 member immune).
#[test]
fn ginko_hs_debut_window_rests_cost_le9_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let gin = game.id("PL!HS-bp6-004-R");
    let cheap = game.id("PL!-sd1-010-SD"); // cost 4 — eligible
    let huge = game.id("PL!HS-bp5-004-R"); // cost 15 — gate blocks
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player2.stage.stage = [cheap, huge, -1];
    fill_decks(&mut game, filler);
    game.give_energy(20);
    game.add_to_hand(gin);

    game.play_to_stage(gin, MemberArea::Center);

    // Single eligible candidate (cheap) auto-resolves; no prompt may remain.
    game.drain_auto_ability_choices();
    assert!(
        !game.has_pending_choice(),
        "single-candidate wait must auto-resolve without dangling prompts"
    );

    assert_eq!(
        orientation(&game, cheap),
        Some(CardOrientation::Wait),
        "debut window rests the cost-9-or-less member"
    );
    assert_ne!(
        orientation(&game, huge),
        Some(CardOrientation::Wait),
        "cost gate: the cost-15 member is not a legal target"
    );
}
