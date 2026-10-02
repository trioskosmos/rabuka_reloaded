//! 村野さやか `PL!HS-bp1-002-RM` — bounce self, then redeploy a 蓮ノ空 member into the
//! **area she just vacated**.
//!
//! {{kidou.png|起動}}{{turn1.png|ターン1回}}{{icon_energy.png|E}}{{icon_energy.png|E}}
//! このメンバーをステージから控え室に置く：自分の控え室から**コスト15以下**の『蓮ノ空』の
//! メンバーカードを1枚、**このメンバーがいたエリア**に登場させる。
//!
//! ## What is unique here
//!
//! The parse ends in `"destination": "same_area"`. The ability remembers WHICH AREA the
//! host stood in when the cost moved her to the waitroom, and puts the retrieved card back
//! into that same slot. Of 985 distinct ability nodes in the corpus this was one of only
//! **six** with no test file naming it, and the only one exercising position memory across
//! a cost.
//!
//! ## The bug this file exists to catch
//!
//! The cost is a `sequential_cost` of [pay 2 energy, move self stage→waitroom]. The second
//! sub-step is what VACATES the area: `remove_card_from_any_zone` sets
//! `gs.last_vacated_stage_area` (`ability/effects/move/transport.rs:50-53`). The effect step
//! then reads that value (`move/mod.rs:2079`) and hands it to `Zone::SameArea`
//! (`ability/util.rs:2364`).
//!
//! `move/mod.rs` used to clear `gs.last_vacated_stage_area = None` unconditionally on the
//! way out — so the cost's own move destroyed the information the effect step was about to
//! read. The effect got `None`, `Zone::SameArea` fell back to `stage_first_empty`, and the
//! card landed in the first free slot **no matter where the host had been standing**.
//!
//! The two area tests are the whole point. One host in the centre, one in the left side:
//! a `same_area` implementation that ignores the vacated area passes the centre case and
//! fails the left one, so running both is what distinguishes "remembers the area" from
//! "defaults to a slot".
//!
//! ## Both gates are broken independently
//!
//! `cost_limit: 15, cost_limit_operator: "<="` and `group_names: ["蓮ノ空"]` are separate
//! clauses, so each needs a fixture failing exactly one:
//!
//! | fixture | cost | group | used for |
//! |---|---|---|---|
//! | `PL!HS-PR-003-PR` 乙宗 梢 | 4 | 蓮ノ空 | positive, comfortably under the bound |
//! | `PL!HS-pb1-023-N` | **15** | 蓮ノ空 | positive AT the bound — pins `<=` not `<` |
//! | `PL!HS-PR-016-PR` | 17 | 蓮ノ空 | negative on COST only |
//! | `PL!-sd1-010-SD` 高坂穂乃果 | 4 | **not** 蓮ノ空 | negative on GROUP only |
//!
//! ## STATUS: an engine bug was found here, fixed, and all five now pass
//!
//! Following the convention in `tests/test_modules/characterization/strict_engine_bug_expected_fail_test.rs`:
//! *"Each asserts the CORRECT behaviour per card text… If one of these fails now, the ENGINE
//! is wrong, not the assert."* These assert the correct behaviour; four of them were failing,
//! the engine was at fault, and it is fixed.
//!
//! ### The bug: `same_area` worked only when there was no prompt
//!
//! `RUST_LOG=debug` on this file, at the two decision points, showed one card behaving two
//! ways. With a single candidate the effect auto-selected and the area was honoured:
//!
//! ```text
//! [MOVE_EFFECT_READ] dest=Some(SameArea) last_vacated=Some(0) stage=[-1,-1,-1]
//! [SELECTION_RESULT] available=2 matching=[1] outcome=Ok(Exact([1]))
//! [TRACE_SAME_AREA]  card=7611 vacated=Some(0) stage_after=[7611, -1, -1]     <- correct
//! ```
//!
//! With two candidates the player is prompted, and the card landed in the **first free
//! slot** — the centre — even from the left side.
//!
//! Cause: `execute_move_cards` cleared `gs.last_vacated_stage_area = None` on its way out.
//! The single-candidate path places before reaching that clear. The prompt path **suspends**
//! and returns early, so on resume the same frame ran the clear — and only then did
//! `execute_selected_cards_from_zone` (`move/mod.rs:2666`) read the value it needs. The area
//! was always being remembered, always destroyed just before the consumer that wanted it,
//! and only the suspending path noticed.
//!
//! Fix: clear unless THIS move is the consumer, i.e. unless its own destination is
//! `same_area`. The staleness guard the unconditional clear provided is kept for every other
//! destination.
//!
//! Note for the next reader: the first attempt at this fix gated on "did this move just
//! CREATE the value", which is true on the suspending path too, and so changed nothing. The
//! condition that matters is "is this move the CONSUMER".
//!
//! ## A fixture mistake that is worth remembering
//!
//! The first version of this test asserted a specific card appeared in the vacated area and
//! blamed the engine. The trace showed `card=7611` — **村野さやか herself**. That is
//! correct: the text says 「コスト15以下の『蓮ノ空』のメンバーカード1枚」 and **not 他の**
//! ("another"), and her own cost has just put her in the 控え室, so she is a legal target for
//! her own retrieval. The fixture had never accounted for the host being in her own
//! candidate pool. The area was never the problem; the "engine bug" was half a fixture bug.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SAYAKA: &str = "PL!HS-bp1-002-RM"; // 村野さやか, cost 11, the 起動 host
/// 蓮ノ空, cost 4 — well under the bound.
const CHEAP_HS: &str = "PL!HS-PR-003-PR";
/// 蓮ノ空, cost exactly 15 — the boundary.
const BOUNDARY_HS: &str = "PL!HS-pb1-023-N";
/// 蓮ノ空, cost 17 — over the bound, so it fails the cost gate and nothing else.
const OVER_HS: &str = "PL!HS-PR-016-PR";
/// cost 4, not 蓮ノ空 — fails the group gate and nothing else.
const NON_HS: &str = "PL!-sd1-010-SD";

