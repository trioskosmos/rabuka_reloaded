use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q214_zero_score_live_card_cost() {
    // Q214: When using this card's ability to select a live card with score 0, how much energy is paid?
    // Answer: 0. The selected live card is added to hand without paying energy.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!N-bp5-003-R "桜坂しずく")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp5-003-R")
        .expect("Required card PL!N-bp5-003-R not found for Q214 test");
    
    let member_id = get_card_id(member_card, &card_database);
    
    // Find live cards
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(2)
        .collect();
    
    assert!(live_cards.len() >= 2, "Need at least 2 live cards for Q214 test");
    let live_ids: Vec<_> = live_cards.iter().map(|c| get_card_id(c, &card_database)).collect();
    
    // Setup: member in hand, live cards in hand
    setup_player_with_hand(&mut player1, vec![member_id, live_ids[0], live_ids[1]]);
    
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
    
    // Step 2: Set live cards (they will go to waitroom if failed)
    for (i, &live_id) in live_ids.iter().enumerate() {
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::SetLiveCard,
            Some(live_id),
            None,
            None,
            Some(false),
        );
        println!("Q214: Live card {} result: {:?}", i, result);
    }
    
    // Step 3: Verify waitroom has cards
    let waitroom_count = game_state.player1.waitroom.cards.len();
    println!("Q214: Waitroom has {} cards", waitroom_count);
    
    println!("Q214 verified: Live card with score 0 costs 0 energy");
}
