use rabuka_engine::game_state::{GameState, Phase, TurnPhase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_deck};

#[test]
fn test_q038_live_card_definition() {
    // Q038: What is a "live card" (ライブ中のカード)?
    // Answer: A live card placed face-up in the live card zone during performance phase.

    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());

    // Find a live card
    let live_card = cards.iter()
        .find(|c| c.is_live() && get_card_id(c, &card_database) != 0)
        .expect("Need a live card for Q038");
    let live_id = get_card_id(live_card, &card_database);

    // Find a member card
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.unwrap_or(0) <= 2)
        .filter(|c| get_card_id(c, &card_database) != 0)
        .next()
        .expect("Need a member card for Q038");
    let member_id = get_card_id(member_card, &card_database);

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    setup_player_with_hand(&mut player1, vec![live_id, member_id]);

    let energy_card_ids: Vec<i16> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(10)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids.clone());

    let deck_cards: Vec<i16> = cards.iter()
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    setup_player_with_deck(&mut player1, deck_cards.clone());
    setup_player_with_deck(&mut player2, deck_cards);

    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;

    // Record before state
    let hand_before = game_state.player1.hand.cards.len();

    // Advance through phases to LiveCardSet
    TurnEngine::advance_phase(&mut game_state);
    TurnEngine::advance_phase(&mut game_state);
    assert_eq!(game_state.current_phase, Phase::LiveCardSetP1Turn,
        "Should be in LiveCardSetP1Turn");

    // Set the live card via TurnEngine action
    TurnEngine::execute_main_phase_action(
        &mut game_state, &rabuka_engine::game_setup::ActionType::SetLiveCard,
        Some(live_id), None, None, None,
    ).expect("SetLiveCard should succeed");

    // Verify: card moved from hand to live_card_zone
    assert!(game_state.player1.live_card_zone.cards.contains(&live_id),
        "Live card should be in live_card_zone after SetLiveCard");
    assert!(!game_state.player1.hand.cards.contains(&live_id),
        "Live card should not be in hand");

    // Verify: hand decreased (card moved from hand)
    assert_eq!(game_state.player1.hand.cards.len(), hand_before - 1,
        "Hand should have 1 fewer card");

    // Verify: the card in the live zone IS a live-type card
    let card_in_zone = game_state.player1.live_card_zone.cards[0];
    let db_card = card_database.get_card(card_in_zone);
    assert!(db_card.is_some(), "Card in live zone should exist in DB");
    assert!(db_card.unwrap().is_live(), "Card in live zone should be live type");

    // Advance through passes to reach performance phase
    TurnEngine::execute_main_phase_action(
        &mut game_state, &rabuka_engine::game_setup::ActionType::Pass,
        None, None, None, None,
    ).expect("P1 pass");
    TurnEngine::execute_main_phase_action(
        &mut game_state, &rabuka_engine::game_setup::ActionType::Pass,
        None, None, None, None,
    ).expect("P2 pass");

    assert_eq!(game_state.current_phase, Phase::FirstAttackerPerformance,
        "Should reach performance phase");

    // Verify: live card persists through phase transitions
    assert!(game_state.player1.live_card_zone.cards.contains(&live_id),
        "Live card should remain during performance phase");
    assert_eq!(game_state.player1.live_card_zone.cards.len(), 1,
        "Should have exactly 1 live card");

    // Verify: live card is NOT in hand (it's on the field now)
    assert!(!game_state.player1.hand.cards.contains(&live_id),
        "Live card should not be in hand during performance");
}
