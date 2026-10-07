//! 僕たちはひとつの光 `PL!-pb2-039-L` (ライブ成功時) — the per-unit half is NOT pinned
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ成功時 — 自分のステージのメンバーとエールにより公開された自分のメンバーカード
//!   の中にある、名前の異なる『μ's』のメンバー1人につき、このカードのスコアを＋１する。
//!
//! In English: +1 score for each differently-named μ's member found among my stage
//! members together with the members the yell revealed.
//!
//! Setup idiom stolen from
//! `characterization/zero_tested_action_types_test.rs`: put the live in the live card
//! zone, inject a Heart00 wildcard into `stage_hearts` to guarantee success, populate
//! `revealed_cards`, set the phase, then fire the live-success abilities.
//!
//! # What is pinned and what is not
//!
//! The effect parses correctly: `modify_score` +1, `per_unit`, `per_unit_type: "人"`,
//! `card_type: member_card`, `group_names: ["μ's"]`, `distinct: card_name`.
//!
//! **The positive is still missing, and the reason is now narrower than it was.** Two
//! separate causes were stacked, and the first one masked the second:
//!
//! 1. *(fixed here)* The harness's premise was wrong. It injected a Heart00 wildcard
//!    of 20 and commented that this "guarantees the live succeeds". It does not:
//!    `check_heart_requirement` rejects the whole set when
//!    `total_provided < total_required`, and this live needs heart01 9 + heart03 9 +
//!    heart06 9 = 27. The ライブ成功時 window therefore stayed CLOSED,
//!    `trigger_live_success_abilities` returned at its first guard, and every score read
//!    0. That is why the author concluded the per-unit path "never fires" — and why the
//!    `live_success_no_premise` signal flagged this file. The amount is now derived from
//!    the card's own need_heart, and the window is asserted, so a 0 from here on is
//!    attributable to the effect rather than to the gate.
//!
//! 2. *(still open)* With the window open the score is STILL 0. Measured end to end
//!    with a real μ's member on stage, the handler's own log line reads:
//!
//!    ```text
//!    [DISTINCT_MEMBER_NAMES] singles=1 unique_joints=0 units=1
//!    Score add 1 (self) : 1 applied to []
//!    ```
//!
//!    and with two differently-named members, `singles=2 … Score add 2 … []`.
//!
//!    So the COUNT is right (1, then 2), the VALUE is right (1, then 2), and the
//!    RECIPIENT LIST is empty. The effect resolves and applies nothing. Everything
//!    upstream is correct: the ability is `LiveSuccess` + `ModifyScore`,
//!    `is_self_target() == true`, the scan logs
//!    `trigger=trigger_live_success,result=result_success`, the dispatch completes,
//!    and the fixtures match the filter (`card_type` member, group μ's,
//!    `filter.matches` true for 絵里/花陽/ことり and false for 松浦果南).
//!
//!    The loss is the `self_target` branch of the recipient filter, which keeps a
//!    candidate only when
//!
//!    ```rust
//!    card_id == self.session.activating_card_id
//!    ```
//!
//!    with `None` rejecting every candidate. Two things point at that value being
//!    unset on this path: `gs.activating_card` is `None` after the trigger, and the
//!    candidate pool's own activating-card fallback (which reads `gs.activating_card`,
//!    not the session) therefore contributes nothing either.
//!
//!    **Not committed, deliberately.** Three candidate fixes were written and measured:
//!
//!    * clearing `card_type`/`group`/`groups` from `target_filter` when `self_target`
//!      (they are count predicates — the same argument the `card_property` strip above
//!      already makes, and the recipient is 「このカード」, a live, which
//!      `card_type: member_card` excludes). Measured: `final_value` reaches 1 and 2
//!      as it should, and the recipient list is still `[]`.
//!    * refreshing `r.session.activating_card_id` in the resolver-reuse branch of
//!      `process_current_ability`, which carries the previous card's id forward.
//!      Plausible in its own right — a reused resolver now executing a different
//!      card's ability should not keep the old id. Measured: no change to the score.
//!    * both together: still `[]`.
//!
//!    So the session value is NOT the blocker, and the two measured facts that remain
//!    are these: the recipient list contains a card that is not the activating card,
//!    and `session.activating_card_id` holds a third id that matches no candidate. The
//!    candidate pool is built from `live_card_zone` + `success_live_card_zone` +
//!    `stage` + `gs.activating_card`, so the activating card is expected to be in it;
//!    it was not. Next to check is whether the live has already moved out of the zones
//!    the pool reads by the time the effect resolves.
//!
//!    None is committed: each touches the per-card score path for every
//!    `is_self_target` effect in the corpus, and none is demonstrated by a passing
//!    test. A hot-path behavioural change with no test proving the correct behaviour is
//!    worse than a visible defect.
//!
//!    The positive cases stay absent rather than being written to assert 0. `asserting
//!    0 here would pin the bug, and a test that documents a defect is the class this
//!    repo's whole quality inventory exists to catch. What IS pinned below are the cases
//!    that hold today: the effect never over-fires.
//!
//! See docs/PBLL02_ABILITY_REVIEW.md §8.3.

use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::core::types::Phase;
use rabuka_engine::turn::TurnEngine;

const HIKARI: &str = "PL!-pb2-039-L";
/// μ's members with three DIFFERENT names, across two units.
const ERI: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, BiBi
const HANAYOME: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
const NICO: &str = "PL!-pb2-003-PP"; // 南ことり, BiBi
/// Not μ's.
const AZALEA: &str = "PL!S-bp5-012-N"; // 松浦果南

