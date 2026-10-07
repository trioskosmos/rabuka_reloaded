//! PL!-PR-023-PR 絢瀬絵里 ab#1
//!
//! {{kidou.png|起動}}{{turn1.png|ターン1回}}メンバー1人をウェイトにし、
//! 手札を1枚控え室に置く：カードを1枚引く
//!
//! Printed behaviour under test:
//!   * the sequential cost waits ONE own member and discards one hand card
//!   * the effect draws one card, so the hand nets to unchanged and the deck
//!     is one card shorter
//!   * the cost's wait is a real transition, so ab#0's watcher takes a blade
//!   * ターン1回 — a second activation the same turn is refused without
//!     spending energy or paying the cost
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

const ERI: &str = "PL!-PR-023-PR"; // ab#0 watcher + ab#1 起動
const MEMBER: &str = "PL!-sd1-010-SD"; // abilityless cost-4 member (ally / fodder)

/// Eri at slot 0, the ally at slot 1, three hand cards, a padded deck and
/// enough energy that cost is never the blocker.
fn setup() -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eri = game.id(ERI);
    let ally = game.id(MEMBER);
    game.state.player1.stage.stage = [eri, ally, -1];
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(MEMBER));
    }
    for _ in 0..3 {
        game.state.player1.hand.cards.push(game.id(MEMBER));
    }
    game.give_energy(25);
    (game, eri, ally)
}

/// Drive the activation chain: pay the wait cost, pay the discard, resolve any
/// auto-ability ordering prompt, and fail loudly on anything else. Returns
/// whether the cost asked WHICH member to wait (it resolves silently when a
/// single member qualifies).
fn activate_and_resolve(game: &mut TestGame, eri: i16) -> bool {
    game.activate_ability_index(eri, 1);
    let mut saw_stage = false;
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 20, "起動 chain did not drain in 20 prompts");
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { zone, .. } if zone == "stage" && !saw_stage => {
                // Stage order is [eri, ally, ...]: index 1 is the ally.
                game.select_indices(&[1]);
                saw_stage = true;
            }
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                game.select_indices(&[0]);
            }
            other => panic!("unexpected choice in the 起動 chain: {:?}", other),
        }
    }
    saw_stage
}

#[test]
fn kidou_waits_a_member_discards_one_and_draws_one() {
    let (mut game, eri, ally) = setup();
    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();
    let blade_before = game.state.mods.get_blade_modifier(eri);

    let saw_stage = activate_and_resolve(&mut game, eri);

    assert!(
        saw_stage,
        "the cost must ask which of the eligible members to wait"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(ally),
        Some("wait"),
        "「メンバー1人をウェイトにし」: the picked ally is waited"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "discard 1 then draw 1 leaves the hand size unchanged"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "「カードを1枚引く」: the effect draws exactly one card"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(eri),
        blade_before + 1,
        "the cost's wait transition feeds ab#0's [ターン3回] watcher"
    );
}

#[test]
fn kidou_asks_which_member_when_two_are_eligible() {
    let (mut game, eri, ally) = setup();
    let other = game.id(MEMBER);
    game.state.player1.stage.stage[2] = other;

    let saw_stage = activate_and_resolve(&mut game, eri);

    assert!(
        saw_stage,
        "two eligible members → the cost must ask which one to wait"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(ally),
        Some("wait"),
        "stage index 1 (the pick) is the member that waits"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(other),
        None,
        "the unpicked eligible member was never written to — base active"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(eri),
        None,
        "the activating member is not her own cost target — never written"
    );
}

#[test]
fn kidou_is_refused_a_second_time_in_the_same_turn() {
    let (mut game, eri, ally) = setup();
    activate_and_resolve(&mut game, eri);
    assert_eq!(
        game.state.mods.get_orientation_modifier(ally),
        Some("wait"),
        "precondition: the first 起動 resolved"
    );

    let energy_before = game.state.player1.energy_zone.active_count() as i16;
    let hand_before = game.state.player1.hand.cards.len();
    assert!(
        game.try_activate_ability_index(eri, 1).is_err(),
        "ターン1回 — the second activation must be refused, got Ok"
    );
    game.drain_choices();
    game.assert_energy_untouched_after_refusal(energy_before, "起動 ターン1回");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "a refusal must not pay the discard cost"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(ally),
        Some("wait"),
        "a refusal changes nothing on the board"
    );
}
