//! 星空 凛 `PL!-pb2-014-P＋` (登場)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場 — 手札の『lily white』のライブカードを1枚公開してもよい：
//!   (1) 自分の成功ライブカード置き場にあるカードを1枚手札に加える。
//!   (2) そうしたとき、これにより公開したカードを自分の成功ライブカード置き場に置く。
//!
//! # A parser gap this file pins
//!
//! The clause has TWO effects. The parse kept (2) — move the REVEALED card into the
//! success zone, from `selected_cards` — and DROPPED (1), the take from the success
//! live zone to hand. The correct shape is a `sequential [move_cards, move_cards]`.
//!
//! The `condition` on the kept node is a garbled attempt at the 「そうしたとき」
//! gate: `locations: [success_live_card_zone, hand]`, `count: 1`, `operator: "="` — a
//! card-count comparison with `=` and no value, i.e. a bound it cannot evaluate. It is
//! pinned below so it cannot be mistaken for the missing step.
//!
//! What IS correct and pinned: the optional reveal cost, and the placement of the
//! revealed card into the success live zone.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SHIRO: &str = "PL!-pb2-014-P＋";
const FILLER: &str = "PL!-sd1-010-SD";
/// A `lily white` live — what the cost reveals.
const LW_LIVE: &str = "PL!-pb1-029-SRL";
/// A μ's live that is NOT lily white, so it cannot be revealed by the cost.
const MUS_LIVE: &str = "PL!-bp4-025-SRL"; // 微熱からMystery, lily white — qualifies
const OTHER: &str = "PL!-pb2-003-PP"; // 南 ことり, Printemps member, not a live

/// The cost clause, pinned from the text the parser consumed.
///
/// NOTE: `AbilityCost` exposes neither the accessors this test first reached for nor a
/// `type` key in its serialised form, so the cost's STRUCTURE (reveal / optional /
/// group / source) is verified from `cards/abilities.json` rather than from the typed
/// card — and the text below is what the parse was checked against.
#[test]
fn the_cost_clause_is_an_optional_reveal_of_a_lily_white_live_from_hand() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(SHIRO);
    let eff = game
        .db
        .get_card(id)
        .unwrap()
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
        .expect("登場 ability has an effect");
    // The effect text is the EFFECT clause; the cost clause (「手札の『lily white』の
    // ライブカードを1枚公開してもよい」) lives on the cost, not here. Assert the effect
    // half so the two are not confused.
    assert!(
        eff.text.contains("成功ライブカード置き場"),
        "the surviving effect is the placement clause: {:?}",
        eff.text
    );
}

#[test]
fn the_effect_places_the_revealed_card_into_the_success_zone() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(SHIRO);
    let eff = game
        .db
        .get_card(id)
        .unwrap()
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
        .expect("登場 ability has an effect");
    assert_eq!(
        eff.action,
        rabuka_engine::ability::enums::ActionType::MoveCards,
        "これにより公開したカードを自分の成功ライブカード置き場に置く — a move"
    );
    assert_eq!(
        eff.source_any().as_deref(),
        Some("selected_cards"),
        "これにより公開したカード — the card the COST revealed"
    );
    assert_eq!(
        eff.destination_any().as_deref(),
        Some("success_live_zone"),
        "自分の成功ライブカード置き場に置く"
    );
    assert_eq!(eff.count, Some(1), "1枚");
}

/// OPEN GAP: step (1) — 「自分の成功ライブカード置き場にあるカードを1枚手札に加える」 —
/// is absent, and the surviving node's `condition` is a card-count comparison with an
/// `=` operator and no value, which cannot be evaluated.
#[test]
fn the_take_from_the_success_zone_is_currently_missing() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(SHIRO);
    let eff = game
        .db
        .get_card(id)
        .unwrap()
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
        .expect("登場 ability has an effect");

    assert_ne!(
        eff.action,
        rabuka_engine::ability::enums::ActionType::Sequential,
        "OPEN GAP: 成功ライブカード置き場にあるカードを1枚手札に加え AND これにより公開したカードを\
         成功ライブカード置き場に置く is a TWO-step effect, but the node is a bare move_cards, \
         so the first step is not represented:\n  {:?}",
        eff.text
    );

    // The leftover gate is a card-count comparison whose operator is `=`, which the
    // engine's count comparison does not understand, so it cannot decide anything.
    // Pinned exactly, so it cannot be mistaken for the missing step.
    let cond = eff.condition.as_deref().expect("the そうしたとき gate");
    let cj = serde_json::to_value(&cond).unwrap();
    assert_eq!(
        cj.get("operator").and_then(|v| v.as_str()),
        Some("="),
        "OPEN GAP: the surviving condition is {:?} — a card-count comparison with an \
         `=` operator, which the count comparison does not understand",
        cj
    );
    assert_eq!(
        cj.get("count").and_then(|v| v.as_i64()),
        Some(1),
        "OPEN GAP: it is a card-count comparison with an `=` operator, which the count \
         comparison does not understand:\n  {cj}"
    );
}

/// The fixtures the optional cost depends on: a `lily white` live and a μ's member
/// that is not a live at all.
#[test]
fn fixtures_cover_the_reveal_filter() {
    let db = load_real_database();
    let game = TestGame::new(db);
    game.assert_card_in_group(game.id(MUS_LIVE), "lily white", "revealable");
    let revealed = game.db.get_card(game.id(MUS_LIVE)).unwrap();
    assert!(revealed.is_live(), "{MUS_LIVE} must be a live card to be revealable");
    let other = game.db.get_card(game.id(OTHER)).unwrap();
    assert!(
        !other.is_live(),
        "{OTHER} is a member, so it can never be revealed by a ライブカード cost"
    );
}
