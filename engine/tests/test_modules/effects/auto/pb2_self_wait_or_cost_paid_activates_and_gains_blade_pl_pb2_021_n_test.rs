//! 南ことり `PL!-pb2-021-N` (自動, ターン1回)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   自動／ターン1回 — このメンバーが、自分のカードの能力のコストを支払うため**か**、
//!   自分のカードの効果によってウェイト状態になったとき、このメンバーをアクティブにし、
//!   ライブ終了時まで、ブレードを得る。
//!
//! In English: once per turn, when this member EITHER paid a cost for one of my other
//! abilities, OR was put into wait by one of my own card's effects, she becomes
//! active and gains a blade until the end of the live.
//!
//! # Both trigger branches are EVENTS, and both used to be tautologies
//!
//! This ability was over-firing: she gained a blade EVERY turn whether or not anything had
//! happened, because the `or_condition` was satisfied unconditionally. Both halves were the
//! same defect, and it is the same defect the engine had already fixed once for a sibling
//! card — see the `cards_activated_this_turn` field and its comment at
//! `core/game_state/mod.rs`, added for 「このターン中に…アクティブにしていた」.
//!
//! **Branch 1, 「このメンバーが、自分のカードの能力のコストを支払うため」.** A per-turn
//! history of who PAID a cost, not a comparison on cost. The parser had nowhere to put an
//! event, so it fell through to `comparison_condition` with `comparison_type: "cost"` and
//! **no operator and no value**. `compare_counts` reads a missing operator as `>=`
//! (`ability/util.rs`), and with no bound to compare against that is `cost >= 0` — true for
//! every card, forever. Now parsed to `paid_cost_this_turn: true`, backed by the new
//! `GameState::cards_paid_cost_this_turn` per-turn set, written by
//! `GameState::record_cost_paid()` at the cost-payment sites and cleared with the other
//! per-turn trackers.
//!
//! **Branch 2, 「自分のカードの効果によってウェイト状態になったとき」.** Also an event, but it
//! parsed to a static `state_condition {state: wait}` — 「IS in wait」, which is satisfied for
//! as long as she sits waited, by any means, and says nothing about the cause. Now a
//! turn-scoped transition: `from: active`, `to: wait`, `temporal: this_turn`, resolved
//! against `GameState::turn_state_changes` — the same attributed, turn-scoped log the Q203
//! path uses.
//!
//! A consequence worth stating plainly, because it looks like a bug and is not: **putting
//! her in wait by writing the orientation directly no longer triggers her.** A hand-written
//! orientation is not an EVENT — there is no `turn_state_changes` entry, so branch 2 is
//! correctly false, and with branch 1 equally false the 自動 does not fire. That is the
//! printed meaning, and `being_in_wait_without_a_transition_does_not_trigger_it` holds the
//! engine to it.
//!
//! The parser rule for branch 2 is deliberately narrow. Three more qualified forms share the
//! 「カードの効果によってウェイト状態になったとき」 spine and are parsed by richer handlers that
//! keep their group / target / location / cost scoping — notably PL!-pb1-015-P＋ and
//! PL!-pb2-015-P＋ (西木野真姫). The rule refuses any text carrying 「相手のステージ」, 「『」,
//! 「コスト」 or a bound, so it cannot flatten those.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-021-N"; // 南ことり, Printemps, cost 4
const FILLER: &str = "PL!-sd1-010-SD";

