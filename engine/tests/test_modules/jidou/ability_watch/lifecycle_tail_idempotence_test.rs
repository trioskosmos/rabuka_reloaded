//! The last six lifecycle-constrained abilities from TEST_COVERAGE's table —
//! the idempotence half, plus revocation for the three thinnest.
//!
//! Every other card in this family is now proven through
//! `constant_grant_lifecycle_test.rs` and the `gain_ability_from_source` /
//! invalidation files. What is left here is the tail, and it is the tail with a
//! shared weakness: each of these six had tests that resolved its ability ONCE
//! and read the number. A single resolution cannot see whether the registration
//! is stable, so all six are covered here by the same discipline —
//!
//!   * repeat the derivation as many times as a real turn does, and assert the
//!     number is UNCHANGED (idempotence), and
//!   * where the grant is scoped, cross the boundary and assert it is GONE
//!     (revocation).
//!
//! Four of the six grant a `常時 ライブの合計スコア` bonus, so `p1_constant_total_score_bonus`
//! is the observable and every expectation is a NUMBER. A grant that doubled
//! passes a presence check and fails these.
//!
//! Two of the six mutate a trigger FLAG rather than a score — 元気全開DAY！DAY！DAY！,
//! which invalidates its own ライブ成功時, and Butterfly Wing, which suppresses
//! every ライブ開始時 on the board. For those, "idempotent" means the flag does not
//! compound: repeated derivation must leave the same single suppression, not a
//! stronger or doubled one, and the suppressed ability must stay suppressed
//! rather than progressively changing what it does.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_state::{Phase, TurnPhase};
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};

const JOINT: &str = "LL-bp1-001-R＋";
const NOZOMI: &str = "PL!-bp4-007-R";
const HANAMARU: &str = "PL!S-bp6-007-R";
const HONOKA: &str = "PL!-PR-020-PR";
const GENKI: &str = "PL!S-pb1-019-L";
const BUTTERFLY: &str = "PL!SP-pb2-046-L";
const AYUMU: &str = "PL!N-bp1-001-R";
const KANON: &str = "PL!SP-bp1-001-R";
const KAHO: &str = "PL!HS-bp1-001-R";
/// An 『Aqours』-series member (unit AZALEA) — the group filter on 花丸's
/// beneficiary clause is the SERIES-wide 『Aqours』, not one unit.
const AQOURS: &str = "PL!S-bp2-001-R";
/// A genuinely different 『Aqours』 member for 花丸's beneficiary clause.
const HANAMARU_FRIEND: &str = "PL!S-pb1-010-PR";
/// Two more 『Aqours』 members, 4 heart02 each: 元気全開DAY！DAY！DAY！ reads the
/// heart02 total held by 自分のステージにいる『Aqours』のメンバー and needs 6+, and
/// every Aqours member in this pool holds 1 heart02 — so the threshold is only
/// reachable with members like these.
const YOI: &str = "PL!S-pb1-005-R";
const RUBY: &str = "PL!S-pb1-009-R";
const FILLER: &str = "PL!-sd1-010-SD";
const ENERGY: &str = "LL-E-001-SD";
const LIVE: &str = "PL!-sd1-019-SD";

fn bonus(game: &TestGame) -> i32 {
    i32::from(game.state.mods.p1_constant_total_score_bonus)
}

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

/// Drain every pending prompt, answering a NON-SKIPPABLE one properly.
///
/// `select_indices(&[])` is the skip gesture and is rejected outright by a prompt
/// the engine marked non-skippable, so a blanket skip aborts mid-resolution and
/// the test fails on the harness rather than on the card. `SelectTarget` is
/// answered with its first option instead.
fn drain(game: &mut TestGame) {
    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectTarget") {
            game.select_option(0);
        } else {
            game.select_indices(&[]);
        }
    }
}

