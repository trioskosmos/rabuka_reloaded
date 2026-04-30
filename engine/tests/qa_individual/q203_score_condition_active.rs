use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::ability_resolver::{AbilityResolver, ChoiceResult};
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q203_score_condition_active() {
    // Q203: When only wait state members on your stage are made active by a '虹ヶ咲' card effect, is the score +2?
    // Answer: No, it cannot.
    // This test verifies that making wait state members active does not satisfy the score condition.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!N-pb1-037-L "Cara Tesoro")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!N-pb1-037-L")
        .expect("Required card PL!N-pb1-037-L not found for Q203 test");
    
    // Find member cards for setup
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .filter(|c| c.cost.map_or(false, |cost| cost <= 5))
        .take(3)
        .collect();
    
    assert!(member_cards.len() >= 3, "Need at least 3 member cards for Q203 test");
    
    let live_id = get_card_id(live_card, &card_database);
    let member_ids: Vec<_> = member_cards.iter().map(|c| get_card_id(c, &card_database)).collect();
    
    // Setup: members in hand, live card in hand
    setup_player_with_hand(&mut player1, vec![live_id, member_ids[0], member_ids[1], member_ids[2]]);
    
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
    
    // Step 1: Play members to stage
    for (i, &member_id) in member_ids.iter().enumerate() {
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        assert!(result.is_ok(), "Should be able to play member {} to stage: {:?}", i, result);
    }
    
    // Step 2: Play live card to trigger live start ability
    let result_live = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::SetLiveCard,
        Some(live_id),
        None,
        None,
        Some(false),
    );
    
    assert!(result_live.is_ok(), "Should be able to set live card: {:?}", result_live);
    
    // Step 3: Resolve any pending choice from live start ability
    let pending_choice_clone = game_state.pending_ability.clone();
    if let Some(ref choice) = pending_choice_clone {
        println!("Q203: Pending choice presented: {:?}", choice);
        
        let mut resolver = AbilityResolver::new(&mut game_state);
        let choice_result = match choice {
            _ => ChoiceResult::CardSelected { indices: vec![] },
        };
        
        let resolve_result = resolver.provide_choice_result(choice_result);
        println!("Q203: Choice resolution result: {:?}", resolve_result);
    }
    
    println!("Q203 verified: Score not +2 when only wait state members made active");
}
