//! Consequence half of 澁谷かのん's 登場 invalidation — 「…のすべての
//! {{live_start.png|ライブ開始時}}能力を、ライブ終了時まで、無効にしてもよい」.
//!
//! `kanon_debut_invalidate_recovers_discard_card_test.rs` pins the GRANT:
//! the prompt, the skip, the Q106 double-invalidation refusal, the recovery
//! out of the waitroom, and the flag's expiry at live end. It never drives the
//! thing the invalidation is FOR — the silenced card's ライブ開始時 not
//! resolving — and it never shows the ability coming BACK.
//!
//! Two things make the difference between "the flag is set" and "the ability
//! did not fire":
//!
//!  * The existing expiry test asserts `blade == 0` on 嵐千砂都
//!    (`PL!SP-sd1-003-SD`), whose ライブ開始時 is 「手札を2枚控え室に置いてもよい」 —
//!    OPTIONAL. So `blade == 0` also holds when the ability was never
//!    *offered*, which makes that negative ambiguous. Here the target is 米女メイ
//!    (`PL!SP-pb1-007-R`), whose ライブ開始時 「エネルギーを2枚アクティブにする」
//!    is unconditional and has exactly one observable: the active-energy
//!    counter. Silence therefore means "the counter did not move", which no
//!    un-offered prompt can imitate.
//!  * The invalidation is scoped to 自分のステージ. Nothing in the suite
//!    stages an opponent's copy of the same print, so 「own stage only」 is
//!    currently untested in the direction that could break.
//!
//! Both tests below read the effect at the LiveStart dispatch itself, on entry
//! to `FirstAttackerPerformance`, and never depend on the live SUCCEEDING —
//! ライブ開始時 is scanned before the heart check can matter, so a live with
//! unmet requirements still exercises the trigger under test.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_state::Phase;
use rabuka_engine::zones::MemberArea;

const KANON: &str = "PL!SP-bp2-001-R＋";
/// 米女メイ — 『Liella!』 (unit CatChu!), ライブ開始時 「エネルギーを2枚
/// アクティブにする。」 Unconditional, and the active-energy counter is its
/// only observable.
const MEI: &str = "PL!SP-pb1-007-R";
const FILLER: &str = "PL!-sd1-010-SD";
const ENERGY: &str = "LL-E-001-SD";
const RECOVERY: &str = "PL!SP-sd1-001-SD";

/// Walk to the live's ライブ開始時 dispatch and return the active-energy delta
/// that entering `FirstAttackerPerformance` produced, for p1 and p2.
///
/// The energy zones are seeded at `LiveCardSetSecondAttacker` — the last point
/// where both live cards are in place and the Energy phase is behind us. That
/// timing is load-bearing, not cosmetic: the Energy phase re-seats the zone
/// (return-to-deck + charge, and it re-reads an empty `energy_deck`), so a zone
/// seeded before the walk is re-shaped on the way in and 「2枚アクティブ」 finds
/// no waited energy to activate. The effect then no-ops and the measurement
/// reads 0 for a reason that has nothing to do with the invalidation.
///
/// The baseline is the previous step, because an earlier phase in the window may
/// still move a counter; `LIVE_START_TRIGGER` fires on entry to
/// `FirstAttackerPerformance` (`phases.rs`).
fn live_start_energy_delta(game: &mut TestGame, wanted: i16) -> (i32, i32) {
    for _ in 0..24 {
        if game.state.current_phase == Phase::LiveCardSetSecondAttacker {
            break;
        }
        game.pass();
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();
    }
    assert_eq!(
        game.state.current_phase,
        Phase::LiveCardSetSecondAttacker,
        "the walk never reached the point where both live cards are set \
         (stuck at {:?}); wanted {:?}",
        game.state.current_phase, wanted
    );
    stage_energy(game, 6, 4);

    let mut last = (
        game.state.player1.energy_zone.active_count() as i32,
        game.state.player2.energy_zone.active_count() as i32,
    );
    for _ in 0..16 {
        if game.state.current_phase == Phase::FirstAttackerPerformance {
            return (
                game.state.player1.energy_zone.active_count() as i32 - last.0,
                game.state.player2.energy_zone.active_count() as i32 - last.1,
            );
        }
        last = (
            game.state.player1.energy_zone.active_count() as i32,
            game.state.player2.energy_zone.active_count() as i32,
        );
        game.pass();
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();
    }
    panic!(
        "the turn never reached FirstAttackerPerformance within 16 passes (stuck \
         at {:?}); wanted {:?}",
        game.state.current_phase, wanted
    );
}