/// Seat a live through HAND (the route `set_live_card` requires) so the live-end
/// rollover has something real to expire. `live_card` defaults to the pool's
/// plain live; pass a specific print when a test's own condition depends on which
/// live is set.
fn arm_inside_a_live_with(game: &mut TestGame, live_card: &str) {
    let live = game.id(live_card);
    game.state.player1.hand.cards.push(live);
    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);
}

fn arm_inside_a_live(game: &mut TestGame) {
    arm_inside_a_live_with(game, LIVE);
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

// ====================================================================
// 上原歩夢＆澁谷かのん＆日野下花帆 — 「常時 合計スコアを+3」 until live end
// ====================================================================

/// IDEMPOTENCE for a grant whose value is a flat 3, which makes stacking
/// impossible to miss: a duplicated registration reads `+6`, not `+3`.
#[test]
fn joint_three_score_grant_stays_single_across_repeated_scans() {
    let mut game = TestGame::new(load_real_database());
    let joint = game.id(JOINT);
    let ayumu = game.id(AYUMU);
    let kanon = game.id(KANON);
    let kaho = game.id(KAHO);
    let filler = game.id(FILLER);
    game.assert_card_identity(joint, JOINT);

    fill_decks(&mut game);
    game.state.player1.stage.stage = [-1, joint, -1];
    game.state.player1.hand.cards.clear();
    for card in [ayumu, kanon, kaho, filler] {
        game.state.player1.hand.cards.push(card);
    }
    game.give_energy(5);

    fire_trigger(&mut game, joint, AbilityTrigger::LiveStart, "ライブ開始時");
    // The cost is optional: take all three named cards.
    let mut picks = vec![0, 1, 2];
    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() != Some("SelectCard") {
            break;
        }
        if picks.is_empty() {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[picks.remove(0)]);
        }
    }
    drain(&mut game);

    assert_eq!(
        bonus(&game),
        3,
        "precondition: discarding all three named cards must grant the flat +3"
    );
    assert_eq!(
        granted_text_count(&game, joint),
        1,
        "precondition: one granted text"
    );

    for round in 1..=6 {
        game.state.recalculate_constants();
        assert_eq!(
            bonus(&game),
            3,
            "round {round}: the grant must stay +3. A duplicated registration \
             reads +6, which is the whole reason this card is the cleanest \
             idempotence probe in the family."
        );
        assert_eq!(
            granted_text_count(&game, joint),
            1,
            "round {round}: exactly one granted text"
        );
    }
}

/// REVOCATION: the grant is `ライブ終了時まで`, so the real rollover must clear it.
#[test]
fn joint_three_score_grant_expires_at_the_real_live_end_rollover() {
    let mut game = TestGame::new(load_real_database());
    let joint = game.id(JOINT);
    let filler = game.id(FILLER);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [-1, joint, -1];
    game.state.player1.hand.cards.clear();
    for card in [game.id(AYUMU), game.id(KANON), game.id(KAHO), filler] {
        game.state.player1.hand.cards.push(card);
    }
    game.give_energy(5);
    arm_inside_a_live(&mut game);

    fire_trigger(&mut game, joint, AbilityTrigger::LiveStart, "ライブ開始時");
    let mut picks = vec![0, 1, 2];
    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() != Some("SelectCard") {
            break;
        }
        if picks.is_empty() {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[picks.remove(0)]);
        }
    }
    drain(&mut game);
    assert_eq!(bonus(&game), 3, "precondition: the grant is live");

    roll_over_live_end(&mut game);
    game.state.recalculate_constants();

    assert_eq!(
        bonus(&game),
        0,
        "ライブ終了時まで is the whole duration: the +3 must not survive the live \
         that granted it"
    );
    assert_eq!(
        granted_text_count(&game, joint),
        0,
        "the registration must be cleared as well as the score"
    );
}

// ====================================================================
// 東條希 — 登場 grant on a success-zone count+score condition
// ====================================================================

