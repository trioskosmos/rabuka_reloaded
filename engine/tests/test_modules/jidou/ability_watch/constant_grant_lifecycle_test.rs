//! LIFECYCLE of `gain_ability` grants — the second half of the
//! stateful-registration axis that TEST_COVERAGE's "Lifecycle-constrained
//! abilities" table tracks.
//!
//! The `gain_ability` family grants a 常時, for a printed duration. Both
//! transitions the report asks for are implemented in the engine and neither was
//! proven by any test:
//!
//!   * `modifiers.rs:1656` dedups the text registration, so a re-registration
//!     cannot accumulate a second grant.
//!   * `modifiers.rs:990-1011` re-derives the whole expected set each constant
//!     scan and EXPIRES any 常時 grant no longer expected, removing it from
//!     `gained_abilities`.
//!   * the live-end rollover in `phases.rs` clears a `ライブ終了時まで` grant, via a
//!     real phase walk rather than a hand-called expiry check.
//!
//! A single-resolution test cannot see any of the three. It fires the ability
//! once, reads `+1`, and stops — which is exactly what would have happened had
//! the grant stacked to `+2` on the next scan or outlived its live.
//!
//! ## Two families, two different revocations
//!
//! The interesting part of this family is that revocation means DIFFERENT things
//! for its two shapes, and a first draft of this file got it wrong in a way worth
//! recording:
//!
//!   * A grant made by a **常時** (乙宗梢) has a condition the engine RE-EVALUATES
//!     on every scan, so it must be withdrawn the instant a stage slot empties.
//!   * A grant made by a **ライブ開始時** (矢澤にこ, 宮下愛) LATCHES. Its condition
//!     is assessed once, when the trigger fires, and the printed
//!     `ライブ終了時まで` governs from there. Dropping 矢澤にこ's waitroom below 25
//!     mid-live correctly does NOT revoke the grant — revoking there would be
//!     the bug. `niko_grant_latches_at_live_start` pins that, so the distinction is
//!     enforced from both sides rather than left to whichever reading a
//!     re-implementation happens to take.
//!
//! Every assertion is on a NUMBER (bonus and granted-text length), never "> 0".
//! A grant that doubled passes a presence check and fails these.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_state::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;

const NIKO: &str = "PL!-sd1-009-SD";
const AI: &str = "PL!N-bp3-005-R＋";
const KOTO: &str = "PL!HS-bp1-003-R＋";
/// Two more 『蓮ノ空』 names, so 乙宗梢's 「エリアすべて…名前が異なる」 condition can
/// actually be satisfied — two copies of one name do not satisfy 名前が異なる.
const RENJO_UTA: &str = "PL!HS-bp1-016-PR";
const RENJO_KOKO: &str = "PL!HS-bp2-010-PR";
const FILLER: &str = "PL!-sd1-010-SD";
const MU: &str = "PL!-sd1-004-SD";
const LIVE: &str = "PL!-sd1-020-SD";

fn bonus(game: &TestGame) -> i32 {
    i32::from(game.state.mods.p1_constant_total_score_bonus)
}

/// How many granted ability TEXTS the host holds. This is the assertion that
/// names duplication directly — a grant that stacks registers twice — and it also
/// separates "expired" from "expired but still registered", which the score
/// aggregate alone cannot tell apart.
fn granted_text_count(game: &TestGame, card: i16) -> usize {
    game.state
        .gained_abilities
        .get(&card)
        .map(|texts| texts.len())
        .unwrap_or(0)
}

fn fill_decks(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn drain(game: &mut TestGame) {
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
}

/// Seat a live card and advance to the point where ライブ開始時 dispatch is real.
///
/// Granting outside a live works for the constant scan, but the live-END
/// rollover only happens on a genuine phase walk — a grant armed while still in
/// Main never sees a rollover, and a test that then asserts expiry is asserting
/// nothing. The live card goes through HAND because that is the route
/// `set_live_card` requires; the Draw phase would otherwise empty a hand it never
/// filled.
fn arm_inside_a_live(game: &mut TestGame) -> i16 {
    let live = game.id(LIVE);
    game.state.player1.hand.cards.push(live);
    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);
    live
}

/// Ride the genuine phase walk out of the live, through victory determination,
/// into the next turn's Active.
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

// ====================================================================
// 矢澤にこ — a ライブ開始時 grant with a latched condition
// ====================================================================

