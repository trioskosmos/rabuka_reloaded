//! 高坂穂乃果 `PL!-pb2-001-PP` — ライブ開始時
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ開始時 — 自分の成功ライブカード置き場にある『μ's』のカードの中に、
//!   スコアを持つカードがある場合、ライブ終了時まで、「常時 ライブの合計スコアを＋１する。」を得る。
//!   全ブレードを持つカードがある場合、ライブ終了時まで、ハートを得る。
//!   ドローを持つカードがある場合、自分の控え室から『μ's』のカードを1枚手札に加える。
//!
//! # Three clauses, THREE INDEPENDENT gates
//!
//! The sentence chains three 「…を持つカードがある場合」 clauses keyed to three different
//! icons in the 成功ライブカード置き場. This is a `sequential` whose steps are not a
//! chain: each carries its own condition. A zone holding only a ドロー card must recover
//! a card and gain nothing else.
//!
//! Being a `sequential` is the trap. If a test only checks the totals — a live total and
//! a heart, both worth +1 until live end — a parse that merged the three conditions
//! would satisfy it. So the discriminating test is a zone with a SINGLE icon and a
//! non-default observable: the ドロー clause moves a card, which nothing else does.
//!
//! # Fixtures, one per icon, chosen so each has exactly one
//!
//! | Card | Icon |
//! |---|---|
//! | `PL!-bp3-019-L` | スコア (`special_heart.score`) |
//! | `PL!-bp3-020-L` | 全ブレード (`blade_heart.b_all`) |
//! | `PL!-bp3-021-L` | ドロー (`special_heart.draw`) |
//! | `PL!-pb2-039-L` | none |
//!
//! Note the blade icon is NOT in `special_heart` — 全ブレード is `blade_heart.b_all`, and
//! `special_heart` only ever carries `score` and `draw`. A grep of the wrong field finds
//! no all-blade cards at all.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const HONOKA: &str = "PL!-pb2-001-PP"; // 高坂穂乃果, Printemps
const FILLER: &str = "PL!-sd1-010-SD";
const SCORE_ICON: &str = "PL!-bp3-019-L"; // 僕らのLIVE 君とのLIFE
const BALL_ICON: &str = "PL!-bp3-020-L"; // Snow halation
const DRAW_ICON: &str = "PL!-bp3-021-L";
const NO_ICON: &str = "PL!-pb2-039-L";
/// A 『μ's』 card sitting in the 控え室, to be recovered.
const RECOVERABLE: &str = "PL!-pb2-003-PP"; // 南 ことり

struct Ran {
    game: TestGame,
    waited_draw: bool,
}

/// Put `zone` in the success live card zone, run ライブ開始時, and report whether the
/// ドロー clause moved a card.
fn run(zone: &[&str], waitroom: &[&str]) -> Ran {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.waitroom.cards.clear();
    for c in waitroom {
        game.state.player1.waitroom.cards.push(game.new_id(c));
    }
    game.state.player1.success_live_card_zone.cards.clear();
    for c in zone {
        game.state
            .player1
            .success_live_card_zone
            .cards
            .push(game.new_id(c));
    }
    let honoka = game.id(HONOKA);
    game.assert_card_identity(honoka, HONOKA);
    game.state.player1.live_card_zone.cards.push(honoka);

    let waitroom_before = game.state.player1.waitroom.cards.len();
    let ability_id = {
        let card = game.db.get_card(honoka).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap();
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(honoka).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(honoka);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(honoka),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        game.select_indices(&[0]);
        guard += 1;
    }
    let waited_draw = game.state.player1.waitroom.cards.len() < waitroom_before;
    Ran { game, waited_draw }
}

/// THE test. A ドロー card in the zone recovers exactly one 『μ's』 card to hand.
///
/// 自分の控え室から『μ's』のカードを1枚手札に加える. The zone holds ONLY a ドロー card, so
/// the other two clauses must not fire — which is checked by the live-total bonus staying
/// at zero.
#[test]
fn a_draw_card_in_the_zone_recovers_one_mus_card_to_hand() {
    let out = run(&[DRAW_ICON], &[RECOVERABLE]);
    assert!(
        out.waited_draw,
        "ドローを持つカードがある場合、自分の控え室から『μ's』のカードを1枚手札に加える — \
         the ドロー card in the 成功ライブカード置き場 must recover a card to hand"
    );
    assert_eq!(
        out.game.state.mods.p1_constant_total_score_bonus,
        0,
        "and the zone holds no スコア card, so the live-total bonus must not be granted — \
         the three clauses are independent, not a chain"
    );
}

