//! 星空 凛 `PL!-pb2-014-P＋` (登場)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場 — 手札の『lily white』のライブカードを1枚公開してもよい：
//!   (1) 自分の成功ライブカード置き場にあるカードを1枚手札に加える。
//!   (2) そうしたとき、これにより公開したカードを自分の成功ライブカード置き場に置く。
//!
//! # The parser dropped a leg until 2026-10-04
//!
//! The effect has TWO steps and the parse kept only the second — move the REVEALED card
//! into the success zone, from `selected_cards` — and dropped the first, the take from the
//! success live zone to hand. Worse, the 「そうしたとき」 gate was emitted as
//! `{ locations: [success_live_card_zone, hand], count: 1, operator: "=" }`: a card-count
//! comparison with `=` and no value, which decides nothing. So the card emptied the
//! success zone and put nothing into hand.
//!
//! The fix is `_try_two_step_zone_move`, a `sequential [take, place]` with the second leg
//! gated on `source: "preceding_moved"`. This is **not** the 「そうしたとき」 family the
//! parser already had: every rule there splits 「<optional>そうしたとき、<consequence>」
//! into a `conditional_on_optional`, and here the optionality belongs to the COST, so both
//! legs are mandatory and the marker only means "if leg (1) happened".
//!
//! What was already correct and is still pinned: the optional reveal cost.

use crate::helpers::*;

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
    // Leg (2) of the two-step effect — the placement. Leg (1) is pinned by
    // `the_take_from_the_success_zone_is_the_first_leg_and_the_placement_the_second`.
    let place = eff
        .compound_branch()
        .and_then(|c| c.actions.as_ref())
        .expect("a sequential's legs live in compound.actions")[1]
        .as_ref()
        .clone();
    assert_eq!(
        place.action,
        rabuka_engine::ability::enums::ActionType::MoveCards,
        "これにより公開したカードを自分の成功ライブカード置き場に置く — a move"
    );
    assert_eq!(
        place.source_any().as_deref(),
        Some("selected_cards"),
        "これにより公開したカード — the card the COST revealed"
    );
    assert_eq!(
        place.destination_any().as_deref(),
        Some("success_live_zone"),
        "自分の成功ライブカード置き場に置く"
    );
    assert_eq!(place.count, Some(1), "1枚");
}

/// Both legs are present, in order, and the second is gated on the first.
///
/// This was an OPEN GAP asserted the wrong way round — `assert_ne!(Sequential)` with an
/// "OPEN GAP" message, pinning the partial parse so it could not regress into being fixed.
/// The parse now builds `sequential [ take, place ]`:
///
///   (1) 自分の成功ライブカード置き場にあるカードを1枚手札に加える
///   (2) そうしたとき、これにより公開したカードを自分の成功ライブカード置き場に置く
///
/// 「そうしたとき」 is NOT the 「してもよい」 conditional family here — the optional part
/// belongs to the COST (the reveal), so both legs are mandatory and the marker only means
/// "if leg (1) happened". It is expressed as `source: "preceding_moved"`, the engine's
/// existing spelling for a preceding-step result.
#[test]
fn the_take_from_the_success_zone_is_the_first_leg_and_the_placement_the_second() {
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
        rabuka_engine::ability::enums::ActionType::Sequential,
        "a take AND a place is a TWO-step effect. The parse used to keep only the place \
         (: {:?})",
        eff.text
    );
    let legs = eff
        .compound_branch()
        .and_then(|c| c.actions.as_ref())
        .expect("a sequential's legs live in compound.actions");
    assert_eq!(legs.len(), 2, "(1) take from the success zone, (2) place into it");

    let take = legs[0].as_ref();
    assert_eq!(take.action, rabuka_engine::ability::enums::ActionType::MoveCards);
    assert_eq!(
        take.source_any().as_deref(),
        Some("success_live_zone"),
        "(1) 自分の成功ライブカード置き場にあるカードを1枚手札に加える — FROM the success zone"
    );
    assert_eq!(take.destination_any().as_deref(), Some("hand"), "…を手札に加える");
    assert_eq!(
        take.condition, None,
        "(1) is unconditional: the printed optionality is the COST's reveal, not this leg"
    );

    let place = legs[1].as_ref();
    assert_eq!(place.action, rabuka_engine::ability::enums::ActionType::MoveCards);
    assert_eq!(
        place.source_any().as_deref(),
        Some("selected_cards"),
        "(2) これにより公開したカード — the card the COST revealed"
    );
    assert_eq!(
        place.destination_any().as_deref(),
        Some("success_live_zone"),
        "自分の成功ライブカード置き場に置く"
    );

    // 「そうしたとき」 = "if (1) moved something".
    let gate = place.condition.as_deref().expect("(2) is gated on (1)");
    assert_eq!(
        gate.get_source(),
        Some("preceding_moved"),
        "そうしたとき is a preceding-step result, not a zone and not a property"
    );
    assert_eq!(
        (gate.get_count(), gate.get_operator()),
        (Some(1), Some(">=")),
        "at least one card moved. The old parse emitted `operator: \"=\"` with no value — a \
         comparison that could not decide anything — which is how the dropped leg went \
         unnoticed"
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
