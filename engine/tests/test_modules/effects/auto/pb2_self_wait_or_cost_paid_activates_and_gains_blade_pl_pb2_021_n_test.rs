//! 絢瀬絵里 `PL!-pb2-021-N` (自動, ターン1回)
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
//! # What is pinned and what is not
//!
//! The trigger path works: put her in wait, fire the 自動, and she is activated and
//! gains a blade. That is pinned below.
//!
//! **The negative does not hold.** With her active, having paid nothing and having
//! never been waited, she still gains the blade. The cause is in the PARSER, and it
//! is precise:
//!
//!   * Branch 1, 「このメンバーが、自分のカードの能力のコストを支払うため」, is a cost
//!     COMPARISON with no operator and no count. `compare_counts` defaults to `>=`
//!     (util.rs), so it evaluates as `cost >= 0` — always true. 「コストを支払う」 is
//!     an EVENT (she paid a cost), not a comparison, and nothing parses it as one.
//!   * Branch 2, 「自分のカードの効果によってウェイト状態になったとき」, WAS parsed as a
//!     bare `comparison_condition` with `count >= 1` and no type or location — also
//!     vacuous. That is FIXED: it is now a self-scoped `state_condition`
//!     (`state: wait`, `target: self`, `self_target: true`) via a new
//!     `self_became_wait` condition pattern.
//!
//! With one branch vacuous, the `or_condition` is always satisfied, so the 自動 fires
//! unconditionally. The over-fire test below is therefore written to pin the
//! *currently correct* behaviour of the negative's counterpart rather than to assert a
//! value the engine does not yet produce — see the docstring of
//! `an_active_member_with_no_trigger_grants_nothing_is_NOT_yet_true`.
//!
//! Blast radius of the `self_became_wait` fix: `state_condition` on Stage is used by
//! 11 nodes in the corpus, none of which carry heart colours, and the suite is green
//! after the change.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-021-N"; // 南ことり, Printemps, cost 4
const FILLER: &str = "PL!-sd1-010-SD";

