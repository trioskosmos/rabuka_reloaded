use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn setup_maki_reveal(waitroom_cards: &[i16], success_cards: &[i16]) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id("PL!-sd1-006-SD");
    let crossroads = game.id("PL!-bp6-024-L");

    game.add_to_hand(maki);
    game.add_to_hand(crossroads);
    game.state
        .player1
        .waitroom
        .cards
        .extend_from_slice(waitroom_cards);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .extend_from_slice(success_cards);
    game.give_energy(9);

    (game, maki, crossroads)
}

fn select_maki_reveal(game: &mut TestGame, card_id: i16) {
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone,
            card_type,
            count,
            allow_skip,
            ..
        } => {
            assert_eq!(zone, "hand");
            assert_eq!(card_type.as_deref(), Some("live_card"));
            assert_eq!(*count, 1);
            assert!(*allow_skip);
        }
        choice => panic!("expected optional reveal choice, got {choice:?}"),
    }
    let action = game
        .generated_actions()
        .into_iter()
        .find(|action| {
            action.action_type == ActionType::ChoiceSelect
                && action
                    .parameters
                    .as_ref()
                    .and_then(|parameters| parameters.card_id)
                    == Some(card_id)
                && action
                    .parameters
                    .as_ref()
                    .and_then(|parameters| parameters.disabled)
                    != Some(true)
        })
        .expect("generated action for revealed live");
    let physical_index = action
        .parameters
        .as_ref()
        .and_then(|parameters| parameters.card_index)
        .expect("generated physical hand index");
    TurnEngine::resume_with_choice(&mut game.state, None, Some(vec![physical_index]))
        .expect("revealed live selection");
}

fn assert_crossroads_replacement_choice(game: &TestGame) {
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone,
            card_type,
            count,
            allow_skip,
            group,
            ..
        } => {
            assert_eq!(zone, "discard");
            assert_eq!(card_type.as_deref(), Some("live_card"));
            assert_eq!(*count, 1);
            assert!(*allow_skip);
            assert_eq!(group.as_deref(), Some("μ's"));
        }
        choice => panic!("expected CROSSROADS replacement choice, got {choice:?}"),
    }
}

fn zone_occurrences(game: &TestGame, card_id: i16) -> usize {
    let zones: [&[i16]; 7] = [
        game.state.player1.hand.cards.as_slice(),
        game.state.player1.waitroom.cards.as_slice(),
        game.state.player1.live_card_zone.cards.as_slice(),
        game.state.player1.success_live_card_zone.cards.as_slice(),
        game.state.player1.stage.stage.as_slice(),
        game.state.revealed_cards.as_slice(),
        game.state.revealed_cost_cards.as_slice(),
    ];
    zones
        .into_iter()
        .flat_map(|cards| cards.iter().filter(|candidate| **candidate == card_id))
        .count()
}

#[test]
fn maki_q256_accepts_crossroads_replacement() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let muse = game.id("PL!-bp3-019-L");
    let existing_success = game.new_id("PL!SP-sd1-023-SD");
    game.add_to_discard(muse);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(existing_success);

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, crossroads);

    assert_crossroads_replacement_choice(&game);
    game.select_indices(&[0]);

    assert!(game.state.player1.hand.cards.contains(&existing_success));
    assert!(!game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&existing_success));
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&muse));
    assert!(game.state.player1.waitroom.cards.contains(&crossroads));
    for card_id in [maki, muse, crossroads, existing_success] {
        assert_eq!(zone_occurrences(&game, card_id), 1, "card {card_id}");
    }
}

#[test]
fn maki_q256_declines_crossroads_replacement() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let muse = game.id("PL!-bp3-019-L");
    let existing_success = game.new_id("PL!SP-sd1-023-SD");
    game.add_to_discard(muse);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(existing_success);

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, crossroads);
    assert!(!game.state.player1.hand.cards.contains(&crossroads));
    assert!(!game.state.revealed_cost_cards.contains(&crossroads));

    assert_crossroads_replacement_choice(&game);

    game.select_indices(&[]);

    assert!(game.state.player1.hand.cards.contains(&existing_success));
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&crossroads));
    assert!(game.state.player1.waitroom.cards.contains(&muse));
    assert!(!game.has_pending_choice());
}

#[test]
fn maki_q256_can_decline_reveal() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let muse = game.id("PL!-bp3-019-L");
    let existing_success = game.new_id("PL!SP-sd1-023-SD");
    game.add_to_discard(muse);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(existing_success);

    game.play_to_stage(maki, MemberArea::Center);
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone, allow_skip, ..
        } => {
            assert_eq!(zone, "hand");
            assert!(*allow_skip);
        }
        choice => panic!("expected optional reveal choice, got {choice:?}"),
    }
    game.select_indices(&[]);

    assert!(game.state.player1.hand.cards.contains(&crossroads));
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&existing_success));
    assert!(game.state.player1.waitroom.cards.contains(&muse));
    assert!(!game.has_pending_choice());
}

