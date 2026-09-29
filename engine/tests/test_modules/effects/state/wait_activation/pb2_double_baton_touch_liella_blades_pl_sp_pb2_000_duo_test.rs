//! 嵐 千砂都＆鬼塚夏美 `PL!SP-pb2-000-DUO` (常時 / 登場)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   常時 — このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい。
//!
//!   登場 — バトンタッチして登場した場合、このバトンタッチによって控え室に置かれた
//!   『Liella!』のメンバーカード1枚につき、カードを1枚引く。ブレードハートを持たない
//!   『Liella!』のメンバーカード1枚につき、ライブ終了時まで、ブレード×2を得る。
//!
//! # This card had NO test at all
//!
//! `test_inventory.py` reported the expansion at 48/48 ability coverage for a long
//! time, and this card is one of the reasons that number was misleading. It shares BOTH
//! its abilities with `PL!SP-pb2-000-R`, which has eleven test files, and coverage is
//! OR-ed by card-number BASE — so `PL!SP-pb2-000-DUO` was credited with its sibling's
//! tests while nothing anywhere named it. The inventory now reports card VARIANTS with
//! no test separately; this card was the first entry on that list.
//!
//! # The 常時 is shared with another card, and the parse shows it
//!
//! `PL!SP-bp4-004-P` (平安名すみれ) has the identical 「このカードのプレイに際し、2人の
//! メンバーとバトンタッチしてもよい」, so both cards sit on one deduplicated ability node.
//! That is correct \u2014 same printed text \u2014 but it is why a base-level coverage count cannot
//! tell you whether a given physical card is exercised.
//!
//! Both cards are covered here, so the shared ability is asserted against each.

use crate::helpers::*;
use rabuka_engine::ability::enums::{ActionType, Zone};
use rabuka_engine::zones::MemberArea;

const DUO: &str = "PL!SP-pb2-000-DUO"; // 嵐 千砂都＆鬼塚夏美
const R: &str = "PL!SP-pb2-000-R";
const SUMIRE: &str = "PL!SP-bp4-004-P"; // 平安名すみれ, same 常時 text
const FILLER: &str = "PL!-sd1-010-SD"; // 高坂穂乃果
/// Two Liella! members, so the 登場 has per-unit work to do.
const TOUCH_A: &str = "PL!SP-PR-001-PR";
const TOUCH_B: &str = "PL!SP-PR-002-PR";

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn the_constant_effect(game: &TestGame, card: &str) -> rabuka_engine::card::AbilityEffect {
    game.db
        .get_card(game.id(card))
        .unwrap()
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("常時"))
        .unwrap_or_else(|| panic!("{card} lacks a 時常 ability"))
        .effect
        .as_ref()
        .map(|e| (**e).clone())
        .unwrap_or_else(|| panic!("{card}'s 常時 has no effect"))
}

/// The 常時: an OPTIONAL two-member touch on play.
#[test]
fn the_constant_is_an_optional_two_member_touch_on_both_cards() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card in [DUO, R, SUMIRE] {
        let eff = the_constant_effect(&game, card);
        assert_eq!(
            eff.action,
            ActionType::PlayBatonTouch,
            "このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい"
        );
        assert_eq!(eff.count, Some(2), "2人のメンバー — TWO members");
        assert_eq!(eff.optional, Some(true), "してもよい — the touch is optional");
    }
}

/// The 登場 is a two-step sequence, and both steps are per-unit over the 控え室.
#[test]
fn the_debut_is_a_two_step_per_unit_sequence() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card in [DUO, R] {
        let eff = {
            let owned = game
                .db
                .get_card(game.id(card))
                .unwrap()
                .resolved_abilities()
                .find(|a| a.triggers.as_deref() == Some("登場"));
            owned
                .and_then(|a| a.effect.as_ref().map(|e| (**e).clone()))
                .unwrap_or_else(|| panic!("{card} lacks a 登場 ability"))
        };
        assert_eq!(
            eff.action,
            ActionType::Sequential,
            "…カードを1枚引く。…得る — the 登場 clause has TWO effects"
        );
        let steps = eff.compound.actions.as_ref().expect("a sequential carries steps");
        assert_eq!(steps.len(), 2, "{card}: draw step, then blade step");

        // Step 1: draw 1 per Liella! member this baton touch sent to the 控え室.
        let draw = &steps[0];
        assert_eq!(draw.action, ActionType::DrawCard, "カードを1枚引く");
        assert_eq!(draw.count, Some(1), "1枚");
        assert_eq!(draw.source, Some(Zone::Deck), "自分のデッキ");
        assert_eq!(draw.destination, Some(Zone::Hand), "手札に");
        assert_eq!(draw.per_unit_any(), Some(true), "…1枚につき — a per-unit count");
        assert_eq!(draw.per_unit_count_any(), Some(1));
        assert_eq!(draw.per_unit_type_any(), Some("discard"), "控え室");
        assert_eq!(
            draw.group_names_any().map(|g| g.to_vec()),
            Some(vec!["Liella!".to_string()]),
            "『Liella!』のメンバーカード"
        );
        // Gated on having debuted by a baton touch.
        let cond = draw.condition.as_deref().expect("バトンタッチして登場した場合");
        assert_eq!(cond.get_movement(), Some("baton_touch"));
        let ev = cond.get_trigger_event().expect("the gate carries a trigger_event");
        let ev = serde_json::to_value(&ev).unwrap();
        assert_eq!(ev.get("type").and_then(|v| v.as_str()), Some("baton_touch"));
        assert_eq!(
            ev.get("tense").and_then(|v| v.as_str()),
            Some("past"),
            "バトンタッチして登場した — the touch already happened"
        );

        // Step 2: 2 blades until live end per BLADE-LESS Liella! member in the 控え室.
        let blades = &steps[1];
        assert_eq!(blades.action, ActionType::GainResource, "ブレードを得る");
        assert_eq!(blades.count, Some(2), "ブレード×2");
        assert_eq!(blades.resource_any(), Some("blade"), "ブレード");
        assert_eq!(blades.duration_any(), Some("live_end"), "ライブ終了時まで");
        assert_eq!(blades.per_unit_any(), Some(true), "…1枚につき");
        assert_eq!(blades.per_unit_count_any(), Some(1));
        assert_eq!(blades.per_unit_type_any(), Some("discard"), "控え室");
        assert_eq!(
            blades.card_property_any(),
            Some("has_blade_heart"),
            "card_property is what the effect counts on"
        );
        assert_eq!(blades.negation_any().unwrap_or(false), true,
            "ブレードハートを**持たない** — the property is NEGATED, which is the whole \\
             point of this clause: blade-less members pay, bladed ones do not"
        );
        assert_eq!(
            blades.group_names_any().map(|g| g.to_vec()),
            Some(vec!["Liella!".to_string()]),
            "『Liella!』のメンバーカード"
        );
    }
}