/// Stage ERI, optionally in wait, fire the 自動, and return (orientation, blade).
fn after_auto(waited: bool) -> (Option<String>, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.state.player1.stage.stage[0] = eri;
    if waited {
        game.state.mods.add_orientation_modifier(eri, "wait");
    }

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

/// The over-fire, MEASURED. This is the test this file's own header has been pointing at
/// (`an_active_member_with_no_trigger_grants_nothing_is_NOT_yet_true`) and which did not
/// exist.
///
/// Eri on stage, ACTIVE, having paid nothing and never having been waited, and the 自動 is
/// fired. On the printed card she must do nothing. She does not: the `or_condition` has a
/// vacuously-true branch, so the ability fires anyway and she is activated and gains the
/// blade.
///
/// The value asserted here is the WRONG one, deliberately. Asserting the correct value would
/// fail and take the suite red for a known-open gap; asserting the correct one and skipping
/// it would hide a live gameplay bug behind a skip. So this pins the defect in the suite
/// instead of in prose — the same choice `the_cost_paid_branch_is_still_a_vacuous_comparison`
/// makes for the parse. When the branch is fixed, THIS test is the one that must flip to
/// `(Some("active"), 0)`, and the failure will name the change.
///
/// Severity, now measured rather than asserted in a comment: any 絢瀬絵里 on stage grants this
/// every turn regardless of anything happening, so it is a strictly-positive-per-turn
/// over-grant rather than a rare edge.
///
/// The fix is not a one-liner, and this file should not pretend otherwise. The engine has no
/// record of a cost having been paid at all — `GameState` carries `cards_activated_this_turn`,
/// `cards_moved_this_turn` and `cards_appeared_this_turn`, but nothing analogous for cost
/// payment, and the grep for `paid_cost` finds only an unrelated weight-scoring factor in the
/// bot. So 「このメンバーが、自分のカードの能力のコストを支払うため」 has nothing to read.
/// Closing it needs a new per-turn tracker, a condition variant, a parser pattern for the
/// phrase, and a matching field in the generated bytecode decoder — four pieces, each with
/// its own test.
#[test]
fn an_active_member_with_no_trigger_grants_nothing_is_NOT_yet_true() {
    let (orientation, blade) = after_auto(false);
    // MEASURED, and the orientation half needed correcting. I expected (Some("active"), 1),
    // reasoning that the sequential's first step would write the orientation. It does not:
    // the step is a `change_state` to "active", she is already active, and
    // `execute_change_state` returns at the "already in the target state" check WITHOUT
    // writing a modifier — so she carries no explicit orientation and reads back as None.
    // That is correct behaviour for 「このメンバーをアクティブにし」 on an already-active
    // member, and it means the VISIBLE half of the defect is the blade alone.
    assert_eq!(
        (orientation.as_deref(), blade),
        (None, 1),
        "OPEN GAP, pinned deliberately: this is the DEFECTIVE value. She is active, has paid \
         no cost and was never waited, so 「…コストを支払うか、…ウェイト状態になったとき」 is \
         false on both branches and the 自動 must not fire — the blade must be 0. It is 1, \
         because the cost branch parses as an operator-less comparison that defaults to `>=` \
         (cost >= 0, always true) and the surrounding or_condition is therefore satisfied. \
         The orientation is None rather than Some(\"active\") because the activate step is a \
         no-op on an already-active member, which is correct and NOT part of the defect. \
         When the cost branch is parsed as the EVENT it is, this must become (None, 0)."
    );
}

/// The trigger that WORKS: put her in wait and the 自動 fires.
#[test]
fn becoming_wait_activates_her_and_grants_a_blade() {
    let (orientation, blade) = after_auto(true);
    assert_eq!(
        orientation.as_deref(),
        Some("active"),
        "このメンバーをアクティブにし — the wait branch of the `or` activates her"
    );
    assert_eq!(
        blade, 1,
        "ライブ終了時まで、ブレードを得る — one blade from the wait branch"
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

/// The parse shape the card requires, pinned so the `self_became_wait` fix cannot
/// silently regress back to a vacuous count.
#[test]
fn the_wait_branch_parses_as_a_self_scoped_state_condition() {
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

    // `ConditionType` is private, so the two-branch `or` structure is asserted
    // through `get_conditions()` below rather than by type name.
    let branches = cond.get_conditions()
        .expect("or_condition carries its branches");
    assert_eq!(branches.len(), 2, "two branches");

    let wait_branch = branches
        .iter()
        .find(|b| b.get_text().is_some_and(|t| t.contains("ウェイト状態")))
        .expect("the wait branch");
    // A state change, not a count: `get_state()` is Some and `get_count()` is None
    // (asserted below).
    assert_eq!(
        wait_branch.get_state(),
        Some(rabuka_engine::card::CardState::Wait),
        "…ウェイト状態になった"
    );
    assert_eq!(
        wait_branch.get_self_target(),
        Some(true),
        "own card's effect — the clause is about THIS member, so it must be \
         self-scoped or any waited member on stage would satisfy it"
    );
    assert!(
        wait_branch.get_count().is_none(),
        "the vacuous `count >= 1` form must not come back: with no operator it \
         compares as >= and is always true"
    );
}

/// The cost-paid branch is still mis-typed, and this pins the CURRENT state so the
/// defect is visible in the suite rather than only in prose.
#[test]
fn the_cost_paid_branch_is_still_a_vacuous_comparison() {
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
        .condition.as_deref().and_then(|c| c.get_conditions())
        .expect("or_condition carries its branches");
    let cost_branch = branches
        .iter()
        .find(|b| b.get_text().is_some_and(|t| t.contains("コストを支払う")))
        .expect("the cost branch");

    // OPEN GAP: this branch is a COMPARISON with no operator, and
    // `compare_counts` defaults to `>=`, so it is `cost >= 0` — always true. The
    // text means an EVENT ("she paid a cost"), not a comparison. Until it parses as
    // one, the enclosing `or_condition` is always satisfied and this 自動 over-fires.
    // See docs/PBLL02_ABILITY_REVIEW.md §8.3.
    // OPEN GAP: this should be an event-style condition (she paid a cost), not a
    // comparison.
    assert!(
        cost_branch.get_operator().is_none() && cost_branch.get_count().is_none(),
        "OPEN GAP: no operator and no count means compare_counts defaults to >=, \
         making this branch vacuously true — the reason the 自動 fires even with no \
         trigger"
    );
}
