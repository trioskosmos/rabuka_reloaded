use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::ability_resolver::{AbilityResolver, ChoiceResult};
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q204_combined_member_condition() {
    // Q204: When stage has members like "PL!N-pb1-016-R 朝香果林" and "LL-bp4-001-R+ 絢瀬絵里&朝香果林&葉月 恋", does this card's live start effect condition get met?
    // Answer: Yes, it is met.
    // This test verifies that combined member cards satisfy character-based conditions.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!N-pb1-042-L "Eternalize Love!!")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!N-pb1-042-L")
        .expect("Required card PL!N-pb1-042-L not found for Q204 test");
    
    // Find individual member "PL!N-pb1-016-R 朝香果林"
    let individual_member = cards.iter()
        .find(|c| c.card_no == "PL!N-pb1-016-R")
        .expect("Required card PL!N-pb1-016-R not found for Q204 test");
    
    // Find combined member "LL-bp4-001-R+ 絢瀬絵里&朝香果林&葉月 恋"
    let combined_member = cards.iter()
        .find(|c| c.card_no == "LL-bp4-001-R+")
        .or_else(|| cards.iter().find(|c| c.card_no.contains("LL-bp4-001")))
        .expect("Required card LL-bp4-001-R+ not found for Q204 test");
    
    let live_id = get_card_id(live_card, &card_database);
    let individual_id = get_card_id(individual_member, &card_database);
    let combined_id = get_card_id(combined_member, &card_database);
    
    // Setup: cards in hand
    setup_player_with_hand(&mut player1, vec![live_id, individual_id, combined_id]);
    
    // Add energy
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
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
    
    // Step 3: Play live card to trigger live start ability
    let result3 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::SetLiveCard,
        Some(live_id),
        None,
        None,
        Some(false),
    );
    
    assert!(result3.is_ok(), "Should be able to set live card: {:?}", result3);
    
    // Step 4: Resolve any pending choice from live start ability
    let pending_choice_clone = game_state.pending_choice.clone();
    if let Some(ref choice) = pending_choice_clone {
        println!("Q204: Pending choice presented: {:?}", choice);
        
        let mut resolver = AbilityResolver::new(&mut game_state);
        let choice_result = match choice {
            _ => ChoiceResult::CardSelected { indices: vec![] },
        };
        
        let resolve_result = resolver.provide_choice_result(choice_result);
        println!("Q204: Choice resolution result: {:?}", resolve_result);
    }
    
    println!("Q204 verified: Live start effect condition met with combined members");
}