/// Fire her 自動 on an already-prepared board and return (orientation, blade).
fn fire_eri_auto(game: &mut TestGame, eri: i16) -> (Option<String>, i16) {
    let ability_id = {
        let card = game.db.get_card(eri).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"))
            .unwrap_or_else(|| panic!("{ERI} lacks a 自動 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(eri).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(eri);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Auto,
        pid.clone(),
        Some(card_no),
        Some(eri),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();

    (
        game.state.mods.get_orientation_modifier(eri).map(|s| s.to_string()),
        game.state.mods.get_blade_modifier(eri),
    )
}

/// Stage ERI, optionally in wait, fire the 自動, and return (orientation, blade).
///
/// `by_effect` distinguishes the two ways of being in wait, and the difference is the whole
/// point of branch 2:
///   * `false` — the orientation is written directly. No `turn_state_changes` entry exists,
///     so 「自分のカードの効果によってウェイト状態になったとき」 is correctly FALSE. She is in
///     wait, but nothing *put* her there.
///   * `true` — the transition is also recorded in `turn_state_changes`, which is exactly
///     what `execute_change_state` writes for a real state change, and what branch 2 reads.
fn after_auto_with(by_effect: bool) -> (Option<String>, i16) {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.state.player1.stage.stage[0] = eri;
    if by_effect {
        let source = game.id(FILLER);
        game.state.mods.add_orientation_modifier(eri, "wait");
        // The tuple shape `execute_change_state` pushes: (source, target, from_state, to_state).
        game.state
            .turn_state_changes
            .push((source, eri, "active".to_string(), "wait".to_string()));
    }
    fire_eri_auto(&mut game, eri)
}

/// The negative, now actually holding: no trigger, no blade.
///
/// This was the test the file pinned a DEFECTIVE value under the name
/// `an_active_member_with_no_trigger_grants_nothing_is_NOT_yet_true` — a name that said out
/// loud what the assertion was doing, which is to say the file was documenting a live bug in
/// prose rather than failing on it. The bug is fixed; the name goes with it.
///
/// She is on stage, ACTIVE, has paid no cost and has never been waited, and the 自動 is
/// fired. Both branches of the `or_condition` are false, so nothing happens at all — no
/// orientation is written, because the ability never runs.
///
/// Orientation `None` here against `Some("active")` in the fired case below is the clearest
/// statement of the fix: the over-firing version also produced an orientation, because the
/// tautological branch made the ability run on every turn.
#[test]
fn an_active_member_with_no_trigger_grants_nothing() {
    let (orientation, blade) = after_auto_with(false);
    assert_eq!(
        (orientation.as_deref(), blade),
        (None, 0),
        "she is active, has paid no cost and was never waited, so \
         「…コストを支払うか、…ウェイト状態になったとき」 is false on BOTH branches and the \
         自動 must not fire. The blade used to be 1: branch 1 parsed as an operator-less cost \
         comparison, which compare_counts read as `cost >= 0` — true forever — so the \
         or_condition was satisfied on every turn."
    );
}

/// Branch 2 firing for real: she was put into wait BY A CARD EFFECT this turn.
///
/// The transition is recorded in `turn_state_changes` the way `execute_change_state` records
/// it. That is what makes this the positive counterpart of the test above rather than a
/// restatement: branch 2 reads that log, and it used to be satisfied by the STATIC 「is in
/// wait」 state instead — which is why merely writing the orientation was enough, and also
/// why it fired for a wait caused by anything at all.
#[test]
fn becoming_wait_by_a_card_effect_activates_her_and_grants_a_blade() {
    let (orientation, blade) = after_auto_with(true);
    assert_eq!(
        blade, 1,
        "ライブ終了時まで、ブレードを得る — exactly one blade from the wait branch. Before the \
         fix this was also 1, but for the wrong reason: branch 1 was a tautology, so it fired \
         whether or not any transition happened."
    );
    assert_eq!(
        orientation.as_deref(),
        Some("active"),
        "「このメンバーをアクティブにし」 — the ability ran, and taking her OUT of wait is a \
         real transition, so the orientation is written. I had expected None on the \
         grounds that the step would be a no-op, but she is in WAIT when the trigger fires, \
         so active→? is a genuine change. Orientation Some(\"active\") plus blade 1 is the \
         full effect; the no-trigger case below is orientation None plus blade 0."
    );
}

/// Branch 1 firing for real: SHE paid the cost of one of her own abilities this turn.
///
/// The two branches are independent, so the wait branch is exercised above and the cost
/// branch here — with her ACTIVE and never waited, which is precisely the board on which
/// branch 2 is false. That makes this the only test that can reach branch 1 at all.
///
/// The payment is recorded the way `GameState::record_cost_paid()` records it, at the point a
/// cost is actually settled. `cards_paid_cost_this_turn` is a SET, so a second payment by the
/// same member does not make her a different card.
#[test]
fn paying_one_of_my_own_ability_costs_triggers_it() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.state.player1.stage.stage[0] = eri;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_orientation_modifier(eri),
        None,
        "premise: she is active, so branch 2's transition cannot be what fires this"
    );
    assert!(
        game.state.cards_paid_cost_this_turn.is_empty(),
        "premise: no cost paid yet"
    );

    // The writer behind the tracker, used directly because standing up a whole ability with a
    // payable cost is not what this test is about — the payment itself is recorded by
    // `record_cost_paid` at every cost-payment site in the engine.
    game.state.cards_paid_cost_this_turn.push(eri);

    let (_, blade) = fire_eri_auto(&mut game, eri);
    assert_eq!(
        blade, 1,
        "「このメンバーが、自分のカードの能力のコストを支払うため」 — having paid, her 自動 \
         fires. Before the fix this fired with NOTHING having happened, because the branch was \
         an operator-less cost comparison reading as `cost >= 0`"
    );
}

/// Being in wait is NOT the trigger — being put into wait by one of your own card's
/// effects is.
///
/// Same board as the test above: she is in wait. The only difference is that no
/// `turn_state_changes` entry was written, because nothing put her there. She must NOT fire.
///
/// This is the distinction the old parse could not make. Branch 2 used to be a static
/// `state_condition {state: wait}`, so 「自分のカードの効果によってウェイト状態になったとき」
/// was satisfied by the member merely BEING waited — by physical wait, by the opponent, by
/// anything. 「…なったとき」 names a transition and a cause, and this test is what holds the
/// engine to both.
#[test]
fn being_in_wait_without_a_transition_does_not_trigger_it() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.state.player1.stage.stage[0] = eri;
    game.state.mods.add_orientation_modifier(eri, "wait");
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_orientation_modifier(eri),
        Some("wait"),
        "premise: she really is in wait"
    );
    assert!(
        !game.state
            .turn_state_changes
            .iter()
            .any(|(_, cid, _, _)| *cid == eri),
        "premise: no state-change transition was recorded for her"
    );

    let (_, blade) = fire_eri_auto(&mut game, eri);
    assert_eq!(
        blade, 0,
        "「自分のカードの効果によってウェイト状態になったとき」 is about a transition with a \
         cause. Merely BEING in wait is a state, and it is not the trigger — before the fix \
         this fired for any wait at all"
    );
}