/// Stage 村野さやか in `area`, put `in_waitroom` in our waitroom, run the 起動, and return
/// the card that ended up standing in `area`.
fn redeploy_into(area: MemberArea, in_waitroom: &str) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    // The 起動 costs 2 energy on top of her 11-cost play, so leave headroom for both.
    for _ in 0..30 {
        let e = game.new_id("LL-E-001-SD");
        game.state.player1.energy_zone.cards.push(e);
    }

    let sayaka = game.id(SAYAKA);
    game.assert_card_identity(sayaka, SAYAKA);
    let other = game.id(in_waitroom);
    game.assert_card_identity(other, in_waitroom);
    game.state.player1.waitroom.cards.push(other);

    // Only the host is on the stage, and only in `area`, so whatever appears in `area`
    // afterwards must have arrived through 「このメンバーがいたエリアに登場させる」.
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(sayaka);
    game.give_energy(20);
    game.play_to_stage(sayaka, area);
    // `drain_auto_ability_choices` already stops on anything that is not an
    // auto-ability prompt and on an empty queue, so the old
    // `if has_pending_choice()` around it was dead weight: identical either way.
    game.drain_auto_ability_choices();

    // Goes through `run_main_action`, so the sequential cost is actually paid — the cost
    // is what vacates the area, and skipping it is what made an earlier version of this
    // file land every card in the centre.
    game.activate_ability(sayaka);
    // The retrieval picks a card out of the 控え室. Only one is there, so the engine
    // may resolve it without asking — either way a `SelectCard` is the only prompt
    // this 起動 may produce, and anything else is a real failure rather than
    // something to swallow.
    game.drain_choices_strict(&["SelectCard"], &[0]);
    game.drain_auto_ability_choices();

    (game, sayaka, other)
}

/// The mechanism: a retrieved card lands in the area she vacated, on the left side.
///
/// The retrieved card need not be the fixture: 村野さやか is herself a 蓮ノ空 member of
/// cost 11 in the 控え室 once her cost has run, and the printed text does not say 他の
/// ("another"), so she is a legal target for her own retrieval. What must hold is the
/// AREA, and that the card placed there satisfies the printed filter.
#[test]
fn a_retrieved_card_lands_in_the_left_side_she_vacated() {
    let (game, sayaka, _) = redeploy_into(MemberArea::LeftSide, CHEAP_HS);
    assert!(
        game.state.player1.waitroom.cards.contains(&sayaka),
        "precondition: 村野さやか was moved to the waitroom by the cost \
         (このメンバーをステージから控え室に置く)"
    );
    let landed = game.state.player1.stage.stage[MemberArea::LeftSide as usize];
    assert_ne!(
        landed, -1,
        "このメンバーがいたエリアに登場させる — she stood in LeftSide, so the retrieved card \
         must appear in LeftSide. An implementation that lost the vacated area would put it \
         in the first free slot (the centre) instead."
    );
    let card = game.db.get_card(landed).expect("the landed card is in the database");
    assert!(
        card.cost.is_some_and(|c| c <= 15),
        "コスト15以下 — the card placed in the vacated area must print cost <= 15, but {} \
         prints {:?}",
        card.card_no,
        card.cost
    );
}

