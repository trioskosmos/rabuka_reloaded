//! 星空 凛&小泉花陽 `PL!-pb2-000-DUO` / `PL!-pb2-000-R` (常時 + 登場)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   常時 — このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい。
//!
//!   登場 — 『μ's』のメンバー2人からバトンタッチして登場した場合、
//!   (1) 自分の控え室にある『μ's』のライブカードを1枚手札に加え、
//!   (2) さらに、そのメンバー2人のコストの合計が15の場合、ライブ終了時まで、
//!       「常時 ライブの合計スコアを＋１する。」を得る。
//!
//! Two cards, one text — the tests run over both, since the parse is shared.
//!
//! # A parser gap this file used to pin, and no longer does
//!
//! The 登場 clause has TWO effects and the parse used to keep only (2). Clause (1) —
//! the waitroom recovery — was silently absent, and the old test here asserted that
//! absence, which is how a data loss got a test to bless it.
//!
//! The cause was not the effect rules. `_split_sentences_nesting` breaks only on 。,
//! and this ability separates every clause with 、, so it produced a single "sentence",
//! the さらに rule never saw two parts, and the first clause was absorbed into the
//! trailing 場合 condition. `_set_furthermore`/`_matches_furthermore` now split on a
//! top-level さらに itself, which is what the connective means and does not care
//! which punctuation separates the clauses.
//!
//! The debut is therefore a `sequential`, with the leading trigger gate hoisted to the
//! top level: 「…出現した場合、A、さらに、B」 is read as "if you debuted this way, do A
//! and B", so the gate scopes the whole ability, not just the first clause.

use crate::helpers::*;
use rabuka_engine::ability::enums::ActionType;
use rabuka_engine::zones::MemberArea;

const DUO: &str = "PL!-pb2-000-DUO";
const R: &str = "PL!-pb2-000-R";
const FILLER: &str = "PL!-sd1-010-SD";
/// A μ's live (series ラブライブ！) already in the 控え室, waiting to be recovered.
const MUS_LIVE: &str = "PL!-bp3-019-L"; // 僕らのLIVE
/// A non-μ's live in the same 控え室, which the 『μ's』 group filter must exclude.
const NON_MUS_LIVE: &str = "PL!-sd1-021-SD";
/// Two μ's members for the two-member touch the 登場 gate requires.
const MUS_MEMBER_1: &str = "PL!-PR-001-PR"; // 高坂穂乃果, Printemps
const MUS_MEMBER_2: &str = "PL!-PR-002-PR"; // 絢瀬絵里, BiBi

/// The 登場 effect, for `card`.
fn debut_effect(game: &TestGame, card: &str) -> rabuka_engine::card::AbilityEffect {
    game.db
        .get_card(game.id(card))
        .unwrap()
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .and_then(|a| a.effect.as_ref().map(|e| (**e).clone()))
        .expect("登場 ability has an effect")
}

/// The debut's two steps, in printed order.
fn debut_steps(game: &TestGame, card: &str) -> Vec<rabuka_engine::card::AbilityEffect> {
    let eff = debut_effect(game, card);
    assert_eq!(
        eff.action,
        ActionType::Sequential,
        "the 登場 clause has TWO effects, so it must be a sequence, not a bare gain_ability"
    );
    eff.compound_branch()
        .and_then(|c| c.actions.as_ref())
        .expect("a sequential carries its steps")
        .iter()
        .map(|b| (**b).clone())
        .collect()
}

#[test]
fn the_constant_is_an_optional_two_member_baton_touch_on_play() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card in [DUO, R] {
        let id = game.id(card);
        let card_ref = game.db.get_card(id).unwrap();
        let eff = card_ref
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("常時"))
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("常時 ability has an effect");
        assert_eq!(
            eff.action,
            ActionType::PlayBatonTouch,
            "このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい"
        );
        assert_eq!(eff.count, Some(2), "2人のメンバー");
        assert_eq!(eff.optional, Some(true), "してもよい — the touch is optional");
    }
}

