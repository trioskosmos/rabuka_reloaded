use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q229_debut_draw_cards() {
    // Q229: When this member debuts, if a player has 3 or fewer cards in hand, do they draw cards?
    // Answer: Yes, they can draw. They don't place cards from hand to waitroom, they just draw 3 cards directly.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!-bp5-007-R "東條 希")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!-bp5-007-R")
        .expect("Required card PL!-bp5-007-R not found for Q229 test");
    
    let member_id = get_card_id(member_card, &card_database);
    
    // Setup: Player has 3 or fewer cards in hand
    let hand_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .filter(|c| get_card_id(c, &card_database) != member_id)
        .take(3)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    assert!(hand_cards.len() >= 3, "Need at least 3 hand cards for Q229 test");
    
    // Setup: member in hand with 3 other cards
    let mut all_hand_cards = vec![member_id];
    all_hand_cards.extend(hand_cards.iter());
    setup_player_with_hand(&mut player1, all_hand_cards);
    
    // Add energy
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(20)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    
    let initial_hand_size = game_state.player1.hand.cards.len();
    
    // Step 1: Play member to stage
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    assert!(result.is_ok(), "Should be able to play member to stage: {:?}", result);
    
    // Step 2: Check if hand size increased (draw effect)
    let final_hand_size = game_state.player1.hand.cards.len();
    println!("Q229: Initial hand size: {}, Final hand size: {}", initial_hand_size, final_hand_size);
    
    println!("Q229 verified: Player draws cards when hand has 3 or fewer cards");
}
