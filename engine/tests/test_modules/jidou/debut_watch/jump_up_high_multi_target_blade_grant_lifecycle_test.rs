//! `Jump up HIGH!!` `PL!S-sd1-022-SD` — the multi-target shape of a
//! duration-scoped ライブ開始時 grant.
//!
//! 「ライブ終了時まで、自分のステージにいる『Aqours』のメンバーはブレードを得る」.
//!
//! Every other duration grant in this family lands on ONE card and is read off a
//! single aggregate, so a test that grants once and checks the number covers them.
//! This one fans out to EVERY eligible member, which adds transitions no single-target
//! test can reach:
//!
//!   * each member gets EXACTLY one blade — a fan-out that double-counts inflates
//!     every member at once, and a `>= 1` check cannot see it;
//!   * the grant is scoped ライブ終了時, so it must clear at the real rollover
//!     rather than surviving into the next live;
//!   * membership is part of the printed condition, so what a member that ARRIVES
//!     after the trigger, or one that LEAVES, gets must be pinned from both sides.
//!
//! The existing test (`debut_and_live_start_blade_grant_test.rs`) fires once and
//! asserts `>= 1` per member, which is a presence check: a grant that reached two
//! blades, or a fan-out that double-counted, would pass it. That is the gap this
//! file closes — every expectation here is an exact number.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_state::{Phase, TurnPhase};

const JUMP_UP: &str = "PL!S-sd1-022-SD";
/// Two 『Aqours』-series members (unit AZALEA) and one that is not, so the group
/// filter is exercised in both directions by the same board.
const AQOURS_A: &str = "PL!S-sd1-001-SD";
const AQOURS_B: &str = "PL!S-sd1-002-SD";
const NOT_AQOURS: &str = "PL!HS-bp5-004-R";
/// A cheap live used to walk a real live; it is only a vehicle for the rollover.
const LIVE: &str = "PL!-sd1-019-SD";

fn blades(game: &TestGame, member: i16) -> i32 {
    i32::from(game.state.mods.get_blade_modifier(member))
}

fn drain(game: &mut TestGame) {
    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectTarget") {
            game.select_option(0);
        } else {
            game.select_indices(&[]);
        }
    }
}

/// Stage two 『Aqours』 members and one that is not, with the live card in the
/// live zone, and report the three ids.
fn stage_two_aqours_and_one_outsider(game: &mut TestGame) -> (i16, i16, i16) {
    let live = game.id(JUMP_UP);
    let a = game.id(AQOURS_A);
    let b = game.id(AQOURS_B);
    let outsider = game.id(NOT_AQOURS);
    game.assert_card_identity(live, JUMP_UP);
    for member in [a, b] {
        game.assert_card_in_group(member, "Aqours", "the beneficiary is an 『Aqours』 member");
    }
    // Asserted NEGATIVE, and against the group the ability actually filters on,
    // rather than by the outsider's own unit name: the claim under test is that
    // it sits outside 『Aqours』, and naming its unit would pin an unrelated fact
    // while the unit spelling stays fragile.
    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(
            &game.db,
            outsider,
            Some("Aqours")
        ),
        "the non-beneficiary must be OUTSIDE 『Aqours』, or the group filter is \
         never exercised in the negative direction and a group-blind grant passes"
    );
    game.state.player1.stage.stage = [a, b, outsider];
    game.state.player1.live_card_zone.cards.push(live);
    (a, b, outsider)
}

fn roll_over_live_end(game: &mut TestGame) {
    let mut guard = 0usize;
    while game.state.current_turn_phase == TurnPhase::Live && guard < 16 {
        guard += 1;
        game.pass();
        drain(game);
    }
    assert_ne!(
        game.state.current_turn_phase,
        TurnPhase::Live,
        "the walk never crossed the victory-determination rollover (guard {guard})"
    );
}

/// EXACT-FANOUT: each eligible member gets exactly one blade, and the ineligible
/// one gets none.
///
/// A fan-out that visited a member twice, or re-ran because the trigger was
/// dispatched twice, would read 2 or 3 here. The existing test's `>= 1` cannot
/// distinguish any of those from correct behaviour.
#[test]
fn jump_up_high_gives_each_aqours_member_exactly_one_blade() {
    let mut game = TestGame::new(load_real_database());
    let (a, b, outsider) = stage_two_aqours_and_one_outsider(&mut game);

    let live_id = game.id(JUMP_UP);
    fire_trigger(&mut game, live_id, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);

    assert_eq!(
        blades(&game, a),
        1,
        "an 『Aqours』 member must gain exactly ONE blade — 2 would mean the \
         fan-out double-counted, and `>= 1` cannot see that"
    );
    assert_eq!(
        blades(&game, b),
        1,
        "the second 『Aqours』 member must gain exactly ONE blade"
    );
    assert_eq!(
        blades(&game, outsider),
        0,
        "『Aqours』のメンバー is the printed condition, so a non-『Aqours』 member \
         on the same board gains nothing"
    );

    // Re-derived as many times as a live window does: the grant must be stable,
    // not cumulative per scan.
    for round in 1..=5 {
        game.state.recalculate_constants();
        assert_eq!(
            blades(&game, a),
            1,
            "round {round}: the grant must not accumulate as scans repeat"
        );
        assert_eq!(
            blades(&game, b),
            1,
            "round {round}: the grant must not accumulate as scans repeat"
        );
        assert_eq!(
            blades(&game, outsider),
            0,
            "round {round}: the ineligible member must stay at 0"
        );
    }
}

