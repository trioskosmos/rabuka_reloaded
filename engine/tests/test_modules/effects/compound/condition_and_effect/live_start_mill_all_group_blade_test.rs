use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::types::TurnPhase;

fn mill_group_setup(wrong_index: Option<usize>) -> (TestGame, i16, [i16; 5], i16) {
    let mut game = TestGame::new(load_real_database());
    let member = game.id("PL!HS-bp6-009-R");
    assert_eq!(game.db.get_card(member).unwrap().card_no, "PL!HS-bp6-009-R");
    game.state.player1.stage.stage[1] = member;
    let cards = std::array::from_fn(|index| {
        game.id(if wrong_index == Some(index) || index == 4 {
            "PL!-sd1-010-SD"
        } else if index == 1 {
            "PL!HS-bp6-028-L"
        } else {
            "PL!HS-bp6-010-R"
        })
    });
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let waiting = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(waiting);
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards.push(opponent);
    game.state.current_turn_phase = TurnPhase::Live;
    fire_trigger(&mut game, member, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &cards[4..]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[waiting, cards[0], cards[1], cards[2], cards[3]]);
    assert!(game.state.player1.hand.cards.is_empty());
    (game, member, cards, waiting)
}

#[test]
fn mill_four_all_group_cards_grants_one_blade_until_live_end() {
    let (mut game, member, _, _) = mill_group_setup(None);
    assert_eq!(game.state.mods.get_blade_modifier(member), 1);
    game.state.check_expired_effects();
    assert_eq!(game.state.mods.get_blade_modifier(member), 1);
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.check_expired_effects();
    assert_eq!(game.state.mods.get_blade_modifier(member), 0);
    assert!(game.state.temporary_effects.is_empty());
}

#[test]
fn mill_four_any_wrong_group_card_prevents_blade_rider() {
    for wrong_index in 0..4 {
        let (game, member, _, _) = mill_group_setup(Some(wrong_index));
        assert_eq!(game.state.mods.get_blade_modifier(member), 0);
        assert!(game.state.temporary_effects.is_empty());
    }
}