/// 「自分の控え室に『μ's』のカードが25枚以上ある場合、ライブ終了時まで、常時 +1」.
///
/// IDEMPOTENCE: the constant scan runs on every state change. Once registered the
/// grant must stay a SINGLE grant — a scan that re-added or appended would take it
/// to +2, and the registration length says "appended" outright rather than
/// inferring it from the total.
#[test]
fn niko_grant_stays_single_across_repeated_constant_scans() {
    let mut game = TestGame::new(load_real_database());
    let niko = game.id(NIKO);
    let mu = game.id(MU);
    game.assert_card_identity(niko, NIKO);
    game.assert_card_in_group(niko, "BiBi", "the host is a 『μ's』 BiBi member");

    fill_decks(&mut game);
    game.state.player1.stage.stage = [niko, -1, -1];
    game.give_energy(15);
    for _ in 0..25 {
        game.state.player1.waitroom.cards.push(mu);
    }

    arm_inside_a_live(&mut game);
    fire_trigger(&mut game, niko, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(
        bonus(&game),
        1,
        "precondition: with 25 μ's cards in the waitroom the ライブ開始時 must \
         grant the 常時 +1"
    );
    assert_eq!(
        granted_text_count(&game, niko),
        1,
        "precondition: exactly one granted text"
    );

    for round in 1..=6 {
        game.state.recalculate_constants();
        assert_eq!(
            bonus(&game),
            1,
            "round {round}: the grant must stay a single +1. A 常時 that \
             re-derives its registration must REPLACE it, not append to it."
        );
        assert_eq!(
            granted_text_count(&game, niko),
            1,
            "round {round}: the granted-text list must hold exactly one entry, \
             which is the direct statement of 'not appended' (got {})",
            granted_text_count(&game, niko)
        );
    }
}

/// The LATCH, pinned from the side a re-implementation is most likely to get
/// wrong. The condition is assessed once, when ライブ開始時 fires; the printed
/// duration governs afterwards. So dropping the waitroom to 24 mid-live must NOT
/// take the grant away.
///
/// An engine that re-evaluated a latched condition on every scan would pass the
/// idempotence test above and fail this one, while scoring 矢澤にこ down for a
/// condition the card never re-checks.
#[test]
fn niko_grant_latches_at_live_start_and_survives_its_condition_later_failing() {
    let mut game = TestGame::new(load_real_database());
    let niko = game.id(NIKO);
    let mu = game.id(MU);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [niko, -1, -1];
    game.give_energy(15);
    for _ in 0..25 {
        game.state.player1.waitroom.cards.push(mu);
    }

    arm_inside_a_live(&mut game);
    fire_trigger(&mut game, niko, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(bonus(&game), 1, "precondition: the grant is live");

    game.state.player1.waitroom.cards.pop();
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        24,
        "precondition: one card short of the 25枚以上 threshold"
    );
    game.state.recalculate_constants();

    assert_eq!(
        bonus(&game),
        1,
        "『ライブ終了時まで』 runs from the moment the trigger fired: the \
         25枚以上 condition is assessed ONCE, at ライブ開始時, and is not \
         re-checked. Revoking here would take a legally held grant away."
    );
    assert_eq!(
        granted_text_count(&game, niko),
        1,
        "the registration must be untouched by a condition that no longer applies"
    );
}

/// DURATION: the grant must be gone once the live that granted it ends, even
/// though the condition that produced it still holds.
///
/// Driven through the genuine phase walk. A grant armed in Main never sees a
/// rollover, so a test asserting expiry after arming outside a live would be
/// asserting nothing at all — that is the first draft of this test.
#[test]
fn niko_grant_expires_through_the_real_live_end_rollover() {
    let mut game = TestGame::new(load_real_database());
    let niko = game.id(NIKO);
    let mu = game.id(MU);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [niko, -1, -1];
    game.give_energy(15);
    for _ in 0..25 {
        game.state.player1.waitroom.cards.push(mu);
    }

    arm_inside_a_live(&mut game);
    fire_trigger(&mut game, niko, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(bonus(&game), 1, "precondition: the grant is live");

    roll_over_live_end(&mut game);
    game.state.recalculate_constants();

    assert!(
        game.state.player1.waitroom.cards.len() >= 25,
        "precondition: the condition STILL holds at the rollover, so only the \
         printed ライブ終了時まで duration can be what ended the grant (waitroom \
         holds {}; the next turn's Draw may have refilled it, which only makes \
         the claim stronger)",
        game.state.player1.waitroom.cards.len()
    );
    assert_eq!(
        bonus(&game),
        0,
        "ライブ終了時まで is the whole duration: the grant must not survive the \
         live that granted it"
    );
    assert_eq!(
        granted_text_count(&game, niko),
        0,
        "the registration must be cleared by the rollover too — an expired grant \
         still registered is one anything consulting the registration can re-read"
    );
}

// ====================================================================
// 宮下愛 — a THIS-TURN condition, latched at ライブ開始時
// ====================================================================

/// 「このターン、自分のステージにメンバーが2回以上登場している場合、ライブ終了時まで、
/// 常時 +1」.
///
/// 『2回以上』 is a THRESHOLD, not a counter, so the bonus is a flat +1 however far
/// past 2 the turn goes, and it must not accumulate as the constant scan repeats.
#[test]
fn ai_grant_is_a_flat_bonus_past_its_two_debut_threshold() {
    let mut game = TestGame::new(load_real_database());
    let ai = game.id(AI);
    game.assert_card_identity(ai, AI);
    fill_decks(&mut game);
    game.give_energy(30);

    // 宮下愛 must be ON STAGE for her own ライブ開始時 to resolve, and the
    // condition counts members that have debuted THIS TURN — her own debut is
    // one of them. This is the arrangement the established 宮下愛 idiom uses in
    // `sibling_abilities_on_one_card_interact_test.rs`; staging two fillers and
    // leaving her in hand grants nothing at all.
    game.state.player1.stage.stage = [-1, -1, -1];
    let first = game.new_id(FILLER);
    let third = game.new_id(FILLER);
    game.add_to_hand(first);
    game.add_to_hand(ai);
    game.add_to_hand(third);
    game.play_to_stage(first, MemberArea::LeftSide);
    scan_autos_both(&mut game);
    game.play_to_stage(ai, MemberArea::Center);
    scan_autos_both(&mut game);
    // A third debut, to sit past the 2回以上 threshold rather than exactly on it.
    game.play_to_stage(third, MemberArea::RightSide);
    scan_autos_both(&mut game);
    assert_eq!(
        game.state.player1.stage.stage.iter().filter(|c| **c != -1).count(),
        3,
        "precondition: three members are staged this turn"
    );

    // Fired directly, matching the established 宮下愛 idiom. This ability's
    // condition is a THIS-TURN debut count, and driving the live walk first
    // advances through phases that can retire that count before the trigger ever
    // fires — arming it inside a live here tests the phase walk, not the
    // threshold.
    let turn_before = game.state.turn_number;
    fire_trigger(&mut game, ai, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(
        game.state.turn_number, turn_before,
        "precondition: arming must not itself roll the turn over"
    );
    assert_eq!(
        bonus(&game),
        1,
        "『2回以上』 is a threshold, not a counter: three debuts past it must \
         still grant a flat +1, never +2"
    );
    assert_eq!(granted_text_count(&game, ai), 1, "precondition: one text");

    for round in 1..=4 {
        game.state.recalculate_constants();
        assert_eq!(
            bonus(&game),
            1,
            "round {round}: the grant must not accumulate as scans repeat"
        );
        assert_eq!(
            granted_text_count(&game, ai),
            1,
            "round {round}: exactly one granted text"
        );
    }
}

/// The same grant's duration: gone at the live-end rollover, with the two debuts
/// still on stage the whole time — so this is the DURATION ending it, not the
/// condition lapsing.
#[test]
fn ai_grant_expires_through_the_real_live_end_rollover() {
    let mut game = TestGame::new(load_real_database());
    let ai = game.id(AI);
    fill_decks(&mut game);
    game.give_energy(30);

    // Same arrangement as the flat-bonus test: 宮下愛 on stage, and her own debut
    // counted toward the このターン total.
    game.state.player1.stage.stage = [-1, -1, -1];
    let first = game.new_id(FILLER);
    game.add_to_hand(first);
    game.add_to_hand(ai);
    game.play_to_stage(first, MemberArea::LeftSide);
    scan_autos_both(&mut game);
    game.play_to_stage(ai, MemberArea::Center);
    scan_autos_both(&mut game);

    // Armed directly, then walked through a real live so the duration has a real
    // rollover to expire on. See the flat-bonus test for why the trigger is not
    // fired from inside the live for this card.
    fire_trigger(&mut game, ai, AbilityTrigger::LiveStart, "ライブ開始時");
    drain(&mut game);
    assert_eq!(bonus(&game), 1, "precondition: the grant is live");
    arm_inside_a_live(&mut game);

    roll_over_live_end(&mut game);
    game.state.recalculate_constants();

    assert!(
        game.state.player1.stage.stage.iter().filter(|c| **c != -1).count() >= 2,
        "precondition: both debuts are still on stage, so the このターン condition \
         is unchanged and only the duration can have ended the grant"
    );
    assert_eq!(
        bonus(&game),
        0,
        "the grant must not outlive ライブ終了時"
    );
    assert_eq!(
        granted_text_count(&game, ai),
        0,
        "the registration must be cleared as well as the score"
    );
}

// ====================================================================
// 乙宗梢 — a 常時 grant, so the condition IS re-evaluated every scan
// ====================================================================

/// Stage three 『蓮ノ空』 members of THREE DIFFERENT names, satisfying
/// 「自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が
/// 異なる場合、常時 +1」.
fn stage_three_distinct_renjo(game: &mut TestGame, koto: i16) {
    let uta = game.id(RENJO_UTA);
    let koko = game.id(RENJO_KOKO);
    for partner in [uta, koko] {
        game.assert_card_in_group(partner, "スリーズブーケ", "the partner is a 『蓮ノ空』 member");
        assert_ne!(
            partner, koto,
            "precondition: 名前が異なる means the three must be three different cards"
        );
    }
    game.state.player1.stage.stage = [koto, uta, koko];
}

/// REVOCATION for a 常時 grant: the condition is re-derived every scan, so the
/// grant must be withdrawn the moment a single area empties — and return when the
/// area is refilled. Driven through the real zone-exit so this is a revocation
/// test rather than another grant test.
#[test]
fn koto_grant_is_withdrawn_when_one_area_empties_and_returns_when_refilled() {
    let mut game = TestGame::new(load_real_database());
    let koto = game.id(KOTO);
    game.assert_card_identity(koto, KOTO);
    game.assert_card_in_group(koto, "スリーズブーケ", "the host is a 『蓮ノ空』 member");
    stage_three_distinct_renjo(&mut game, koto);

    game.state.recalculate_constants();
    assert_eq!(
        bonus(&game),
        1,
        "precondition: all three areas hold 『蓮ノ空』 members of different names, \
         so the 常時 +1 must be live"
    );
    assert_eq!(
        granted_text_count(&game, koto),
        1,
        "precondition: one granted text"
    );

    let vacated = game.state.player1.stage.stage[2];
    game.state.player1.stage.stage[2] = -1;
    game.state.on_cards_left_zones(&[vacated]);
    game.state.recalculate_constants();

    assert_eq!(
        bonus(&game),
        0,
        "『エリアすべて』 means every area: one empty area must WITHDRAW the grant"
    );
    assert_eq!(
        granted_text_count(&game, koto),
        0,
        "the granted text must be withdrawn with it"
    );

    // Refill and the grant returns — once, not stacked on the withdrawn one.
    game.state.player1.stage.stage[2] = game.id(RENJO_KOKO);
    for round in 1..=4 {
        game.state.recalculate_constants();
        assert_eq!(
            bonus(&game),
            1,
            "round {round}: refilling the empty area re-grants exactly one +1"
        );
        assert_eq!(
            granted_text_count(&game, koto),
            1,
            "round {round}: exactly one granted text after the re-grant"
        );
    }
}

/// 名前が異なる is PART of the printed condition. Three areas covered by two names
/// must not grant — without this, the positive above could be passing on area
/// coverage alone and never exercising the name clause at all.
#[test]
fn koto_grant_requires_distinct_names_not_just_area_coverage() {
    let mut game = TestGame::new(load_real_database());
    let koto = game.id(KOTO);
    let uta = game.id(RENJO_UTA);

    // Three areas, but only two distinct names.
    game.state.player1.stage.stage = [koto, uta, game.new_id(RENJO_UTA)];
    game.state.recalculate_constants();

    assert_eq!(
        bonus(&game),
        0,
        "『名前が異なる』 is part of the condition: three areas holding only TWO \
         distinct names must NOT grant, so the positive test cannot be passing on \
         area coverage alone"
    );
    assert_eq!(
        granted_text_count(&game, koto),
        0,
        "no grant means no registered text either"
    );
}

/// Zone exit, rule 4.1.4: a card that changes zones is a NEW card, so the grant
/// must not outlive the host leaving the stage.
#[test]
fn koto_grant_does_not_outlive_the_host_leaving_the_stage() {
    let mut game = TestGame::new(load_real_database());
    let koto = game.id(KOTO);
    stage_three_distinct_renjo(&mut game, koto);
    game.state.recalculate_constants();
    assert_eq!(bonus(&game), 1, "precondition: the grant is live");

    game.state.player1.stage.stage[0] = -1;
    game.state.on_cards_left_zones(&[koto]);
    game.state.recalculate_constants();

    assert_eq!(
        bonus(&game),
        0,
        "a host that left the stage must carry no grant: zone exit makes it a NEW \
         card (rule 4.1.4), and a surviving +1 would score for a card on no board"
    );
    assert_eq!(
        granted_text_count(&game, koto),
        0,
        "the registration must be cleared by the same zone exit, not left behind \
         (got {} texts)",
        granted_text_count(&game, koto)
    );
}