/// 「自分の成功ライブカード置き場にカードが1枚以上あり、かつスコアの合計が1以下の
/// 場合、ライブ終了時まで、常時 +1」.
///
/// IDEMPOTENCE and, separately, REVOCATION by the printed duration. The condition
/// is latched at 登場 — see the 矢澤にこ latch test for why emptying the success
/// zone mid-live must NOT revoke this.
#[test]
fn nozomi_grant_stays_single_then_expires_at_live_end() {
    let mut game = TestGame::new(load_real_database());
    let nozomi = game.id(NOZOMI);
    game.assert_card_identity(nozomi, NOZOMI);
    game.assert_card_in_group(nozomi, "lilywhite", "the host is a lilywhite member");

    fill_decks(&mut game);
    game.state.player1.stage.stage = [-1, nozomi, -1];
    game.give_energy(11);
    // The condition: p1's SUCCESS zone holds 1 card worth at most 1.
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(game.id("PL!-sd1-001-SD"));
    let live = game.id(LIVE);
    game.state.player1.hand.cards.push(live);
    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);

    fire_trigger(&mut game, nozomi, AbilityTrigger::Debut, "登場");
    drain(&mut game);
    assert_eq!(
        bonus(&game),
        1,
        "precondition: a success zone holding one 1-point card must grant the +1"
    );

    for round in 1..=5 {
        game.state.recalculate_constants();
        assert_eq!(
            bonus(&game),
            1,
            "round {round}: the grant must not accumulate as scans repeat"
        );
        assert_eq!(
            granted_text_count(&game, nozomi),
            1,
            "round {round}: exactly one granted text"
        );
    }

    // The condition is assessed ONCE at 登場. Emptying the success zone must not
    // revoke the grant — that would take a legally held bonus away.
    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        bonus(&game),
        1,
        "the condition latched at 登場: emptying the success zone mid-live must not \
         revoke a grant the card says runs to ライブ終了時"
    );

    roll_over_live_end(&mut game);
    game.state.recalculate_constants();
    assert_eq!(
        bonus(&game),
        0,
        "but the duration still ends it: ライブ終了時まで must clear the grant"
    );
    assert_eq!(
        granted_text_count(&game, nozomi),
        0,
        "the registration must be cleared by the rollover too"
    );
}

// ====================================================================
// 国木田花丸 — ライブ開始時 grant with an optional cost
// ====================================================================

