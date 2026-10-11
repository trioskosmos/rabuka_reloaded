//! Spring Le Bouquet `PL!-pb2-037-L` (ライブ成功時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ成功時 — エールにより公開された自分のメンバーカードが**すべて『Printemps』か、
//!   すべて『lily white』か、すべて『BiBi』のいずれかの場合**、エールにより公開された
//!   自分のカードの中からメンバーカードを1枚手札に加える。
//!
//! In English: if the member cards the yell revealed are ALL Printemps, or ALL lily
//! white, or ALL BiBi, take one member card from the reveal to hand.
//!
//! # The clause, and what it takes to evaluate it
//!
//! Three-way ALL-of-one-group. 「すべて」 is load-bearing on every branch: 「a
//! Printemps member **and** a BiBi member were revealed」 is not 「すべて『Printemps』」,
//! so a mixed reveal must take nothing.
//!
//! Two engine faults made that false in both directions, and each one alone would
//! have hidden the other. Both are fixed; the negatives below are what pins them.
//!
//! 1. **`all_members` did not survive on a non-Group condition.** The marker was a
//!    field of the `Group` variant only, so branch 1 — which the parser types as a
//!    `revealed_cards` `location_condition` — lost it at decode and silently became
//!    an ANY test. `PL!-pb2-037-L` is the card that exposed it, and the same 「すべて」
//! then meant two different things inside one `or`: ALL on branches 2 and 3, "at
//!    least one" on branch 1. Fixed by moving the field onto `ConditionCommon`, which
//!    is what `docs/PBLL02_ABILITY_REVIEW.md` §8.3 item 20 named as the real fix.
//! 2. **`evaluate_group_condition`'s `all_members` branch had no `revealed_cards`
//!    arm.** Its zone dispatch covered Stage / LiveCardZone / SuccessLiveZone and
//!    answered `false` for everything else, so branches 2 and 3 could never match at
//!    any reveal size — not even two cards of their own group.
//!
//! # What this file previously claimed, and why it was wrong
//!
//! It carried three `open_gap_*` tests asserting that nothing is taken for two
//! Printemps, two lily white, and two BiBi members, and a fourth test recording that
//! ONE member is taken while TWO are not. None of that was an engine defect.
//!
//! The helper ended in `drain_auto_ability_choices()`, which only answers
//! `Choice::SelectAutoAbility` and breaks on anything else. 「1枚」 out of a multi-card
//! reveal raises a `Select 1 card from revealed cards` prompt
//! (`move/mod.rs`: `take_count < matching.len()` → prompt), so the helper left it
//! standing and the card did nothing — while the ONE-card case took the `Exact` path
//! with no prompt at all and appeared to work. That count-shaped asymmetry is what the
//! file read as "the gate matches at one card and stops matching past it", and it is
//! the shape of a plausible engine bug that does not exist.
//!
//! It is the trap `docs/PBLL02_ABILITY_REVIEW.md` §8.22 records: *read the state you
//! are measuring before you answer it*. A third of the fault was the fixture — the
//! lily white constant was `PL!-pb1-029-SRL`, which is a **ライブ** card, not a member.
//! A reveal of two of them has no member in it at all, so that case could never have
//! tested 「すべて『lily white』」. Every fixture here now asserts its own group.

use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::core::types::Phase;
use rabuka_engine::turn::TurnEngine;

const SPRING: &str = "PL!-pb2-037-L";
const FILLER: &str = "PL!-sd1-010-SD";
/// One member per μ's unit, plus one that is not μ's. All ability-free, so nothing
/// but the ability under test can contribute.
const P: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
const LW: &str = "PL!-pb2-034-N"; // 東條 希, lily white
const BB: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, BiBi
const NON_MUS: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

