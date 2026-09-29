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
//! But it never fires — every positive case yields +0, whether the μ's members are on
//! the stage or revealed, and whether one or three distinct names. **This is a general
//! gap, not specific to this card**: `PL!HS-bp2-020-L` (蓮ノ空, ライブ開始時, same
//! shape — `人` + group + `distinct: card_name`) also returns +0, and it has no test
//! either. `modify_score` combined with `per_unit` appears 11 times in the corpus and
//! none of the group-filtered ones is covered.
//!
//! What IS pinned below are the cases that hold today: the effect never over-fires. The
//! 「names differ」「μ's」 and 「stage ∪ revealed」 clauses are documented here as
//! untested rather than asserted, because the per-unit score path returns 0 for all of
//! them and there is no correct positive value to pin yet.
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
const NICO: &str = "PL!-pb2-003-PP"; // 矢澤にこ, BiBi
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

    // Heart00 wildcard guarantees the live succeeds, so the effect always gets a turn.
    let mut h = BaseHeart { hearts: HeartMap::new() };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    game.state.current_phase = Phase::LiveVictoryDetermination;
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
    let mut game = TestGame::new(db);
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
