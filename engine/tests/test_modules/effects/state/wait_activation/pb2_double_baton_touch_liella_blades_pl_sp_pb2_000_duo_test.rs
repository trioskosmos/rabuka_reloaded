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
//! That is correct — same printed text — but it is why a base-level coverage count cannot
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
/// Two 『Liella!』 MEMBERS for the two-member touch.
///
/// These were `PL!SP-PR-001-PR` and `PL!SP-PR-002-PR`, which look right and are not:
/// both are ENERGY cards (`type=エネルギー`, `unit=None`). They carry the
/// ラブライブ！スーパースター!! series, so `assert_all_in_group(..., "Liella!")` passed —
/// a group assertion cannot see the wrong card TYPE, and the printed clause is
/// 「『Liella!』のメンバーカード」. The per-unit count came out 0, which was recorded as an
/// open engine defect for a long time.
const TOUCH_A: &str = "PL!SP-PR-005-PR"; // 嵐 千砂都, 5yncri5e!
const TOUCH_B: &str = "PL!SP-PR-003-PR"; // 澁谷かのん, CatChu!

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
        let steps = eff.compound_branch().and_then(|c| c.actions.as_ref()).expect("a sequential carries steps");
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

/// The double-baton debut, run for real. It draws TWO, which is the printed value.
///
/// This test spent a long time asserting the opposite and calling it an open engine
/// defect, with two "narrowed candidate causes" written up. **Both candidate causes
/// were wrong and there is no engine defect.** The per-unit machinery was correct
/// throughout; the fixture was two ENERGY cards where the clause names member cards.
///
/// `PL!SP-PR-001-PR` / `PL!SP-PR-002-PR` carry the ラブライブ！スーパースター!! series,
/// so `assert_all_in_group(..., "Liella!")` passed — a group assertion cannot see the
/// wrong card TYPE. The clause is 「『Liella!』の**メンバーカード**」, the per-unit filter
/// is `member_card` + `Liella!`, and two energy cards fail the first half. `count_matching`
/// returned 0, correctly.
///
/// What the two dead-end investigations did establish, and it is worth keeping:
///
/// * The gate is **not** the problem. `evaluate_baton_touch_condition` was
///   instrumented and reaches the end without an early return:
///   `bt_count=2`, `baton_touch_replaced_member_id` present and in the owner's 控え室.
/// * The tracked batch is **not** empty. `execute_draw` reports
///   `tracked=Some([.., ..])` with both displaced cards in it. The speculation that
///   `recently_moved_cards` is only populated by the selection path was wrong for this
///   play path — the batch arrives — and it was the reason a speculative
///   `accumulate_recently_moved` call was written and then reverted.
///
/// So the lesson is not about the engine. It is that a fixture which fails a *clause*
/// has to be checked against every noun in that clause: the group, the card TYPE, the
/// count, the unit of measure. I asserted the group and the count, and the type was
/// wrong the whole time.
#[test]
fn a_double_baton_debut_draws_one_per_member_it_displaced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let duo = game.id(DUO);
    game.assert_card_identity(duo, DUO);
    let a = game.id(TOUCH_A);
    let b = game.id(TOUCH_B);
    game.assert_all_in_group(&[a, b], "Liella!", "the touched members set the per-unit count");
    // The group assertion ALONE is not enough here, and that is the point. An energy
    // card in the Liella! series satisfies it, while the clause wants
    // 「『Liella!』のメンバーカード」. The previous fixture was two energy cards and every
    // group check passed.
    for id in [a, b] {
        assert_eq!(
            game.db.get_card(id).unwrap().card_type,
            rabuka_engine::card::CardType::Member,
            "『Liella!』のメンバーカード — the touched cards must be MEMBERS, whatever their \
             series says; an energy card in the same series passes the group check and \
             fails the clause"
        );
    }

    let filler = game.id(FILLER);
    game.give_energy(40);
    game.state.player1.stage.set_area(MemberArea::LeftSide, a);
    game.state.player1.stage.set_area(MemberArea::RightSide, b);
    game.state.player1.hand.cards.push(duo);
    for _ in 0..6 {
        game.state.player1.hand.cards.push(filler);
    }
    fill_decks(&mut game, filler);

    // Measured on the DECK, not the hand. The debut consumes the card being played, so a
    // hand delta conflates the play with the draws and can even underflow. The deck
    // loses a card only to a draw.
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

    assert_eq!(
        game.state.baton_touch_arriving_card_ids.len(),
        2,
        "precondition: the engine recorded two touches"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len() - waitroom_before,
        2,
        "このバトンタッチによって控え室に置かれた — both members reached the 控え室, so the \\
         per-unit clause has two to count"
    );
    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "このバトンタッチによって控え室に置かれた『Liella!』のメンバーカード1枚につき、\
         カードを1枚引く — one draw per displaced Liella! member card, so TWO"
    );
}

/// The second half of the same clause, which is the one that reads 「持たない」.
///
/// 「ブレードハートを持たない『Liella!』のメンバーカード1枚につき、ライブ終了時まで、
/// ブレード×2を得る」. The property is NEGATED, and the two displaced members are
/// blade-less members, so both steps of the sequence fire on the same play.
#[test]
fn the_blade_half_also_reads_the_displaced_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let duo = game.id(DUO);
    let a = game.id(TOUCH_A);
    let b = game.id(TOUCH_B);
    for id in [a, b] {
        assert_eq!(
            game.db.get_card(id).unwrap().card_type,
            rabuka_engine::card::CardType::Member
        );
    }

    let filler = game.id(FILLER);
    game.give_energy(40);
    game.state.player1.stage.set_area(MemberArea::LeftSide, a);
    game.state.player1.stage.set_area(MemberArea::RightSide, b);
    game.state.player1.hand.cards.push(duo);
    for _ in 0..6 {
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
    while game.has_pending_choice() && g < 6 {
        game.select_indices(&[0]);
        g += 1;
    }

    // 「持たない」 — the two members are members with no blade heart, so the step's
    // `negation: true` on `card_property: has_blade_heart` is satisfied for both.
    let blade_hearted = game
        .db
        .get_card(a)
        .and_then(|c| c.blade_heart.as_ref().map(|h| !h.hearts.is_empty()))
        .unwrap_or(false);
    assert!(
        !blade_hearted,
        "precondition: 『Liella!〇のメンバーカード in the 控え室 must be BLADE-LESS for \
         「ブレードハートを持たない」 to pay out"
    );
}

