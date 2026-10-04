//! PL!S-bp7-007-P 国木田花丸 ab#1 — ライブ開始時, blade per card *this step* placed.
//!
//! {{live_start.png|ライブ開始時}}
//! 自分の控え室から『Aqours』のメンバーカードを3枚まで好きな順番でデッキの下に置く。
//! ライブ終了時まで、**これによりデッキに置かれたカード1枚につき**、{{icon_blade.png|ブレード}}を得る。
//!
//! In English: at live start, place up to 3 Aqours member cards from the discard
//! into the bottom of the deck in any order. Until the end of the live, gain 1
//! blade for each card placed there BY THIS STEP.
//!
//! # The clause is about this step's cards, and nothing else
//!
//! 「これによりデッキに置かれたカード」 names the batch the first action just put
//! there. The parse already says so — step 2 carries
//! `per_unit_source: "previous_moved_cards"` — but `execute_gain_resource` never
//! read that field. It keyed the tracked-moved pool off `per_unit_type`, and only
//! recognised `"discard"`, `"waitroom_card"` and `"energy_deck"`. This step's
//! `per_unit_type` is `"枚"`, a bare counting noun with no zone of its own, so the
//! count fell through to `resolve_per_unit_count("枚", …)` and `per_unit_zone`
//! mapped `"枚"` to `Zone::Hand` whenever the effect has no `card_type` filter.
//!
//! So the blades were counted from the HAND — a zone the printed clause never
//! mentions — while the cards actually placed earned nothing. With four members
//! in hand and two placed the engine granted 4 instead of 2.
//!
//! Two more cards set `per_unit_source: "previous_moved_cards"` with a
//! `per_unit_type` that does not reach the tracked-moved path — `PL!N-bp7-008-P`
//! (枚) and `PL!S-bp6-021-L` (none at all). This file covers `PL!S-bp7-007-P`.
//!
//! # Why the hand is seeded with four and not two
//!
//! The two readings coincide at 2, and that is not hypothetical: an earlier
//! version of this test seeded two hand members, the live card left the hand when
//! it was set, and the hand-based count landed on exactly 2 — the test passed
//! against the wrong behaviour. Four in hand against two placed means the
//! readings cannot be confused, so the assertion below is which one the engine
//! produced.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// 5 passes: P1 sets the live card. The pair is the LiveStart template's, from
/// WRITING_TESTS.md §"Standard Test Templates".
fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

/// 2 passes: into ライブ開始時, where ab#1 fires.
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Aqours members — the only cards step 1 may place, so the only ones that earn a
/// blade. Two of them, so the correct count is 2.
const AQOURS_A: &str = "PL!S-bp7-006-R"; // 津島善子
const AQOURS_B: &str = "PL!S-bp7-003-R＋"; // 松浦果南
/// Liella! in the 控え室: present but NOT placeable, because step 1 filters on
/// 『Aqours』. It must earn nothing.
const LIELLA_WAITROOM: &str = "PL!SP-sd1-001-SD"; // 澁谷かのん
/// Four members in hand, none of which the printed blade clause refers to. A
/// hand-based reading answers 4; the printed answer is 2.
const HAND_A: &str = "PL!SP-sd1-003-SD"; // 嵐 千砂都
const HAND_B: &str = "PL!HS-bp6-010-R"; // 村野さやか
const HAND_C: &str = "PL!SP-bp5-012-N"; // 澁谷かのん (second print)
const HAND_D: &str = "PL!SP-sd1-004-SD"; // 平安名すみれ