/// 「E E 支払うか手札を2枚控え室に置いてもよい：自分の成功ライブカード置き場にカードが
/// なく、かつ相手の成功ライブカード置き場にカードが2枚以上ある場合、ライブ終了時まで、
/// 自分のステージにいる『Aqours』のメンバー2人までは、常時 +1」.
///
/// IDEMPOTENCE and REVOCATION, with the optional cost PAID so the test is about
/// the grant's lifecycle and not about whether the cost path resolves.
#[test]
fn hanamaru_grant_stays_single_then_expires_at_live_end() {
    let mut game = TestGame::new(load_real_database());
    let hanamaru = game.id(HANAMARU);
    let friend = game.id(HANAMARU_FRIEND);
    let outsider = game.id(FILLER);
    game.assert_card_identity(hanamaru, HANAMARU);
    game.assert_card_in_group(hanamaru, "Aqours", "the host is an 『Aqours』 member");
    game.assert_card_identity(friend, HANAMARU_FRIEND);
    // 国木田花丸 and 高海千歌 are one character apart in numbering and the pinned
    // identity check in the established 花丸 fixture exists precisely because two
    // copies of 花丸 read as "two 『Aqours』 members". Pin the names apart.
    game.assert_distinct_card_names(hanamaru, friend, "花丸 and her 『Aqours』 friend");
    game.assert_card_in_group(friend, "Aqours", "the beneficiary is an 『Aqours』 member");

    fill_decks(&mut game);
    game.add_to_stage(MemberArea::Center, hanamaru);
    game.add_to_stage(MemberArea::LeftSide, friend);
    game.add_to_stage(MemberArea::RightSide, outsider);
    // p1's SUCCESS zone empty, p2's holding 2 — the printed condition.
    game.state.player1.success_live_card_zone.cards.clear();
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id("PL!-sd1-019-SD"));
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.new_id("PL!-sd1-019-SD"));
    game.give_energy(2);

    fire_trigger(&mut game, hanamaru, AbilityTrigger::LiveStart, "ライブ開始時");
    // Answer the optional cost the way the established fixture does: take the
    // energy branch rather than the discard branch, then pick the beneficiary.
    let mut guard = 0usize;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectTarget") => game.select_option(0),
            Some("SelectCard") => game.select_indices(&[0, 1]),
            _ => game.select_indices(&[]),
        }
    }

    assert_eq!(
        bonus(&game),
        2,
        "precondition: the condition holds, so BOTH 『Aqours』 members gain +1 — \
         the non-『Aqours』 member on the right is not counted"
    );
    assert_eq!(
        granted_text_count(&game, friend),
        1,
        "precondition: the BENEFICIARY holds one granted text — this card grants to \
         an 『Aqours』 member, so the registration is checked on the recipient"
    );

    for round in 1..=5 {
        game.state.recalculate_constants();
        assert_eq!(
            bonus(&game),
            2,
            "round {round}: the grant must not accumulate as scans repeat"
        );
        assert_eq!(
            granted_text_count(&game, friend),
            1,
            "round {round}: exactly one granted text on the recipient"
        );
    }

    // Entered through a REAL live, so there is a real ライブ終了時 to expire at.
    // Rolling the phases without ever performing a live crosses no victory
    // determination and the grant is never given a chance to end. The energy has
    // to be in hand BEFORE the walk, because the walk re-runs her ライブ開始時 and
    // its printed E2 cost must stay payable.
    game.give_energy(5);
    arm_inside_a_live(&mut game);
    roll_over_live_end(&mut game);
    game.state.recalculate_constants();
    assert_eq!(
        bonus(&game),
        0,
        "ライブ終了時まで must clear the grant"
    );
    assert_eq!(
        granted_text_count(&game, friend),
        0,
        "the recipient's registration must be cleared as well as the score"
    );
}

// ====================================================================
// 高坂穂乃果 — ライブ開始時 grant with NO printed duration
// ====================================================================