/// Clause (1), the recovery. This is the step the old parse dropped entirely.
#[test]
fn the_debut_recovers_a_mus_live_card_from_the_waitroom() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card in [DUO, R] {
        let steps = debut_steps(&game, card);
        assert_eq!(steps.len(), 2, "two printed effects, two steps: {card}");

        let recover = &steps[0];
        assert_eq!(
            recover.action,
            ActionType::MoveCards,
            "自分の控え室にある『μ's』のライブカードを1枚手札に加え"
        );
        // This codebase spells 控え室 as the `discard` zone, and `zone_cards` maps
        // `Zone::Discard` and `Zone::Waitroom` to the SAME `player.waitroom.cards`
        // slice — so "discard" here means the waitroom, not a discard pile. Assert that
        // the source really reads the waitroom rather than pinning the spelling, which
        // a future rename would change without changing the mechanic.
        use rabuka_engine::ability::enums::Zone;
        let player = &game.state.player1;
        let source_name = recover.source.map(|z| z.to_str().to_string()).unwrap_or_default();
        let source = Zone::from_str(&source_name).expect("the recovery names a source zone");
        assert!(
            matches!(source, Zone::Discard | Zone::Waitroom),
            "控え室 is the waitroom, got {source:?}"
        );
        assert_eq!(
            rabuka_engine::ability::util::zone_cards(player, &source_name),
            player.waitroom.cards.as_slice(),
            "控え室 must resolve to the waitroom cards"
        );
        assert_eq!(
            recover.destination,
            Some(Zone::Hand),
            "手札に加え"
        );
        assert_eq!(recover.count, Some(1), "1枚");
        let groups: Vec<String> = recover
            .group_names_any()
            .map(|g| g.to_vec())
            .unwrap_or_default();
        assert_eq!(groups, vec!["μ's".to_string()], "『μ's』のライブカード");
        assert!(
            recover.card_type_any().is_some_and(|t| t.as_card_str() == "live_card"),
            "ライブカード, not any card in the waitroom"
        );
    }
}

/// Clause (2), the score gain, and the thing that makes the debut a sequence rather
/// than a single action.
#[test]
fn the_debut_gains_a_constant_live_total_score_plus_one() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card in [DUO, R] {
        let steps = debut_steps(&game, card);
        let gain = &steps[1];
        assert_eq!(
            gain.action,
            ActionType::GainAbility,
            "…得る — the debut GAINS an ability"
        );
        let gained = gain
            .gained_effect_any()
            .expect("the gained 常時 is structured, not just text");
        assert_eq!(gained.action, ActionType::ModifyScore, "ライブの合計スコアを＋１する");
        assert_eq!(
            gained.target_any().as_deref(),
            Some("live_total"),
            "ライブの合計スコア — a live total, not the card's own score"
        );
        assert_eq!(gained.value_any(), Some(1), "＋１する");
        assert_eq!(gained.duration_any().as_deref(), Some("live_end"), "ライブ終了時まで");

        // 「さらに、そのメンバー2人のコストの合計が15の場合」 gates only this step.
        let cond = gain
            .condition
            .as_deref()
            .expect("そのメンバー2人のコストの合計が15の場合 is the gate");
        assert_eq!(cond.get_comparison_type(), Some("cost"), "コストの合計");
        assert_eq!(cond.get_aggregate(), Some("total"), "合計");
        assert_eq!(cond.get_cost_total(), Some(15), "合計が15");
    }
}

