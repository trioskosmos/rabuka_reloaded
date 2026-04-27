// Test plan: tests/qa_individual/q071_area_placement_after_move.md
// Q71: When a member card is placed in an area and then moves to another zone, can you debut/place another member in that area during the same turn?
// Answer: Yes, you can.
// This is an end-to-end test that verifies area placement restriction is cleared when member leaves area.

use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};
use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_q071_area_placement_after_move() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find two member cards with affordable cost
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .filter(|c| c.cost.map_or(false, |cost| cost <= 5))
        .take(2)
        .collect();
    
    if member_cards.len() >= 2 {
        let member1_id = get_card_id(member_cards[0], &card_database);
        let member2_id = get_card_id(member_cards[1], &card_database);
        
        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
        let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        
        // Setup: member1 and member2 in hand
        setup_player_with_hand(&mut player1, vec![member1_id, member2_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids.clone());
        setup_player_with_energy(&mut player2, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Step 1: Play member1 to Center area
        let result1 = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member1_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        assert!(result1.is_ok(), "First member should play to stage: {:?}", result1);
        assert_eq!(game_state.player1.stage.get_area(MemberArea::Center), Some(member1_id),
            "First member should be in center stage");
        
        // Step 2: Send member1 to discard (simulate ability effect)
        // Since we don't have a card with an ability that sends itself to discard,
        // we'll manually move it to waitroom to simulate the effect
        let center_card = game_state.player1.stage.get_area(MemberArea::Center);
        assert_eq!(center_card, Some(member1_id), "member1 should be in center");
        
        // Move member1 from stage to waitroom (simulating ability effect)
        game_state.player1.stage.clear_area(MemberArea::Center);
        game_state.player1.waitroom.cards.push(member1_id);
        
        // Step 3: Play member2 to Center area (should succeed because area is now empty)
        let result2 = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member2_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        assert!(result2.is_ok(), "Second member should play to stage after first left: {:?}", result2);
        assert_eq!(game_state.player1.stage.get_area(MemberArea::Center), Some(member2_id),
            "Second member should be in center stage");
        
        // Verify final state
        assert!(game_state.player1.waitroom.cards.contains(&member1_id),
            "First member should be in waitroom");
        assert!(!game_state.player1.hand.cards.contains(&member2_id),
            "Second member should not be in hand (played to stage)");
        assert!(!game_state.player1.hand.cards.contains(&member1_id),
            "First member should not be in hand (in waitroom)");
        
        println!("Q071 verified: Can debut/place another member in area after first member leaves");
    } else {
        panic!("Required member cards not found for Q071 test");
    }
}