#[test]
fn maki_q256_without_live_card_in_hand_does_nothing() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let existing_success = game.new_id("PL!SP-sd1-023-SD");
    game.state
        .player1
        .hand
        .cards
        .retain(|card_id| *card_id != crossroads);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(existing_success);

    game.play_to_stage(maki, MemberArea::Center);

    assert!(!game.has_pending_choice());
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&existing_success));
}

#[test]
fn maki_q256_empty_success_still_attempts_placement() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let muse = game.id("PL!-bp3-019-L");
    game.add_to_discard(muse);

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, crossroads);
    assert!(!game.state.player1.hand.cards.contains(&crossroads));
    assert!(!game.state.revealed_cost_cards.contains(&crossroads));

    assert_crossroads_replacement_choice(&game);
    game.select_indices(&[0]);

    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&muse));
    assert!(game.state.player1.waitroom.cards.contains(&crossroads));
}

#[test]
fn maki_q256_selects_exact_success_card() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let selected = game.new_id("PL!SP-sd1-023-SD");
    let remaining = game.new_id("PL!SP-sd1-023-SD");
    game.state
        .player1
        .success_live_card_zone
        .cards
        .extend_from_slice(&[selected, remaining]);

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, crossroads);

    match game.get_pending_choice() {
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            filtered_indices,
            ..
        } => {
            assert_eq!(zone, "success_live_zone");
            assert_eq!(*count, 1);
            assert!(!*allow_skip);
            assert_eq!(filtered_indices.as_deref(), Some(&[0, 1][..]));
        }
        choice => panic!("expected success-zone choice, got {choice:?}"),
    }
    game.select_indices(&[0]);

    assert!(game.state.player1.hand.cards.contains(&selected));
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&remaining));
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&crossroads));
    assert!(!game.has_pending_choice());
}

#[test]
fn maki_q256_non_crossroads_reveal_bypasses_replacement() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let ordinary_live = game.id("PL!HS-bp6-026-L");
    game.state
        .player1
        .hand
        .cards
        .retain(|card_id| *card_id != crossroads);
    game.add_to_hand(ordinary_live);

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, ordinary_live);

    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&ordinary_live));
    assert!(!game.has_pending_choice());
}

#[test]
fn maki_q256_non_mus_waitroom_does_not_prompt() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let non_mus = game.id("PL!SP-sd1-023-SD");
    game.add_to_discard(non_mus);

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, crossroads);

    assert!(!game.has_pending_choice());
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&crossroads));
    assert!(game.state.player1.waitroom.cards.contains(&non_mus));
}

#[test]
fn maki_q256_filters_multiple_mus_targets() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let non_mus = game.new_id("PL!SP-sd1-023-SD");
    let first_mus = game.id("PL!-bp3-019-L");
    let second_mus = game.id("PL!-bp6-022-L");
    game.state
        .player1
        .waitroom
        .cards
        .extend_from_slice(&[non_mus, first_mus, second_mus]);

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, crossroads);

    match game.get_pending_choice() {
        Choice::SelectCard {
            filtered_indices, ..
        } => assert_eq!(filtered_indices.as_deref(), Some(&[1, 2][..])),
        choice => panic!("expected filtered replacement choice, got {choice:?}"),
    }
    game.select_waitroom_card_filtered(second_mus);

    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&second_mus));
    assert!(game.state.player1.waitroom.cards.contains(&first_mus));
    assert!(game.state.player1.waitroom.cards.contains(&non_mus));
    assert!(game.state.player1.waitroom.cards.contains(&crossroads));
}

#[test]
fn maki_q256_uses_only_the_cost_revealed_card() {
    let (mut game, maki, crossroads) = setup_maki_reveal(&[], &[]);
    let muse = game.id("PL!-bp3-019-L");
    let unrelated_reveal = game.id("PL!S-PR-022-PR");
    let existing_success = game.new_id("PL!SP-sd1-023-SD");
    game.add_to_discard(muse);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(existing_success);
    game.state
        .push_revealed_card(unrelated_reveal, None, false, Some(0), "yell");

    game.play_to_stage(maki, MemberArea::Center);
    select_maki_reveal(&mut game, crossroads);

    assert_crossroads_replacement_choice(&game);
    game.select_indices(&[0]);

    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&muse));
    assert!(game.state.player1.waitroom.cards.contains(&crossroads));
    assert!(game.state.revealed_cards.contains(&unrelated_reveal));
    assert!(!game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&unrelated_reveal));
    assert!(game.state.player1.hand.cards.contains(&existing_success));
}