/// The baton gate scopes the WHOLE debut, so it is hoisted onto the sequence rather
/// than left on the recovery step alone. Otherwise the score gain would fire for a
/// member that never debuted by a two-member baton touch.
#[test]
fn the_debut_is_gated_on_a_two_member_mus_baton_touch() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card in [DUO, R] {
        let eff = debut_effect(&game, card);
        let cond = eff
            .condition
            .as_deref()
            .expect("『μ's』のメンバー2人からバトンタッチして登場した場合 gates the debut");
        assert_eq!(
            cond.get_movement(),
            Some("baton_touch"),
            "the debut is gated on HAVING BEEN baton touched"
        );
        let ev = cond
            .get_trigger_event()
            .expect("the gate carries a trigger_event");
        // `TriggerEvent` is a typed struct, not a map.
        let ev_json = serde_json::to_value(&ev).unwrap();
        assert_eq!(
            ev_json.get("type").and_then(|v| v.as_str()),
            Some("baton_touch")
        );
        assert_eq!(
            ev_json.get("min_count").and_then(|v| v.as_i64()),
            Some(2),
            "メンバー2人 — TWO members, not one"
        );
        assert_eq!(
            ev_json.get("tense").and_then(|v| v.as_str()),
            Some("past"),
            "バトンタッチして登場した — the touch already happened"
        );
        let groups: Vec<String> = cond
            .get_group_names()
            .map(|g| g.to_vec())
            .unwrap_or_default();
        assert_eq!(groups, vec!["μ's".to_string()], "『μ's』のメンバー2人");
    }
}

// # The runtime trigger IS covered now
//
// This file previously said the recovery was verified as data only, and recorded the
// end-to-end behaviour as a known gap, because the 登場 gate is
// 『μ's〇のメンバー2人からバトンタッチして登場した場合 and
// `play_to_stage_baton` cannot build that fixture: the engine only takes the
// double-baton route when the action carries two area INDICES, and with one it places
// the card into an empty area and records no touch at all. The helper did not exist, so
// the gap was real — but it was a missing helper, not a missing feature.
// `play_to_stage_double_baton` in the test helpers closes it, and the runtime tests
// below now run the whole chain: two recorded touches, the recovery, and the cost-15
// gate correctly withholding the live-end score bonus.

/// The debut, run for real through a genuine two-member baton touch.
///
/// The 登場 gate is 「『μ's』のメンバー2人からバトンタッチして登場した場合」, so the fixture has
/// to touch exactly two μ's members. `play_to_stage_baton` cannot do that: the engine
/// only takes the double-baton route when the action carries two area INDICES, and with
/// one it simply places the card into an empty area, recording no touch at all. That is
/// why this file previously recorded the runtime behaviour as unverifiable — the helper
/// needed did not exist. `play_to_stage_double_baton` is it.
#[test]
fn the_recovery_moves_a_mus_live_out_of_the_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let duo = game.id(DUO);
    game.assert_card_identity(duo, DUO);

    let mus_live = game.id(MUS_LIVE);
    let other_live = game.id(NON_MUS_LIVE);
    game.assert_card_in_group(mus_live, "μ's", "the card the recovery must take");
    game.state.player1.waitroom.cards.push(mus_live);
    game.state.player1.waitroom.cards.push(other_live);

    let m1 = game.id(MUS_MEMBER_1);
    let m2 = game.id(MUS_MEMBER_2);
    game.assert_all_in_group(&[m1, m2], "μ's", "the two members the gate requires");

    let filler = game.id(FILLER);
    game.give_energy(40);
    game.state.player1.stage.set_area(MemberArea::LeftSide, m1);
    game.state.player1.stage.set_area(MemberArea::RightSide, m2);
    game.state.player1.hand.cards.push(duo);
    for _ in 0..4 {
        game.state.player1.hand.cards.push(filler);
    }
    fill_decks(&mut game, filler);

    game.play_to_stage_double_baton(
        duo,
        MemberArea::LeftSide,
        MemberArea::RightSide,
        MemberArea::Center,
    );

    // First, the fixture itself: the play really was a two-member touch, and the gate's
    // `min_count = 2` really was satisfied. Without this the rest could pass for the
    // wrong reason — the original attempt to write this test did exactly that.
    assert_eq!(
        game.state.baton_touch_arriving_card_ids.len(),
        2,
        "『μ's』のメンバー2人からバトンタッチして登場した場合 — the debut is gated on TWO \
         recorded touches, and only the double-baton path records two"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&m1)
            && !game.state.player1.stage.stage.contains(&m2),
        "both touched members must have left the stage"
    );
    let mut g = 0;
    while game.has_pending_choice() && g < 5 {
        game.select_indices(&[0]);
        g += 1;
    }

    // Then the printed clause: 自分の控え室にある『μ's』のライブカードを1枚手札に加え
    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "自分の控え室にある『μ's』のライブカードを1枚手札に加え — the μ's live must reach hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&other_live),
        "the group filter is 『μ's』 — a non-μ's live in the 控え室 must be left alone"
    );
}