/// Fill both energy zones so 2枚アクティブ has waited energy to activate, and
/// leave the decks empty so no Energy phase inside the live window can refill
/// a zone and be charged to the trigger.
///
/// Each zone gets its OWN card instances (`new_id`), not one shared id stamped
/// `cards` times into both: a zone's active/waited split is tracked per zone,
/// and a single id sitting in two zones at once makes "how many cards does this
/// zone hold" ambiguous in a way that quietly zeroes one side's measurement.
fn stage_energy(game: &mut TestGame, cards: usize, active: u8) {
    game.state.player1.energy_zone.cards.clear();
    game.state.player2.energy_zone.cards.clear();
    for _ in 0..cards {
        let card = game.new_id(ENERGY);
        game.state.player1.energy_zone.cards.push(card);
        game.state.player2.energy_zone.cards.push(card);
    }
    game.state.player1.energy_zone.set_active_count(active);
    game.state.player2.energy_zone.set_active_count(active);
    assert!(
        game.state.player1.energy_zone.cards.len()
            - game.state.player1.energy_zone.active_count() as usize
            >= 2,
        "p1 must keep waited energy so 2枚アクティブ has something to activate \
         (p1: {} cards / {} active)",
        game.state.player1.energy_zone.cards.len(),
        game.state.player1.energy_zone.active_count()
    );
    assert!(
        game.state.player2.energy_zone.cards.len()
            - game.state.player2.energy_zone.active_count() as usize
            >= 2,
        "p2 must keep waited energy so 2枚アクティブ has something to activate \
         (p2: {} cards / {} active)",
        game.state.player2.energy_zone.cards.len(),
        game.state.player2.energy_zone.active_count()
    );
    assert!(game.state.player1.energy_deck.cards.is_empty());
    assert!(game.state.player2.energy_deck.cards.is_empty());
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

/// Control run: 米女メイ on stage with NO invalidation. Proves the observable
/// this file reads is really +2, so a later "0" means the invalidation and not
/// a target that never fires.
fn control_delta_without_invalidation() -> i32 {
    let mut game = TestGame::new(load_real_database());
    let mei = game.id(MEI);
    let filler = game.id(FILLER);
    game.assert_card_identity(mei, MEI);
    // The printed unit is CatChu!, a DIFFERENT 『Liella!』 unit from Kanon's own
    // 5yncri5e!. 澁谷かのon's picker filters on the series-wide 『Liella!』, so pin
    // that group — pinning her unit instead would quietly narrow the claim to
    // "Kanon can name her own unit", which is not what the card says.
    game.assert_card_in_group(mei, "Liella!", "the silenced target is inside 『Liella!』");

    game.state.player1.stage.stage = [mei, filler, -1];
    game.state.player1.live_card_zone.cards.push(game.id("PL!-sd1-019-SD"));
    fill_decks(&mut game);

    live_start_energy_delta(&mut game, mei).0
}

/// CONTROL: the un-invalidated ライブ開始時 activates exactly 2 energy.
///
/// Without this, every negative below would also pass for a target whose
/// ability never fires at all.
#[test]
fn mei_live_start_activates_two_energy_when_not_invalidated() {
    let delta = control_delta_without_invalidation();
    assert_eq!(
        delta, 2,
        "control: 米女メイ's ライブ開始時 must move the active counter by 2, \
         otherwise the silenced assertions below are vacuous"
    );
}

/// THE GAP: 澁谷かのん's 登場 invalidation silences the target's ライブ開始時 in
/// the live it was cast in, and the ability is BACK for the next live.
///
/// One test, both halves, because the two halves are only meaningful together:
/// a suppression that never lifts and a suppression that never lands look
/// identical from either side alone.
#[test]
fn kanon_invalidation_silences_the_live_start_then_restores_it_next_live() {
    // Control first: pin that the untouched board really does fire.
    assert_eq!(control_delta_without_invalidation(), 2, "control");

    let mut game = TestGame::new(load_real_database());
    let kanon = game.id(KANON);
    let mei = game.id(MEI);
    let live = game.id("PL!-sd1-019-SD");
    let recovery = game.id(RECOVERY);

    game.assert_card_identity(kanon, KANON);
    game.assert_card_identity(mei, MEI);
    game.assert_card_in_group(mei, "Liella!", "the silenced target is inside 『Liella!』");
    game.assert_card_in_group(kanon, "Liella!", "Kanon's own group is 『Liella!』");

    fill_decks(&mut game);
    // Kanon's invalidation is optional, so the prompt is what arms it. She costs
    // 13; the recovery comes out of the waitroom, not the deck.
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.waitroom.cards.push(recovery);
    game.give_energy(13);
    game.state.player1.stage.stage = [mei, -1, -1];
    game.state.player1.live_card_zone.cards.push(live);

    game.play_to_stage(kanon, MemberArea::Center);
    // Accept the optional invalidation: the only eligible target is 米女メイ.
    assert!(
        game.has_pending_choice(),
        "Kanon's debut must offer the optional invalidation"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected a target selection for the invalidation"
    );
    game.select_indices(&[0]);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    assert!(
        game.state.is_ability_invalidated(mei, &AbilityTrigger::LiveStart),
        "precondition: the invalidation must actually be registered on 米女メイ"
    );

    // --- Live 1: silenced -------------------------------------------------
    let silenced = live_start_energy_delta(&mut game, mei).0;
    assert_eq!(
        silenced, 0,
        "the invalidated ライブ開始時 must NOT activate energy"
    );
    assert!(
        game.state.is_ability_invalidated(mei, &AbilityTrigger::LiveStart),
        "the invalidation is live-end scoped, so it must still hold inside the live"
    );

    // --- live end: the invalidation lifts ---------------------------------
    for _ in 0..16 {
        if game.state.current_phase == Phase::Active {
            break;
        }
        game.pass();
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();
    }
    assert_eq!(
        game.state.current_phase,
        Phase::Active,
        "the walk never reached ライブ終了時"
    );
    assert!(
        !game.state.is_ability_invalidated(mei, &AbilityTrigger::LiveStart),
        "ライブ終了時まで is the whole duration: the invalidation must be gone"
    );

    // --- Live 2: restored -------------------------------------------------
    // The shared helper re-seeds the zones at the last moment before the
    // dispatch, which is also why the second live needs a live card again —
    // ライブ開始時 is scanned per live, and p1's live card left the zone when the
    // first one ended.
    fill_decks(&mut game);
    game.state.player1.live_card_zone.cards.push(live);
    assert!(
        !game.state.is_ability_invalidated(mei, &AbilityTrigger::LiveStart),
        "precondition: nothing may re-arm the invalidation between the lives"
    );

    let restored = live_start_energy_delta(&mut game, mei).0;
    assert_eq!(
        restored, 2,
        "the next live must run 米女メイ's ライブ開始時 again — an invalidation \
         that is removed but not forgotten, or one that outlives ライブ終了時, \
         both show up here"
    );
}

/// The invalidation is 自分のステージ — an opponent's copy of the same print
/// is untouched, and its ライブ開始時 still resolves.
///
/// Parallels `butterfly_suppresses_only_the_owners_live_start_members` for the
/// suppression path; this is the same owner-scoping claim for the invalidation
/// path, which no test made.
#[test]
fn kanon_invalidation_does_not_touch_the_opponents_copy() {
    let mut game = TestGame::new(load_real_database());
    let kanon = game.id(KANON);
    let mei_p1 = game.id(MEI);
    let mei_p2 = game.new_id(MEI);
    let recovery = game.id(RECOVERY);
    assert_ne!(
        mei_p1, mei_p2,
        "the two stages must hold two DISTINCT card instances"
    );

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.waitroom.cards.push(recovery);
    game.give_energy(13);
    game.state.player1.stage.stage = [mei_p1, -1, -1];
    game.state.player2.stage.stage = [mei_p2, -1, -1];
    game.state.player1.live_card_zone.cards.push(game.id("PL!-sd1-019-SD"));
    // p2 needs a live card of its own: the second attacker's ライブ開始時 is only
    // scanned for a player who is holding a live card (`triggers.rs` guards a
    // live-less second attacker), and this harness drives p1's actions, so p2's
    // reveal is placed directly — the established idiom.
    game.state.player2.live_card_zone.cards.push(game.id("PL!-sd1-019-SD"));

    game.play_to_stage(kanon, MemberArea::Center);
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "Kanon's debut must offer the optional invalidation"
    );
    // The picker is 自分のステージにある『Liella!』のメンバー, so the only
    // offered candidate is p1's instance. If the engine ever offered p2's too,
    // taking index 0 must still be p1's, and the p2 assertion below is what
    // actually decides the test.
    game.select_indices(&[0]);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    assert!(
        game.state.is_ability_invalidated(mei_p1, &AbilityTrigger::LiveStart),
        "precondition: p1's instance must be invalidated"
    );
    assert!(
        !game.state
            .is_ability_invalidated(mei_p2, &AbilityTrigger::LiveStart),
        "澁谷かのon's invalidation is scoped to 自分のステージ: p2's instance of \
         the same print must be untouched"
    );
    assert!(
        !game.state.player2.stage.stage.contains(&mei_p1),
        "precondition: the two instances really are on opposite stages"
    );

    // p1's silenced, p2's not: the same shared measurement the control uses, so
    // `2` on p2 means the same thing here as it does in the control run.
    let (p1_delta, p2_delta) = live_start_energy_delta(&mut game, mei_p1);
    assert_eq!(
        p1_delta, 0,
        "the invalidated member's ライブ開始時 must not activate p1's energy"
    );
    assert_eq!(
        p2_delta, 2,
        "the opponent's identical printed ライブ開始時 must still resolve"
    );
}
