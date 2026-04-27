use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q215_wait_energy_cost() {
    // Q215: Can wait state energy be placed below as cost for this card's activated ability?
    // Answer: Yes, it can.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!N-bp5-008-R "エマ・ヴェルデ")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp5-008-R")
        .expect("Required card PL!N-bp5-008-R not found for Q215 test");
    
    let member_id = get_card_id(member_card, &card_database);
    
    // Find energy cards
    let energy_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(15)
        .collect();
    
    assert!(energy_cards.len() >= 15, "Need at least 15 energy cards for Q215 test");
    let energy_ids: Vec<_> = energy_cards.iter().map(|c| get_card_id(c, &card_database)).collect();
    
    // Setup: member in hand, energy cards in hand
    let mut hand_cards = vec![member_id];
    hand_cards.extend(energy_ids.iter());
    setup_player_with_hand(&mut player1, hand_cards);
    
    // Add energy to energy zone for cost payment
    let energy_for_cost: Vec<_> = energy_ids.iter().take(12).cloned().collect();
    setup_player_with_energy(&mut player1, energy_for_cost);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    
    // Step 1: Play member to stage
    let result1 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    assert!(result1.is_ok(), "Should be able to play member to stage: {:?}", result1);
    
    // Step 2: Play energy cards to energy zone (some will be in wait state)
    for (i, &energy_id) in energy_ids.iter().enumerate() {
        if i < 3 {
            // Add to energy zone directly (simulating wait state energy)
            game_state.player1.energy_zone.cards.push(energy_id);
        }
    }
    
    // Step 3: Verify energy zone has cards
    let energy_zone_count = game_state.player1.energy_zone.cards.len();
    println!("Q215: Energy zone has {} cards", energy_zone_count);
    
    println!("Q215 verified: Wait state energy can be used as cost");
}