/// Succeed the live with `revealed` as the yell's reveal and return the revealed
/// member that reached hand, if any.
///
/// Every prompt the ability raises is answered with `drain_choices_strict`. The
/// `SelectCard` in the allow-list is not a convenience — it is the point: 「1枚」 out
/// of a multi-card reveal is a real choice, and a helper that does not answer it
/// measures its own silence. See the header.
fn taken_member(revealed: &[&str]) -> Option<String> {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let f = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }

    // Pin every fixture before staging it. A wrong constant is silent — the number
    // resolves, the card exists, and the resulting failure reads as a deep engine
    // bug (docs/PBLL02_ABILITY_REVIEW.md §8.11).
    for (card_no, group) in [(P, "Printemps"), (LW, "lily white"), (BB, "BiBi")] {
        let id = game.id(card_no);
        game.assert_card_identity(id, card_no);
        assert!(
            game.db.get_card(id).is_some_and(|c| c.is_member()),
            "{card_no} must be a メンバー — the clause is about 公開された自分のメンバーカード"
        );
        game.assert_card_in_group(id, group, "revealed-member fixture");
    }
    let non_mus = game.id(NON_MUS);
    game.assert_card_identity(non_mus, NON_MUS);
    game.assert_card_in_group(non_mus, "AZALEA", "the non-μ's control must NOT match");

    let spring = game.id(SPRING);
    game.assert_card_identity(spring, SPRING);
    game.state.player1.live_card_zone.cards.push(spring);
    for c in revealed {
        game.state.revealed_cards.push(game.id(c));
    }

    let mut h = BaseHeart { hearts: HeartMap::new() };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    game.state.current_phase = Phase::LiveVictoryDetermination;
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    game.drain_choices_strict(&["SelectAutoAbility", "SelectCard"], &[0]);

    game.state
        .player1
        .hand
        .cards
        .iter()
        .filter_map(|id| game.db.get_card(*id))
        .find(|c| c.is_member())
        .map(|c| c.card_no.to_string())
}

/// All three branches of the three-way 「すべて…のいずれか」 choice, at the size
/// that raises a prompt. Two cards of one group is the case the old file called a
/// defect; it is the plain reading of the card.
#[test]
fn all_printemps_takes_one() {
    assert_eq!(
        taken_member(&[P, P]).as_deref(),
        Some(P),
        "two Printemps members revealed is すべて『Printemps』, so one of them is taken"
    );
}

#[test]
fn all_lily_white_takes_one() {
    assert_eq!(
        taken_member(&[LW, LW]).as_deref(),
        Some(LW),
        "すべて『lily white』 is the second branch. This case needs a lily white \
         MEMBER: the constant this replaced was a ライブ card, so the reveal held \
         no member and the branch had nothing to act on"
    );
}

#[test]
fn all_bibi_takes_one() {
    assert_eq!(
        taken_member(&[BB, BB]).as_deref(),
        Some(BB),
        "すべて『BiBi』 is the third branch"
    );
}

/// One card of one group. 「すべて」 over a single card is satisfied by that card, so
/// this must work — and it takes the `Exact` path with no prompt, which is why the
/// old helper appeared to work here and nowhere else.
#[test]
fn a_single_member_of_one_group_qualifies() {
    assert_eq!(
        taken_member(&[P]).as_deref(),
        Some(P),
        "one Printemps member is all Printemps"
    );
}

/// THE discriminating case for 「すべて」.
///
/// Two different μ's units is not 「すべて『Printemps』か、すべて『lily white』か、
/// すべて『BiBi』のいずれか」 — the reveal is not all of ONE group. Under the dropped-
/// marker bug this wrongly took a card, because branch 1 had degraded to "at least
/// one Printemps member".
#[test]
fn two_different_mus_units_do_not_qualify() {
    assert_eq!(
        taken_member(&[P, BB]),
        None,
        "a Printemps member and a BiBi member together are NOT all of one group"
    );
}

#[test]
fn a_non_mus_member_does_not_qualify() {
    assert_eq!(
        taken_member(&[NON_MUS, NON_MUS]),
        None,
        "neither Printemps, lily white nor BiBi — a reveal of one other group is out \
         of scope of all three branches"
    );
}