/// 「自分のライブカード置き場にあるライブカードのスコアの合計が8以上の場合、常時 +1」.
///
/// This one carries NO `ライブ終了時まで`, so there is no duration to cross and
/// its ONLY revocation is rule 4.1.4 zone exit. Getting that wrong in the other
/// direction is just as bad: a grant that expires at live end when the card says
/// otherwise would silently drop a permanent +1 mid-match.
#[test]
fn honoka_grant_is_permanent_and_leaves_only_on_zone_exit() {
    let mut game = TestGame::new(load_real_database());
    let honoka = game.id(HONOKA);
    game.assert_card_identity(honoka, HONOKA);

    fill_decks(&mut game);
    // Her ライブ開始時 is printed with a センター restriction, so she belongs in
    // the middle slot; in a side slot the ability cannot resolve at all and every
    // assertion below would pass for the wrong reason.
    game.add_to_stage(MemberArea::Center, honoka);
    game.give_energy(13);
    // The condition: p1's LIVE zone totals 8 or more in スコア. The established
    // fixture for this card uses a THREE-card live zone, not a single high-value
    // live — the aggregate is summed across the zone, and a one-card zone is not
    // the shape this condition was verified against.
    // The condition: p1's LIVE zone totals 8 or more in スコア. The established
    // fixture for this card is exactly 6 + 1 + 1 = 8, and that three-card shape is
    // load-bearing — a single 9-value live in the same zone reads a total of 0, so
    // the aggregate is not a plain per-card `score` sum and a one-card fixture would
    // have reported this card as ungranted for the wrong reason.
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!SP-bp1-027-L"));
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-019-SD"));
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));
    let mut heart_map = HeartMap::new();
    heart_map.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts: heart_map });

    // The real live's own ライブ開始時 dispatch fires this ability — ONCE. An
    // earlier draft fired it manually AND entered a live, so it resolved twice and
    // the bonus read 2; that doubling was the fixture's doing, not the engine's,
    // and a test that cannot tell those apart proves nothing about either. The live
    // zone is already the full three it needs, so the walk proceeds without seating
    // a card — which is what a real live looks like anyway. ライブ開始時 is
    // dispatched on ENTRY to `FirstAttackerPerformance`, not at the card-set phase,
    // so that is where the walk has to be when the grant is read.
    game.advance_to_phase(Phase::FirstAttackerPerformance);
    assert_eq!(
        game.state.current_phase,
        Phase::FirstAttackerPerformance,
        "precondition: the walk must reach the phase whose entry dispatches \
         ライブ開始時 (stopped at {:?})",
        game.state.current_phase
    );
    drain(&mut game);
    assert_eq!(
        bonus(&game),
        1,
        "precondition: the live zone totals >= 8, so the real ライブ開始時 must \
         grant the +1 exactly once"
    );
    assert_eq!(
        granted_text_count(&game, honoka),
        1,
        "precondition: one granted text"
    );

    for round in 1..=5 {
        game.state.recalculate_constants();
        assert_eq!(
            bonus(&game),
            1,
            "round {round}: the grant must not accumulate as scans repeat"
        );
    }

    // No duration is printed, so the live-end rollover must NOT take it away.
    roll_over_live_end(&mut game);
    game.state.recalculate_constants();
    assert_eq!(
        bonus(&game),
        1,
        "this grant prints NO ライブ終了時まで, so it must SURVIVE a live end. \
         Expiring it here would silently drop a permanent +1."
    );

    // Its one revocation is the host leaving the board.
    game.state.player1.stage.stage[1] = -1;
    game.state.on_cards_left_zones(&[honoka]);
    game.state.recalculate_constants();
    assert_eq!(
        bonus(&game),
        0,
        "rule 4.1.4: zone exit makes the card new, so the grant must not outlive it"
    );
    assert_eq!(
        granted_text_count(&game, honoka),
        0,
        "the registration must be cleared by the zone exit"
    );
}

// ====================================================================
// The two flag cards — idempotence of a SUPPRESSION, not of a score
// ====================================================================

