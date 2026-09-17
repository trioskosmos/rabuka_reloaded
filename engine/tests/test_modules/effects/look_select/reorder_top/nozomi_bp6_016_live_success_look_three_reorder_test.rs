use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_setup::ActionType;

fn assert_live_success_reorder(order: [usize; 3]) {
    let mut game = TestGame::new(load_real_database());
    let nozomi = game.id("PL!-bp6-016-N");
    assert_eq!(
        game.state.card_database.get_card(nozomi).unwrap().card_no.as_ref(),
        "PL!-bp6-016-N"
    );
    let cards = [
        game.new_id("PL!-sd1-010-SD"),
        game.new_id("PL!S-sd1-001-SD"),
        game.new_id("PL!N-sd1-025-SD"),
        game.new_id("PL!-sd1-010-SD"),
        game.new_id("PL!S-sd1-001-SD"),
    ];
    let opponent = game.new_id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards = cards.to_vec().into();
    game.state.player2.main_deck.cards = vec![opponent].into();
    game.state.player1.stage.stage[1] = nozomi;

    fire_trigger(&mut game, nozomi, AbilityTrigger::LiveSuccess, "ライブ成功時");

    let mut remaining = cards[..3].to_vec();
    for &desired in &order[..2] {
        match game.get_pending_choice() {
            Choice::SelectTarget { target, allow_skip, .. } => {
                assert_eq!(target, "order");
                assert!(!allow_skip, "Returning all looked-at cards is mandatory");
            }
            choice => panic!("Expected mandatory reorder options, got {choice:?}"),
        }
        let option = remaining.iter().position(|&id| id == cards[desired]).unwrap();
        let actions = game.generated_actions();
        assert_eq!(actions.len(), remaining.len());
        assert!(actions.iter().all(|action| action.action_type == ActionType::ChoiceDecision));
        let offered = actions.iter().position(|action| {
            action.parameters.as_ref().and_then(|params| params.card_id) == Some(option as i16)
        }).expect("The desired remaining card must have an offered order action");
        game.select_generated(offered);
        remaining.remove(option);
    }

    assert!(!game.has_pending_choice(), "The final remaining card needs no choice");
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &[cards[order[0]], cards[order[1]], cards[order[2]], cards[3], cards[4]],
        "The chosen top-three order and untouched deck suffix must both be exact"
    );
    assert!(game.state.looked_at_cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(game.state.player1.stage.stage, [-1, nozomi, -1]);
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

#[test]
fn live_success_look_three_supports_every_changed_order() {
    for order in [[0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
        assert_live_success_reorder(order);
    }
}

#[test]
fn live_success_look_three_can_keep_original_order_without_skipping() {
    assert_live_success_reorder([0, 1, 2]);
}
