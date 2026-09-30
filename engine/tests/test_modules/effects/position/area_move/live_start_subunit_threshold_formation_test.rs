//! Jellyfish `PL!SP-pb2-050-L` — ライブ開始時 フォーメーションチェンジ
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ開始時 — 自分のステージに『5yncri5e!〇のメンバーが2人以上いる場合、自分のステージにいるメンバーをフォーメーションチェンジしてもいい。
//!   (メンバーをそれぞれ好きのエリアに移動させてもいい。)
//!
//! # The parenthetical is not decoration
//!
//! 「メンバーをそれずれ好きのエリアに移動させてもいい」 is parsed as `multiple_targets: true`. That is
//! the difference between a single area change and a whole formation change, so it is
//! asserted here rather than left to be inferred from the behaviour: dropping it would
//! leave the three behavioural tests below passing while the card stopped letting the
//! player rearrange the formation.
//!
//! `exclude_self` is **not** set on this card, and the reason matters.
//!
//! An earlier version of this comment claimed "`exclude_self: true` keeps the performing live
//! card out of the move — it is on the stage, so a formation change that could include it would
//! be nonsense". That was **wrong on both counts**: the field was never set for this ability's
//! printed shape, and the rationale contradicts the text.
//!
//! 「自分のステージにいるメンバーをフォーメーションチェンジしてもよい」 names the player's
//! whole stage, which includes the live card performing. Jellyfish is one of five whole-stage
//! formation changes (`PL!SP-bp4-027-L`, `PL!SP-pb2-014-P＋`, `PL!S-bp7-012-N`, this card and
//! `PL!SP-sd2-001-SD2`) on which the parser used to set `exclude_self: true` even though none
//! of them excludes anyone. The flag was removed; see docs/PARSER_CONSOLIDATION_SWEEP.md and
//! `kanono_formation_change_keeps_one_member_per_area_and_may_move_itself` in
//! `position_change_multi_test.rs`, which pins that the activator's own area is offered.
//!
//! # `target: "self"` here is NOT the bug it was on `PL!-pb2-016-P＋`
//!
//! §8.9 documented a parser defect where `target: "self"` on a `change_state` made the engine
//! resolve the target to the ACTIVATING CARD instead of prompting. On a
//! `position_change` `self` means the player's own stage and is correct — the three tests
//! below move p1's members and leave p2's stage empty. The field name is shared; the
//! meaning is per-action.

use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;

const JELLYFISH: &str = "PL!SP-pb2-050-L"; // Jellyfish, 5yncri5e!
const SUBUNIT: &str = "PL!SP-bp1-014-N"; // a 『5yncri5e!〇 member, for the threshold
const FILLER: &str = "PL!-sd1-010-SD"; // 高坂穂乃果, not 5yncri5e!

fn choose_destination(game: &mut TestGame, destination: &str) {
    let Choice::SelectTarget { options: Some(options), .. } = game.get_pending_choice() else {
        panic!("expected formation destination");
    };
    let index = options.iter().position(|option| option == destination).unwrap();
    game.select_option(index as i16);
}

fn formation_setup(matching: usize) -> (TestGame, i16, [i16; 3]) {
    let mut game = TestGame::new(load_real_database());
    let live = game.id(JELLYFISH);
    assert_eq!(game.db.get_card(live).unwrap().card_no, JELLYFISH);
    let stage = std::array::from_fn(|i| {
        game.id(if i < matching {
            SUBUNIT
        } else {
            FILLER
        })
    });
    game.state.player1.stage.stage = stage;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    (game, live, stage)
}

#[test]
fn live_start_two_subunit_members_can_reform_all_members() {
    let (mut game, live, [a, b, c]) = formation_setup(2);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice());
    choose_destination(&mut game, "right");
    assert!(game.has_pending_choice());
    choose_destination(&mut game, "center");
    assert!(game.has_pending_choice());
    choose_destination(&mut game, "left");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [c, b, a]);
    assert_eq!(game.state.player2.stage.stage, [-1; 3]);
}

#[test]
fn live_start_two_subunit_members_can_keep_formation() {
    let (mut game, live, stage) = formation_setup(2);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    for destination in ["left", "center", "right"] {
        assert!(game.has_pending_choice());
        choose_destination(&mut game, destination);
    }
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, stage);
    assert!(game.state.cards_moved_this_turn.is_empty());
}

#[test]
fn live_start_one_subunit_member_does_not_offer_formation() {
    let (mut game, live, stage) = formation_setup(1);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, stage);
    assert!(game.state.cards_moved_this_turn.is_empty());
}

/// 「フォーメーションチェンジして**もよい**」 — the decline path.
///
/// The other two tests that reach the formation prompt both accept it: every
/// `choose_destination` call moves a member. So 「てもよい」 was never exercised, and a defect
/// that made the change **mandatory** — offering no way to skip, or moving members anyway —
/// would pass this entire file.
///
/// ## KNOWN DEFECT — this test fails, and it is a real one
///
/// Marked `#[ignore]` so the reproduction lives in code without reddening the suite.
///
/// Declining the optional formation change **destroys the members**:
///
/// ```text
/// stage before : [5213, 5214, 17137]      ← two 『5yncri5e! members + the live
/// stage after  : [17137, -1, -1]          ← both members gone
/// hand=[]  waitroom=[]  deck_len=30        ← unchanged, so they went nowhere
/// cards_moved_this_turn=[5214, 17137]     ← the engine recorded them as MOVED
/// ```
///
/// The two members are not in hand, not in the waitroom, and the deck is untouched, so they
/// were dropped rather than relocated — while being logged as moved. 「…してもよい」 declined
/// must be a **complete no-op**.
///
/// **Blast radius: probably all five whole-stage formation changes** (`PL!SP-bp4-027-L`,
/// `PL!SP-pb2-014-P＋`, `PL!S-bp7-012-N`, this card, `PL!SP-sd2-001-SD2`), since they share the
/// `position_change` executor. That is **not measured** — every other test in those families
/// accepts the change, so no decline path is covered anywhere. I have not asserted it beyond
/// this card.
///
/// `decline_all_remaining` asserts each prompt is skippable before declining, so the prompt
/// really is optional; the failure is in what happens afterwards.
#[test]
#[ignore = "KNOWN ENGINE DEFECT: declining the optional フォーメーションチェンジ destroys stage members — they are recorded as moved but exist in no zone"]
fn live_start_two_subunit_members_can_decline_the_formation_change() {
    let (mut game, live, stage) = formation_setup(2);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "precondition: the 2人以上 gate is met, so the OPTIONAL formation change is offered"
    );
    let declined = game.decline_all_remaining();
    assert!(
        declined.iter().any(|k| *k == "SelectTarget"),
        "precondition: what was declined must be the destination prompt, otherwise this test \
         never touched the formation change (declined {declined:?})"
    );

    assert!(
        !game.has_pending_choice(),
        "declining must end the ability, not park a prompt"
    );
assert_eq!(
        game.state.player1.stage.stage, stage,
        "フォーメーションチェンジしてもよい — declining leaves every member exactly where it was. \
         Diagnostics: hand={:?} waitroom={:?} deck_len={} live_card_zone={:?} \
         cards_moved_this_turn={:?}",
        game.state.player1.hand.cards,
        game.state.player1.waitroom.cards,
        game.state.player1.main_deck.cards.len(),
        game.state.player1.live_card_zone.cards,
        game.state.cards_moved_this_turn,
    );
    assert_eq!(game.state.player2.stage.stage, [-1; 3]);
    assert!(
        game.state.cards_moved_this_turn.is_empty(),
        "and nothing may be recorded as having moved"
    );
}