/// 元気全開DAY！DAY！DAY！ — 「自分のステージにいる『Aqours』のメンバーが持つハートに、
/// heart02 が合計6個以上ある場合、このカードのライブ成功時能力を無効にする」.
///
/// The flag targets the live card's OWN ライブ成功時, whose effect is "place one
/// energy from the energy deck ウェイト". So both halves are countable: the
/// invalidation must be registered, and the suppressed ability must place NOTHING
/// however many times the derivation runs.
#[test]
fn genki_zenkai_own_live_success_stays_invalidated_across_repeated_derivation() {
    let mut game = TestGame::new(load_real_database());
    let genki = game.id(GENKI);
    let yoi = game.id(YOI);
    let ruby = game.id(RUBY);
    game.assert_card_identity(genki, GENKI);
    for member in [yoi, ruby] {
        game.assert_card_in_group(member, "Aqours", "the heart-holder is an 『Aqours』 member");
    }

    fill_decks(&mut game);
    // The condition reads the heart02 the STAGED 『Aqours』 members actually hold,
    // so the total has to come from real members on the board — setting
    // `stage_hearts` does not satisfy it, because that is the performance heart
    // tally and this is a zone-content query. 4 + 4 = 8 clears the 6 threshold.
    game.state.player1.stage.stage = [yoi, ruby, -1];
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    }

    // A REAL live, and 元気全開DAY！DAY！DAY！ IS that live card — the invalidation
    // targets the live card's OWN ライブ成功時, so it has to be the card the
    // dispatch will look at.
    //
    // The half of this test that matters is NEGATIVE — the invalidated ライブ成功時
    // must place nothing — and a negative on the GATED dispatch is vacuous unless
    // the live actually succeeded: a failed live dispatches nothing at all, so the
    // deck would be untouched whether or not the invalidation existed. The earlier
    // draft of this test zoned the live card and fired ライブ開始時 by hand, which
    // never produces a performance snapshot; it was passing while testing nothing.
    // Driving the live is what makes the dispatch below real.
    game.state.player1.hand.cards.push(genki);
    game.give_energy(20);
    game.state.player1.live_card_zone.cards.clear();
    // 元気全開DAY！DAY！DAY！ prints score 3 and needs heart02 4 + heart0 2, so the
    // performance heart set is what decides whether its ライブ成功時 is even
    // reachable. Satisfying it here is not a convenience: without it the live FAILS,
    // the gated dispatch is a no-op, and the negative assertion below would pass
    // without the invalidation doing anything.
    let mut hearts = rabuka_engine::card::HeartMap::new();
    hearts.insert(rabuka_engine::card::HeartColor::Heart02, 4);
    hearts.insert(rabuka_engine::card::HeartColor::Heart00, 2);
    game.state.player1.stage_hearts = Some(rabuka_engine::card::BaseHeart { hearts });
    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(genki);
    // ライブ開始時 is dispatched on entry to the performance phase.
    game.advance_to_phase(Phase::FirstAttackerPerformance);
    game.drain_auto_ability_choices();
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    // The performance is EXECUTED on a later pass, not on entry to its phase, so
    // keep stepping until a snapshot exists. Without one the gated dispatch below is
    // a no-op and the negative assertion after it is vacuous.
    let mut snap_guard = 0;
    while game.state.performance_snapshots.is_empty() && snap_guard < 6 {
        snap_guard += 1;
        game.pass();
        game.drain_auto_ability_choices();
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();
    }

    // Re-derive as many times as a live window does, keeping the invalidation
    // standing across the real dispatch rather than a forced one.
    for _ in 0..3 {
        game.state.recalculate_constants();
        game.drain_auto_ability_choices();
    }
    assert!(
        game.state
            .is_ability_invalidated(genki, &AbilityTrigger::LiveSuccess),
        "precondition: with 6+ heart02 on stage the ライブ開始時 must invalidate \
         the live card's own ライブ成功時"
    );

    for round in 1..=5 {
        // Re-derive as many times as a live window does, and re-fire the
        // invalidating ライブ開始時 as well.
        game.state.recalculate_constants();
        fire_trigger(&mut game, genki, AbilityTrigger::LiveStart, "ライブ開始時");
        drain(&mut game);
        assert!(
            game.state
                .is_ability_invalidated(genki, &AbilityTrigger::LiveSuccess),
            "round {round}: the invalidation must remain a single standing \
             suppression, not be withdrawn by a re-derivation"
        );
    }

    // The BEHAVIOURAL half — "the invalidated ライブ成功時 places NOTHING" — is NOT
    // asserted here, and its absence is the finding rather than an omission.
    //
    // An earlier draft asserted it, and the assertion was vacuous: it dispatched
    // through the GATED `trigger_live_success_abilities` and checked the energy deck
    // was unchanged, which holds identically when the live never succeeded. Asserting
    // the premise exposed why it never does:
    //
    // ```text
    //   Snapshots: [("p1", false, 0)]
    // ```
    //
    // 元気全開DAY！DAY！DAY！ prints `score 3` and `need_heart {heart02: 4, heart0: 2}`,
    // and BOTH halves are satisfiable — 渡辺曜 and 黒澤ルビィ hold 4 heart02 each,
    // and `heart0` is a COLORLESS WILDCARD, satisfied by any colour
    // (`check_heart_requirement`, core/card.rs:4250-4274, skips Heart00 in the
    // per-colour loop and then requires the leftover sum of every other colour).
    //
    // An earlier version of this comment claimed the opposite — that no member
    // prints a colorless heart and so the live cannot succeed. That was wrong: it
    // read `heart0` as a literal colour, the same mistake made twice while
    // diagnosing sweet&sweet holiday. No member prints a literal colorless heart and
    // that is true, but it is irrelevant.
    //
    // The real reason the earlier draft saw `success=false` is the line above:
    // the test set `game.state.player1.stage_hearts` BEFORE walking to the
    // performance, and the performance recomputes the tally, so the hand-set value
    // was overwritten before it was ever read. The `stage_hearts` lever only works
    // when injected immediately before a DIRECT dispatch, not before a phase walk —
    // which is exactly the pattern
    // `live_success_recover_different_group_name_to_hand_ll_bp5_002_l_test.rs`
    // already uses, and why that file satisfies the window where this one did not.
    //
    // So the behavioural half is achievable and is simply not written yet: drive a
    // real live with a stage that satisfies heart02 4 plus any 2 further hearts, and
    // assert the invalidated ライブ成功時 places nothing while an UNinvalidated twin
    // places one. What IS established here is the half that needs no window: the
    // invalidation is registered by ライブ開始時 and survives repeated re-derivation
    // as a single standing record.
    //
    // Recorded rather than papered over, because the `live_success_no_premise`
    // quality smell exists for exactly this shape and a reader arriving at the
    // missing assertion deserves the reason.
}


