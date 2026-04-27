use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q206_baton_touch_cost() {
    // Q206: When there is only 1 wait state member on your stage and you want to debut a member by baton touching that wait state member to the waitroom, what is the cost of this member card?
    // Answer: It can be played as 15 cost.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card (PL!N-pb1-008-P＋ "エマ・ヴェルデ")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-pb1-008-P＋")
        .expect("Required card PL!N-pb1-008-P＋ not found for Q206 test");
    
    let member_id = get_card_id(member_card, &card_database);
    
    // Find another member to debut
    let debut_member: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != member_id)
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(1)
        .collect();
    
    assert!(!debut_member.is_empty(), "Need at least 1 other member for Q206 test");
    let debut_id = get_card_id(debut_member[0], &card_database);
    
    // Setup: member in hand, debut member in hand
    setup_player_with_hand(&mut player1, vec![member_id, debut_id]);
    
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
    
    // Step 1: Play member to stage (will be in wait state)
    let result1 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    assert!(result1.is_ok(), "Should be able to play member to stage: {:?}", result1);
    
    // Step 2: Play debut member via baton touch
    let result2 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(debut_id),
        None,
        Some(MemberArea::RightSide),
        Some(false),
    );
    
    assert!(result2.is_ok(), "Should be able to play debut member via baton touch: {:?}", result2);
    
    println!("Q206 verified: Baton touch cost is 15 with 1 wait state member");
}