/// Nothing on the board at all → no blade. Pins that the sequential does not fire
/// unprompted from a completely empty setup.
#[test]
fn an_empty_setup_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let eri = game.id(ERI);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(eri),
        0,
        "precondition: a plain 常時 recalculation grants nothing — the 自動 is what \
         grants it"
    );
}

/// The 自動 must not fire for an ability on a DIFFERENT card. This is the over-fire
/// guard that does hold today, and it is what makes the remaining gap a *condition*
/// problem rather than a blanket "always fires" problem.
#[test]
fn it_does_not_fire_for_another_cards_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let eri = game.id(ERI);
    let other = game.new_id(ERI);
    game.state.player1.stage.stage[0] = other;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(eri),
        0,
        "the ability belongs to the member that gained it"
    );
}

/// The wait branch must parse as a TURN-SCOPED TRANSITION, not a static state and not a
/// count — pinned so the fix cannot silently regress into either vacuous form.
#[test]
fn the_wait_branch_parses_as_a_turn_scoped_transition() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let eri = game.id(ERI);
    let eff = {
        let owned = game
            .db
            .get_card(eri)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("自動 ability has an effect")
    };
    let cond = eff
        .condition
        .as_deref()
        .expect("this ability is conditional");

    // `ConditionType` is private, so the two-branch `or` structure is asserted through
    // `get_conditions()` below rather than by type name.
    let branches = cond.get_conditions().expect("or_condition carries its branches");
    assert_eq!(branches.len(), 2, "two branches");

    let wait_branch = branches
        .iter()
        .find(|b| b.get_text().is_some_and(|t| t.contains("ウェイト状態")))
        .expect("the wait branch");
    assert_eq!(
        wait_branch.get_state(),
        Some(rabuka_engine::card::CardState::Wait),
        "…ウェイト状態になった"
    );
    assert_eq!(
        wait_branch.get_temporal(),
        Some("this_turn"),
        "「…なったとき」 is an EVENT. Without this the clause is the static 「is in wait」 \
         state, which stays true for as long as she sits waited and ignores the cause."
    );
    assert_eq!(
        wait_branch.get_from_state(),
        Some("active"),
        "the transition it names is active → wait"
    );
    assert_eq!(
        wait_branch.get_to_state(),
        Some("wait"),
        "the transition it names is active → wait"
    );
    assert_eq!(
        wait_branch.get_self_target(),
        Some(true),
        "own card's effect — the clause is about THIS member, so it must be self-scoped or \
         any waited member on stage would satisfy it"
    );
    assert!(
        wait_branch.get_count().is_none(),
        "the vacuous `count >= 1` form must not come back: with no operator it compares as >= \
         and is always true"
    );
}

/// The cost branch must carry `paid_cost_this_turn` and no operator or bound — pinned so the
/// tautology cannot come back.
#[test]
fn the_cost_paid_branch_parses_as_a_paid_cost_history() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let eri = game.id(ERI);
    let eff = {
        let owned = game
            .db
            .get_card(eri)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("自動 ability has an effect")
    };
    let branches = eff
        .condition
        .as_deref()
        .and_then(|c| c.get_conditions())
        .expect("or_condition carries its branches");
    let cost_branch = branches
        .iter()
        .find(|b| b.get_text().is_some_and(|t| t.contains("コストを支払う")))
        .expect("the cost branch");

    assert_eq!(
        cost_branch.get_paid_cost_this_turn(),
        Some(true),
        "「コストを支払う」 is a per-turn HISTORY of who paid a cost, not a comparison on \
         cost. The flag is what the evaluator branches on, and it is backed by \
         GameState::cards_paid_cost_this_turn."
    );
    assert_eq!(
        cost_branch.get_self_target(),
        Some(true),
        "「このメンバーが」 — the clause is about the PAYER, so it is self-scoped"
    );
    assert!(
        cost_branch.get_operator().is_none() && cost_branch.get_values().is_none(),
        "no operator and no bound may come back: `compare_counts` reads a missing operator as \
         `>=`, so with nothing to compare against this is `cost >= 0` and the 自動 fires every \
         turn"
    );
}