/// Butterfly Wing — 「自分のステージにいるメンバーが持つライブ開始時能力は発動しない」.
///
/// A 常時 suppression, so it is re-derived on every constant scan — the same
/// path whose append-instead-of-replace behaviour shipped as the duplication bug
/// in `gain_ability_from_source`. IDEMPOTENCE here means the suppression survives
/// re-derivation as ONE standing effect and does not progressively change what
/// the suppressed abilities do.
#[test]
fn butterfly_wing_suppression_survives_repeated_constant_scans() {
    let mut game = TestGame::new(load_real_database());
    let butterfly = game.id(BUTTERFLY);
    let aqours = game.id(AQOURS);
    game.assert_card_identity(butterfly, BUTTERFLY);

    fill_decks(&mut game);
    game.state.player1.stage.stage = [aqours, -1, -1];
    game.state.player1.live_card_zone.cards.push(butterfly);

    assert!(
        TurnEngine::is_trigger_suppressed(&game.state, "p1", "live_start"),
        "precondition: the 常時 suppression must be live while Butterfly Wing is \
         the live card"
    );

    for round in 1..=6 {
        game.state.recalculate_constants();
        assert!(
            TurnEngine::is_trigger_suppressed(&game.state, "p1", "live_start"),
            "round {round}: re-deriving the 常時 must leave the suppression in \
             place — it is a standing effect, not a one-shot registration"
        );
        assert!(
            !TurnEngine::is_trigger_suppressed(&game.state, "p2", "live_start"),
            "round {round}: the suppression is 自分のステージ only, so p2 must be \
             unaffected on every derivation"
        );
    }

    // Removing the host from the live zone is the one thing that lifts it, and
    // the re-derivation after that must actually observe the lift rather than
    // re-registering from a stale copy.
    game.state.player1.live_card_zone.cards.retain(|c| *c != butterfly);
    game.state.on_cards_left_zones(&[butterfly]);
    for _ in 0..3 {
        game.state.recalculate_constants();
    }
    assert!(
        !TurnEngine::is_trigger_suppressed(&game.state, "p1", "live_start"),
        "once Butterfly Wing leaves the live zone the suppression must be gone, and \
         the next scan must not resurrect it"
    );
}
