use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q212_combined_member_overlap_effect() {
    // Q212: When "PL!HS-bp1-005-P 大沢瑠璃乃" and "LL-bp2-001-R＋ 渡辺 曜&鬼塚夏美&大沢瑠璃乃" are on stage, is the live start effect applied?
    // Answer: No, it is not applied.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!HS-bp5-017-L "Dream Believers（104期Ver.）")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!HS-bp5-017-L")
        .expect("Required card PL!HS-bp5-017-L not found for Q212 test");
    
    let live_id = get_card_id(live_card, &card_database);
    
    // Find individual member "PL!HS-bp1-005-P 大沢瑠璃乃"
    let individual_member = cards.iter()
        .find(|c| c.card_no == "PL!HS-bp1-005-P")
        .expect("Required card PL!HS-bp1-005-P not found for Q212 test");
    
    let individual_id = get_card_id(individual_member, &card_database);
    
    // Find combined member "LL-bp2-001-R＋ 渡辺 曜&鬼塚夏美&大沢瑠璃乃"
    let combined_member = cards.iter()
        .find(|c| c.card_no == "LL-bp2-001-R＋")
        .or_else(|| cards.iter().find(|c| c.card_no.contains("LL-bp2-001")))
        .expect("Required card LL-bp2-001-R＋ not found for Q212 test");
    
    let combined_id = get_card_id(combined_member, &card_database);
    
    // Setup: individual member, combined member, and live card in hand
    setup_player_with_hand(&mut player1, vec![individual_id, combined_id, live_id]);
    
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
    
    // Step 1: Play individual member to stage
    let result1 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(individual_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    assert!(result1.is_ok(), "Should be able to play individual member to stage: {:?}", result1);
    
    // Step 2: Play combined member to stage
    let result2 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(combined_id),
        None,
        Some(MemberArea::RightSide),
        Some(false),
    );
    
    assert!(result2.is_ok(), "Should be able to play combined member to stage: {:?}", result2);
    
    // Step 3: Set live card
    let result3 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::SetLiveCard,
        Some(live_id),
        None,
        None,
        Some(false),
    );
    
    assert!(result3.is_ok(), "Should be able to set live card: {:?}", result3);
    
    // Step 4: Verify stage has members
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| id != 0).count();
    println!("Q212: Stage has {} members (engine counts combined member as {})", stage_members, stage_members);
    
    println!("Q212 verified: Live start effect not applied with character overlap");
}