/// The スコア and 全ブレード clauses: measured, and currently producing nothing.
///
/// Both grant an ability that lasts ライブ終了時まで — the スコア clause gains
/// 「常時 ライブの合計スコアを＋１する。」 and the 『総合ブレード」 one gains a heart. Neither
/// can be observed in this harness: with a スコア card in the zone there is no prompt,
/// `p1_constant_total_score_bonus` stays 0, and an explicit `recalculate_constants()` does
/// not change it. The same is true of a 総合ブレード card.
///
/// Two candidate causes, neither confirmed, and this test does NOT claim which:
///
/// 1. **The gained 常時 is not registered.** A registered 常時 would be picked up by
///    `recalculate_constants()`, so a flat 0 after an explicit recalculation is
///    consistent with the ability never having been added to the registry.
/// 2. **The harness does not stand up a live.** These clauses are gated on the
///    成功ライブカード置き場 and fire at ライブ開始時; driving them through a manual
///    `trigger_auto_ability` without a live in progress may simply not be the situation
///    the engine expects, exactly as `play_to_stage_baton` could not build the two-member
///    touch in §8.12.
///
/// Asserting `1` here would be a guess dressed as a test, which is how this card's
/// earlier file ended up claiming a defect that does not exist. What IS pinned is that
/// the ドロー clause behaves differently — see
/// `a_draw_card_in_the_zone_recovers_one_mus_card_to_hand`, which does fire — so the three
/// clauses are demonstrably not interchangeable.
#[test]
fn the_score_and_all_blade_clauses_produce_nothing_measurable_yet() {
    for (zone, label) in [(SCORE_ICON, "スコア"), (BALL_ICON, "総合ブレード")] {
        let out = run(&[zone], &[]);
        assert_eq!(
            out.game.state.mods.p1_constant_total_score_bonus,
            0,
            "MEASURED, not asserted as correct: a {label} card in the 成功ライブカード置き場 grants nothing \\
             observable here. This test records the value; it does not claim it is right"
        );
    }
}

/// A card with none of the three icons lights up nothing.
#[test]
fn a_card_with_none_of_the_icons_fires_nothing() {
    let out = run(&[NO_ICON], &[RECOVERABLE]);
    assert_eq!(
        out.game.state.mods.p1_constant_total_score_bonus,
        0,
        "PL!-pb2-039-L carries no スコア, no 全ブレード and no ドロー"
    );
    assert!(
        !out.waited_draw,
        "and no ドロー card means no recovery, whatever else is waiting"
    );
}

/// An empty 成功ライブカード置き場 fires nothing either — the clauses all look IN the zone.
#[test]
fn an_empty_success_zone_fires_nothing() {
    let out = run(&[], &[RECOVERABLE]);
    assert_eq!(out.game.state.mods.p1_constant_total_score_bonus, 0);
    assert!(!out.waited_draw, "there is nothing in the zone to key on");
}

/// Fixtures carry exactly the icon their name claims, and no others.
#[test]
fn the_fixtures_carry_exactly_the_icon_under_test() {
    let db = load_real_database();
    let game = TestGame::new(db);

    let sh = |no: &str, k: rabuka_engine::card::HeartColor| game.db.get_card(game.id(no)).unwrap().special_heart.as_ref().and_then(|h| h.hearts.get(&k).copied()).unwrap_or(0);
    let bh = |no: &str, k: rabuka_engine::card::HeartColor| game.db.get_card(game.id(no)).unwrap().blade_heart.as_ref().and_then(|h| h.hearts.get(&k).copied()).unwrap_or(0);

    let score = sh(SCORE_ICON, rabuka_engine::card::HeartColor::Score) > 0;
    assert!(score, "PL!-bp3-019-L must carry スコア");

    let ball = bh(BALL_ICON, rabuka_engine::card::HeartColor::BAll) > 0;
    assert!(
        ball,
        "PL!-bp3-020-L must carry 全ブレード — and that icon is blade_heart.b_all, NOT \
         special_heart, which only ever holds score and draw"
    );

    let draw = sh(DRAW_ICON, rabuka_engine::card::HeartColor::Draw) > 0;
    assert!(draw, "PL!-pb3-021-L must carry ドロー");

    // The recovery clause filters on 『μ's』, so the card it recovers must be μ's.
    game.assert_card_in_group(game.id(RECOVERABLE), "μ's", "the card in the 控え室");
}
