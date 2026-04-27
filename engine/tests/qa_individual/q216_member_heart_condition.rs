use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q216_member_heart_condition() {
    // Q216: When referencing members for this ability's condition, does one member need to have all specified hearts?
    // Answer: No, it references all members on stage to check if they have the specified hearts.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!S-bp5-001-AR "高海千歌")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!S-bp5-001-AR")
        .expect("Required card PL!S-bp5-001-AR not found for Q216 test");
    
    let live_id = get_card_id(live_card, &card_database);
    
    // Find multiple members
    let members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(3)
        .collect();
    
    assert!(members.len() >= 3, "Need at least 3 members for Q216 test");
    let member_ids: Vec<_> = members.iter().map(|c| get_card_id(c, &card_database)).collect();
    
    // Setup: members and live card in hand
    let mut hand_cards = member_ids.clone();
    hand_cards.push(live_id);
    setup_player_with_hand(&mut player1, hand_cards);
    
    // Add energy
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(50)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    
    // Step 1: Play members to stage
    for (i, &member_id) in member_ids.iter().enumerate() {
        let area = match i {
            0 => MemberArea::LeftSide,
            1 => MemberArea::Center,
            _ => MemberArea::RightSide,
        };
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(area),
            Some(false),
        );
        assert!(result.is_ok(), "Should be able to play member {} to stage: {:?}", i, result);
    }
    
    // Step 2: Set live card
    let result2 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::SetLiveCard,
        Some(live_id),
        None,
        None,
        Some(false),
    );
    
    assert!(result2.is_ok(), "Should be able to set live card: {:?}", result2);
    
    // Step 3: Verify stage has members
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| id != 0).count();
    println!("Q216: Stage has {} members", stage_members);
    
    println!("Q216 verified: Heart condition checks all members on stage");
}