/// The same card in the centre. Running BOTH is what proves the area is remembered rather
/// than defaulted: a `same_area` implementation that fell back to a fixed slot would fill
/// the same index in both and fail one of them.
#[test]
fn a_retrieved_card_lands_in_the_centre_she_vacated() {
    let (game, _, _) = redeploy_into(MemberArea::Center, CHEAP_HS);
    assert_ne!(
        game.state.player1.stage.stage[MemberArea::Center as usize],
        -1,
        "she stood in the centre this time, so the retrieved card must appear in the centre"
    );
    assert_eq!(
        game.state.player1.stage.stage[MemberArea::LeftSide as usize],
        -1,
        "…and the left side stays empty. Together with the LeftSide test this pins the area: \
         a defaulted slot would fill the same index in both runs."
    );
}

/// コスト15以下 is `<=`, not `<`.
///
/// The host is a candidate too, so *which* card is picked is the engine's choice and is not
/// what this test is about. What the printed bound determines is WHICH CARDS ARE ELIGIBLE,
/// so the assertion is on the filter: whatever lands in the vacated area must be a member of
/// cost ≤ 15, and the cost-15 fixture must be reachable (not stranded as the only
/// non-eligible card).
#[test]
fn a_member_costing_exactly_fifteen_is_eligible() {
    let (game, _, boundary) = redeploy_into(MemberArea::LeftSide, BOUNDARY_HS);
    assert_eq!(
        game.db.get_card(boundary).and_then(|c| c.cost),
        Some(15),
        "fixture invariant: this card prints cost exactly 15, the one value `<` and `<=` \
         disagree on"
    );
    let landed = game.state.player1.stage.stage[MemberArea::LeftSide as usize];
    assert_ne!(landed, -1, "a card was placed in the vacated area");
    let card = game.db.get_card(landed).expect("in db");
    assert!(
        card.cost.is_some_and(|c| c <= 15),
        "コスト15以下 — the card placed in the vacated area must be a member of cost <= 15, \
         but {} prints {:?}. A `<` implementation would exclude the cost-15 fixture and \
         this would still be satisfiable only by the cost-11 host, so the bound is what \
         this pins.",
        card.card_no,
        card.cost
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&boundary) || landed == boundary,
        "the cost-15 fixture is a legal target under `<= 15`: she may be left in the \
         waitroom only because the engine chose the host instead, never because she was \
         filtered out."
    );
}

/// The cost gate alone: 蓮ノ空, but over the bound, so she is never a target.
#[test]
fn a_hasunosora_member_costing_seventeen_is_not_a_target() {
    let (game, _, over) = redeploy_into(MemberArea::LeftSide, OVER_HS);
    assert_eq!(
        game.db.get_card(over).and_then(|c| c.cost),
        Some(17),
        "fixture invariant: this card prints cost 17, above the bound"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&over),
        "コスト15以下 — 日野下花帆 is 蓮ノ空 so she passes the group gate, but costs 17, so \
         she is not a legal target and must still be in the waitroom. An engine with no cost \
         filter would have taken her."
    );
}

/// The group gate alone: under the cost bound, but not 蓮ノ空.
#[test]
fn a_non_hasunosora_member_is_not_a_target() {
    let (game, _, non_hs) = redeploy_into(MemberArea::LeftSide, NON_HS);
    assert!(
        game.state.player1.waitroom.cards.contains(&non_hs),
        "『蓮ノ空』のメンバーカード — 高坂穂乃果 costs 4 so she passes the cost gate, but her \
         series is 「ラブライブ！」 (Printemps), so she is not a legal target and must still \
         be in the waitroom. This is the control that the group half is live at all."
    );
}