/// The cost-15 half of the clause. 「そのメンバー2人のコストの合計が15の場合」 gates ONLY the
/// ライブ終了時まで gain, not the recovery, so this is the case that separates the two
/// steps in the sequence: 4 + 4 = 8, which is not 15, so the gained 常時 must not appear.
#[test]
fn the_live_end_score_bonus_needs_a_cost_total_of_fifteen() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let duo = game.id(DUO);
    let mus_live = game.id(MUS_LIVE);
    game.state.player1.waitroom.cards.push(mus_live);
    let m1 = game.id(MUS_MEMBER_1);
    let m2 = game.id(MUS_MEMBER_2);
    game.assert_all_in_group(&[m1, m2], "μ's", "fixture");

    let filler = game.id(FILLER);
    game.give_energy(40);
    game.state.player1.stage.set_area(MemberArea::LeftSide, m1);
    game.state.player1.stage.set_area(MemberArea::RightSide, m2);
    game.state.player1.hand.cards.push(duo);
    for _ in 0..4 {
        game.state.player1.hand.cards.push(filler);
    }
    fill_decks(&mut game, filler);

    game.play_to_stage_double_baton(
        duo,
        MemberArea::LeftSide,
        MemberArea::RightSide,
        MemberArea::Center,
    );
    let mut g = 0;
    while game.has_pending_choice() && g < 5 {
        game.select_indices(&[0]);
        g += 1;
    }

    let cost_total = game
        .db
        .get_card(m1)
        .and_then(|c| c.cost)
        .unwrap_or(0)
        + game.db.get_card(m2).and_then(|c| c.cost).unwrap_or(0);
    assert_eq!(
        cost_total, 8,
        "precondition: the two touched members cost 4 + 4, which is not the printed 15"
    );
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "そのメンバー2人のコストの合計が15の場合、ライブ終了時まで、\
         「常時 ライブの合計スコアを＋１する。」を得る — with a cost total of 8 the gated \
         half of the debut must not be granted, while the UNGATED recovery still runs"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "the recovery is not gated on the cost total — it still runs"
    );
}

/// The same debut on the other card, so the shared parse is exercised on both.
#[test]
fn the_single_card_debut_also_recovers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let r = game.id(R);
    game.assert_card_identity(r, R);
    let mus_live = game.id(MUS_LIVE);
    game.state.player1.waitroom.cards.push(mus_live);
    let m1 = game.id(MUS_MEMBER_1);
    let m2 = game.id(MUS_MEMBER_2);

    let filler = game.id(FILLER);
    game.give_energy(40);
    game.state.player1.stage.set_area(MemberArea::LeftSide, m1);
    game.state.player1.stage.set_area(MemberArea::RightSide, m2);
    game.state.player1.hand.cards.push(r);
    for _ in 0..4 {
        game.state.player1.hand.cards.push(filler);
    }
    fill_decks(&mut game, filler);

    game.play_to_stage_double_baton(
        r,
        MemberArea::LeftSide,
        MemberArea::RightSide,
        MemberArea::Center,
    );
    let mut g = 0;
    while game.has_pending_choice() && g < 5 {
        game.select_indices(&[0]);
        g += 1;
    }

    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "PL!-pb2-000-R carries the same 登場 text and the same parse, so the recovery runs"
    );
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}