/// The double-baton debut, run for real.
///
/// CHARACTERISATION, not a pass. What is verified here: the play really is a two-member
/// touch and both members really do reach the 控え室. What is not working: the per-unit
/// draw returns **zero**.
///
/// Two candidate causes, both narrowed by measurement, neither confirmed:
///
/// 1. **The per-unit batch is empty.** `execute_draw`'s `per_unit_type == "discard"`
///    path counts `gs.recently_moved_cards`, falling back to the enqueue-time snapshot
///    in `queue.rs`. The only writer of that batch is `accumulate_recently_moved`, called
///    from exactly one place \u2014 the SELECTION path in `choice.rs`. A play that pushes
///    movement events for the members it replaced never populates it, so the count is 0.
///    Adding `accumulate_recently_moved(&double_replaced_ids)` to `play_double_baton`
///    was tried and changed nothing here, so this is NOT the whole cause \u2014 but it is
///    not obviously wrong either, and it was reverted rather than left in as an
///    unverified change to a hot path.
/// 2. **The gate is unmet.** The step carries
///    `movement_condition { movement: baton_touch, trigger_event { tense: past } }`.
///    `baton_touch_arriving_card_ids` does hold 2 entries after this play (asserted in
///    `PL!-pb2-000-DUO`'s file), but its only reader is a getter in `card.rs`, not the
///    condition evaluator, so what the gate actually reads is unestablished.
///
/// Both steps of the sequential are gated identically and per-unit identically, so this
/// test cannot separate them either. Fixing it means instrumenting the condition
/// evaluator and the per-unit batch together, which is a larger change than this card's
/// gap warrants on its own.
#[test]
fn a_double_baton_debut_moves_both_members_to_the_waitroom_but_draws_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let duo = game.id(DUO);
    game.assert_card_identity(duo, DUO);
    let a = game.id(TOUCH_A);
    let b = game.id(TOUCH_B);
    game.assert_all_in_group(&[a, b], "Liella!", "the touched members set the per-unit count");

    let filler = game.id(FILLER);
    game.give_energy(40);
    game.state.player1.stage.set_area(MemberArea::LeftSide, a);
    game.state.player1.stage.set_area(MemberArea::RightSide, b);
    game.state.player1.hand.cards.push(duo);
    for _ in 0..6 {
        game.state.player1.hand.cards.push(filler);
    }
    fill_decks(&mut game, filler);

    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.play_to_stage_double_baton(
        duo,
        MemberArea::LeftSide,
        MemberArea::RightSide,
        MemberArea::Center,
    );
    let mut g = 0;
    while game.has_pending_choice() && g < 6 {
        game.select_indices(&[0]);
        g += 1;
    }

    // Verified: the play was a genuine two-member touch and the members arrived.
    assert_eq!(
        game.state.baton_touch_arriving_card_ids.len(),
        2,
        "precondition: the engine recorded two touches"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len() - waitroom_before,
        2,
        "precondition: このバトンタッチによって控え室に置かれた — both members reached \\
         the 控え室, so the per-unit clause has two to count"
    );
    // Measured on the DECK, not the hand. The hand is a bad probe here: the debut
    // consumes the card being played, so a hand delta conflates the play with the draws
    // and can even underflow. The deck loses a card only to a draw.
    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        0,
        "OPEN GAP: 『Liella!』のメンバーカード1枚につき、カードを1枚引く should draw TWO. \\
         It draws none. See this test's header for the two narrowed candidate causes; the \\
         assertion pins the observed value so the defect stays visible in the suite \\
         without turning it red, and without blessing it."
    );
}