/// A μ's group and a non-μ's one is not all-of-anything, and it must fail for the
/// same reason as the μ's/μ's mix rather than by escaping on the group test alone.
#[test]
fn a_mus_and_a_non_mus_member_do_not_qualify() {
    assert_eq!(
        taken_member(&[P, NON_MUS]),
        None,
        "the reveal is not all of one μ's group"
    );
}

/// Nothing member-shaped revealed, so there is nothing to take.
///
/// This does NOT settle how an empty reveal reads against 「すべて…のいずれか」 — a
/// universal is vacuously true over an empty set, which is the reading the repo
/// already uses for 「〜のみ」 (`an_empty_own_stage_satisfies_bibi_only_vacuously`).
/// What is settled here is only that no member can reach hand. Whether the
/// CONDITION passes is still an open ruling; see
/// `docs/PBLL02_ABILITY_REVIEW.md` §8.3 item 3.
#[test]
fn no_revealed_members_takes_nothing() {
    assert_eq!(
        taken_member(&[]),
        None,
        "no member was revealed, so there is nothing the clause could take"
    );
}

/// The parse: three branches, each carrying the ALL marker.
///
/// This asserted `vec![false, true, true]` — branch 1 losing the marker because
/// `all_members` was a field of the `Group` variant and branch 1 is a
/// `location_condition`. It was a faithful pin of a defect, kept visible on purpose.
/// The marker now lives on `ConditionCommon`, so all three carry it and the shapes
/// are consistent.
#[test]
fn every_branch_carries_the_all_marker() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(SPRING);
    let eff = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ成功時"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("ライブ成功時 ability has an effect")
    };
    let cond = eff.condition.as_deref().expect("the all-of-one-group gate");
    let branches: Vec<Box<rabuka_engine::card::Condition>> =
        cond.get_conditions().expect("three branches").to_vec();
    assert_eq!(branches.len(), 3, "Printemps / lily white / BiBi");
    let carried: Vec<bool> = branches
        .iter()
        .map(|b| b.get_all_members() == Some(true))
        .collect();
    assert_eq!(
        carried,
        vec![true, true, true],
        "「すべて」 must mean the same thing on all three branches. It used to be \
         dropped on the location_condition, which is what made branch 1 an ANY test"
    );
}

/// Prompt shape, which is what the old file misread as a count-dependent defect.
///
/// 「1枚」 is a choice whenever the reveal holds more than one member, and the engine
/// takes the only candidate without asking when it holds exactly one. Both halves
/// are asserted because the difference between them is the whole reason the old
/// helper looked like it had found an engine bug.
#[test]
fn one_candidate_is_taken_without_a_prompt_and_two_are_a_choice() {
    let run = |revealed: &[&str]| {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let f = game.id(FILLER);
        for _ in 0..20 {
            game.state.player1.main_deck.cards.push(f);
            game.state.player2.main_deck.cards.push(f);
        }
        game.state.player1.live_card_zone.cards.push(game.id(SPRING));
        for c in revealed {
            game.state.revealed_cards.push(game.id(c));
        }
        let mut h = BaseHeart { hearts: HeartMap::new() };
        h.hearts.insert(HeartColor::Heart00, 20);
        game.state.player1.stage_hearts = Some(h);
        game.state.current_phase = Phase::LiveVictoryDetermination;
        TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
        game.state.process_pending_auto_abilities("p1");
        game
    };

    let one = run(&[P]);
    assert!(
        !one.has_pending_choice(),
        "one qualifying member is the Exact path — taken with no prompt"
    );
    assert_eq!(
        one.state.player1.hand.cards.len(),
        1,
        "and it did reach hand"
    );

    let mut two = run(&[P, P]);
    assert!(
        two.has_pending_choice(),
        "「1枚」 out of two candidates is a real choice and must be offered"
    );
    two.drain_choices_strict(&["SelectCard"], &[0]);
    assert_eq!(
        two.state.player1.hand.cards.len(),
        1,
        "answering it takes exactly one"
    );
}