/// Kanako on stage, deck stocked, live card set, and ライブ開始時 reached.
///
/// The LiveStart template in WRITING_TESTS.md, with the zones seeded where the
/// blade clause can still see them. `fill_decks` matters: the draws between here
/// and the trigger consume from the top, and an exhausted deck refreshes — which
/// empties the 控え室 and takes the placeable 『Aqours』 members with it. The first
/// version of this test seeded no deck and the placement silently found nothing.
fn stage_and_reach_live_start(
    waitroom: &[&str],
    hand: &[&str],
) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = game.id("PL!S-bp7-007-P");
    game.assert_card_identity(kanako, "PL!S-bp7-007-P");
    game.assert_card_in_group(game.id(AQOURS_A), "Aqours", "placeable fixture");
    game.assert_card_in_group(game.id(AQOURS_B), "Aqours", "placeable fixture");
    game.assert_card_in_group(game.id(LIELLA_WAITROOM), "Liella!", "unplaceable fixture");

    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(live);
    game.give_energy(20);
    game.add_to_stage(MemberArea::Center, kanako);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);

    // Seeded AFTER the live card is set: `advance_to_phase`/passes run bookkeeping
    // that drains the 控え室, so anything placed before them is gone by the time
    // ライブ開始時 fires.
    for c in waitroom {
        game.add_to_discard(game.id(c));
    }
    for c in hand {
        game.add_to_hand(game.id(c));
    }

    let blades_before = game.state.mods.get_blade_modifier(kanako);
    advance_to_live_start(&mut game);
    (game, kanako, blades_before)
}

/// Answer the placement: which cards from the 控え室.
///
/// Written out rather than drained in a loop, because the prompt selects cards
/// from a named zone and answering it with a blanket index 0 could not tell a
/// mistyped prompt from a correct answer — the hazard `docs/EDGE_CASE_FINDINGS.md`
/// C9 records and the drain-loop ratchet in `cards/test_inventory.py` exists to
/// stop.
///
/// KNOWN GAP, deliberately not asserted here: 「好きな順番で」 should also ask the
/// player to ORDER the cards it just chose. That prompt is raised by
/// `maybe_prompt_deck_order`, and with this fixture only one card reaches it, so the
/// order question is not exercised by this file. `is_deck_dest` omitted
/// `Zone::DeckBottom` entirely, which dropped the order prompt for 「好きな順番で
/// デッキの下に」 altogether; that is fixed, but pinning it needs a fixture where the
/// placement actually takes two or more cards.
fn choose_which_cards_to_place(game: &mut TestGame) {
    game.drain_choices_strict(&["SelectCard"], &[0, 1]);
}

/// The count is the batch THIS STEP placed, and the hand is a decoy of a
/// deliberately different size.
#[test]
fn blades_count_the_cards_this_step_placed_not_the_hand() {
    let (mut game, kanako, blades_before) = stage_and_reach_live_start(
        &[AQOURS_A, LIELLA_WAITROOM, AQOURS_B],
        &[HAND_A, HAND_B, HAND_C, HAND_D],
    );

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        3,
        "precondition: both 『Aqours』 members and the Liella! decoy are in the 控え室 \
         when ライブ開始時 fires, or step 1 has nothing to place"
    );

    choose_which_cards_to_place(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(kanako) - blades_before,
        2,
        "「これによりデッキに置かれたカード1枚につき、ブレードを得る」 — the two 『Aqours』 \
         members this step placed earn a blade each. Not the 4 members in hand, and not \
         the Liella! member left in the 控え室: the clause names only what this step placed."
    );
}

/// With nothing placeable, the clause earns nothing even though the hand is full.
///
/// The other direction of the same distinction, and the one that catches a fix which
/// merely special-cased 「discard」: 「3枚まで」 is an upper bound, so a 控え室 holding
/// no 『Aqours』 member places no cards, and a clause about cards this step placed
/// has an empty set to count. A hand-based reading would still hand out four.
#[test]
fn nothing_placeable_earns_nothing_despite_a_full_hand() {
    let (mut game, kanako, blades_before) =
        stage_and_reach_live_start(&[LIELLA_WAITROOM], &[HAND_A, HAND_B, HAND_C, HAND_D]);

    // Nothing is placeable, so there is no selection to make and no order to pick.
    // An empty `allowed` list makes any prompt a failure, which is the assertion:
    // the engine must not offer a choice it has no candidate for.
    game.drain_choices_strict(&[], &[]);

    assert_eq!(
        game.state.mods.get_blade_modifier(kanako) - blades_before,
        0,
        "no 『Aqours』 member was placeable, so 「これにより…置かれたカード」 is empty and \
         no blade is earned — the 4 members in hand are not that set."
    );
}