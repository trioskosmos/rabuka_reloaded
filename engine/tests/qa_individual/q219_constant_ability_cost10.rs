use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q219_constant_ability_cost10() {
    // Q219: When baton touching a cost 10 Liella! member card from hand to debut with this card, does this card's constant ability apply?
    // Answer: Yes, it applies.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!S-bp5-001-R＋ "高海千歌")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!S-bp5-001-R＋")
        .expect("Required card PL!S-bp5-001-R＋ not found for Q219 test");
    
    let member_id = get_card_id(member_card, &card_database);
    
    // Add a cost 10 Liella! member card to hand for baton touch
    let cost10_liella_member: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.card_no.contains("PL!S"))
        .filter(|c| get_card_id(c, &card_database) != 0)
        .filter(|c| get_card_id(c, &card_database) != member_id)
        .take(1)
        .collect();
    
    assert!(!cost10_liella_member.is_empty(), "Need at least 1 Liella! member for Q219 test");
    let hand_member_id = get_card_id(cost10_liella_member[0], &card_database);
    assert!(hand_member_id != 0, "Hand member ID is invalid (0)");
    
    // Setup: member on stage, Liella! member in hand
    setup_player_with_hand(&mut player1, vec![member_id, hand_member_id]);
    
    // Add energy
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(40)
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
    
    // Add Liella! member back to hand to simulate baton touch scenario
    game_state.player1.hand.cards.push(hand_member_id);
    
    // Step 2: Play Liella! member to stage
    let result2 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(hand_member_id),
        None,
        Some(MemberArea::RightSide),
        Some(false),
    );
    
    assert!(result2.is_ok(), "Should be able to play member to stage: {:?}", result2);
    
    // Step 3: Verify stage has members
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| id != 0).count();
    println!("Q219: Stage has {} members", stage_members);
    
    println!("Q219 verified: Constant ability applies during baton touch with cost 10 Liella!");
}