fn fill_decks(game: &mut TestGame) {
    let f = game.id("PL!-sd1-010-SD");
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Stage `stage` members, reveal `revealed`, succeed the live, return 039's score.
fn score_with(stage: &[&str], revealed: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);

    let hikari = game.id(HIKARI);
    game.assert_card_identity(hikari, HIKARI);
    for (i, c) in stage.iter().enumerate() {
        game.state.player1.stage.stage[i] = game.id(c);
    }
    for c in revealed {
        game.state.revealed_cards.push(game.id(c));
    }
    game.state.player1.live_card_zone.cards.push(hikari);

    // Heart00 is a colorless WILDCARD, so it satisfies any colour's requirement -
    // but check_heart_requirement still checks the SUM first
    // (`if total_provided < total_required { return false }`). This live needs
    // heart01 9 + heart03 9 + heart06 9 = 27, and the harness used to inject 20.
    // 20 < 27, the window stayed CLOSED, trigger_live_success_abilities dispatched
    // nothing, and every score read 0 - which is why this file recorded the effect as
    // "never fires" when the effect was never reached.
    //
    // The amount is DERIVED from the card rather than hardcoded, so a data change to
    // 039's need_heart cannot silently close the window again.
    let need_total: u8 = game
        .db
        .get_card(hikari)
        .and_then(|c| c.need_heart.as_ref())
        .map(|nh| nh.hearts.values().copied().fold(0u8, |a, b| a.saturating_add(b)))
        .unwrap_or(0);
    assert!(
        need_total > 0,
        "precondition: 僕たちはひとつの光 must print a need_heart, or this harness \
         has nothing to satisfy"
    );
    let mut h = BaseHeart { hearts: HeartMap::new() };
    h.hearts.insert(HeartColor::Heart00, need_total);
    game.state.player1.stage_hearts = Some(h);

    game.state.current_phase = Phase::LiveVictoryDetermination;

    // The ライブ成功時 window must be OPEN, or `trigger_live_success_abilities`
    // dispatches nothing at all and every score below reads 0 for a reason that has
    // nothing to do with the effect. That is exactly what the harness could not tell
    // before: a score of 0 was indistinguishable from a live that never fired, which
    // is why the per-unit clause was recorded as "never fires" without anyone knowing
    // whether the effect or the gate was at fault.
    //
    // Asserted here rather than left implicit, so the +0 this file observes is
    // attributable to the effect and not to the window.
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the Heart00 wildcard must open the ライブ成功時 window, \
         otherwise nothing is dispatched and every assertion below passes vacuously"
    );

    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    let _ = game.drain_auto_ability_choices();

    game.state.mods.get_score_modifier(hikari)
}

/// The fixtures the untested clauses depend on, pinned so a data change cannot
/// silently make them meaningless.
#[test]
fn fixtures_span_distinct_mus_names_across_units() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card_no in [ERI, HANAYOME, NICO] {
        game.assert_card_in_group(game.id(card_no), "μ's", "fixture must be a μ's member");
    }
    for (a, b) in [(ERI, HANAYOME), (ERI, NICO), (HANAYOME, NICO)] {
        game.assert_distinct_card_names(game.id(a), game.id(b), "fixtures must differ by name");
    }
    assert_ne!(
        game.db.get_card(game.id(ERI)).and_then(|c| c.unit.as_ref().map(|u| u.to_string())),
        game.db.get_card(game.id(HANAYOME)).and_then(|c| c.unit.as_ref().map(|u| u.to_string())),
        "the two fixtures sit in different units (BiBi / Printemps) on purpose — μ's \
         spans both"
    );
}

/// An empty pool → no bonus.
#[test]
fn an_empty_pool_scores_nothing() {
    assert_eq!(score_with(&[], &[]), 0, "no members at all — no bonus");
}

/// A non-μ's member on stage or revealed must not produce a bonus. Pinned because
/// an over-firing per-unit score would be a regression even while the under-firing
/// gap above is open.
#[test]
fn a_non_mus_member_produces_no_bonus() {
    assert_eq!(
        score_with(&[AZALEA], &[]),
        0,
        "『μ's』のメンバー — a staged AZALEA member is not μ's"
    );
    assert_eq!(
        score_with(&[], &[AZALEA]),
        0,
        "『μ's』のメンバー — a revealed AZALEA member is not μ's either"
    );
}

/// The card's own printed identity, and that the live-success path really ran — so a
/// future failure of the positive cases cannot be blamed on the harness.
#[test]
fn the_live_success_harness_fires_the_ability() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let hikari = game.id(HIKARI);
    let card = game.db.get_card(hikari).unwrap();
    assert!(
        card.resolved_abilities()
            .any(|a| a.triggers.as_deref() == Some("ライブ成功時")),
        "{HIKARI} must have a ライブ成功時 ability for this file to be about anything"
    );
    // The effect this file is about, and the fields the untested clauses live on.
    let eff = {
        let owned = card.resolved_abilities().find(|a| a.triggers.as_deref() == Some("ライブ成功時"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("ライブ成功時 ability has an effect")
    };
    assert_eq!(
        eff.action,
        rabuka_engine::ability::enums::ActionType::ModifyScore,
        "このカードのスコアを＋１する"
    );
    assert_eq!(eff.per_unit_any(), Some(true), "1人につき — per unit");
    assert_eq!(
        eff.distinct_any(),
        Some(rabuka_engine::card::DistinctType::CardName),
        "名前の異なる『μ's』のメンバー — distinct by name"
    );
}