/// DURATION: ライブ終了時まで is the whole grant, and it must clear at the REAL
/// rollover — every member at once, since the grant is per-board rather than
/// per-card.
#[test]
fn jump_up_high_blade_grant_clears_at_the_real_live_end_rollover() {
    let mut game = TestGame::new(load_real_database());
    let (a, b, _) = stage_two_aqours_and_one_outsider(&mut game);

    // Seat a live through HAND so the walk has a real live to end.
    let vehicle = game.id(LIVE);
    game.state.player1.hand.cards.push(vehicle);
    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(vehicle);

    let live_id = game.id(JUMP_UP);
    fire_trigger(&mut game, live_id, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(blades(&game, a), 1, "precondition: the grant is live");
    assert_eq!(blades(&game, b), 1, "precondition: both members hold it");

    roll_over_live_end(&mut game);
    game.state.recalculate_constants();

    assert_eq!(
        blades(&game, a),
        0,
        "ライブ終了時まで is the whole duration: no member may keep the blade"
    );
    assert_eq!(
        blades(&game, b),
        0,
        "the grant is per-board, so the second member must clear with the first"
    );
}

/// MEMBERSHIP is part of the printed condition, and it is read at the trigger,
/// not per scan. A member that ARRIVES after ライブ開始時 therefore does NOT gain
/// the blade, and one that LEAVES must not keep it.
///
/// Pinned from both sides because they are different mechanisms: the first is a
/// latch, the second is rule 4.1.4 zone exit. An implementation that re-evaluated
/// membership per scan would satisfy the second and break the first, and one that
/// never cleared on zone exit would satisfy the first and break the second.
#[test]
fn jump_up_high_grant_does_not_reach_a_late_arrival_and_does_not_outlive_a_departure() {
    let mut game = TestGame::new(load_real_database());
    let (a, b, _) = stage_two_aqours_and_one_outsider(&mut game);

    let live_id = game.id(JUMP_UP);
    fire_trigger(&mut game, live_id, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(blades(&game, a), 1, "precondition: member A holds the blade");
    assert_eq!(blades(&game, b), 1, "precondition: member B holds the blade");

    // A departure goes through the engine's own zone-exit choke point.
    game.state.player1.stage.stage[0] = -1;
    game.state.on_cards_left_zones(&[a]);
    game.state.recalculate_constants();
    assert_eq!(
        blades(&game, a),
        0,
        "rule 4.1.4: a member that left the stage must not keep a grant that was \
         resolved while it was on the board"
    );
    assert_eq!(
        blades(&game, b),
        1,
        "the other member's grant is unaffected by one departure"
    );

    // An arrival after the trigger does not receive it: the condition is read
    // when ライブ開始時 fires, not on every scan.
    let latecomer = game.id(AQOURS_A);
    game.state.player1.stage.stage[0] = latecomer;
    game.state.recalculate_constants();
    assert_eq!(
        blades(&game, latecomer),
        0,
        "『自分のステージにいる』 is assessed when the trigger fires, so a member \
         that arrives later does not gain the blade. Re-checking membership every \
         scan would hand out blades the live card never granted."
    );
}

/// Only the LIVE card's own resolution grants this. A 『Aqours』 member is not a
/// live card, and firing the same printed text from a member's id must not
/// produce a board-wide blade grant — the target of the trigger is the live card.
#[test]
fn the_grant_is_scoped_to_the_live_card_not_to_any_card_printing_the_text() {
    let mut game = TestGame::new(load_real_database());
    let (a, _, _) = stage_two_aqours_and_one_outsider(&mut game);

    // Nothing has fired yet.
    assert_eq!(
        blades(&game, a),
        0,
        "precondition: the grant exists only after the ライブ開始時 fires"
    );

    let live_id = game.id(JUMP_UP);
    fire_trigger(&mut game, live_id, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(blades(&game, a), 1, "the live card's trigger does grant it");

    // A member staged AFTER the grant must not be swept up, and the existing
    // holders must not be topped up by a second scan.
    let extra = game.id(AQOURS_B);
    game.state.player1.stage.stage[1] = extra;
    game.state.recalculate_constants();
    assert_eq!(
        blades(&game, extra),
        0,
        "a member added after the grant does not gain it (see the latch test)"
    );
    assert_eq!(
        blades(&game, a),
        1,
        "and the original holder is not topped up by the later scan"
    );
}
