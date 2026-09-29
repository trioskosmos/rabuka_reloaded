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
//! `exclude_self: true` keeps the performing live card out of the move — it is on the
//! stage, so a formation change that could include it would be nonsense.
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
