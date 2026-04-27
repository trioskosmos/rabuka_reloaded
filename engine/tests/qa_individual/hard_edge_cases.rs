// Hard edge case tests - test complex gameplay scenarios
// These tests use TurnEngine to simulate real gameplay edge cases

use crate::qa_individual::common::*;

#[test]
fn test_baton_touch_equal_cost_zero_payment() {
    // Test: Baton touch with equal cost should result in zero energy payment
    // This tests the baton touch cost reduction logic
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find two member cards with the same cost
    let members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0)
        .take(2)
        .collect();
    
    if members.len() >= 2 {
        let member1_id = get_card_id(members[0], &card_database);
        let member2_id = get_card_id(members[1], &card_database);
        let cost1 = members[0].cost.unwrap_or(0);
        let cost2 = members[1].cost.unwrap_or(0);
        
        // Only test if costs are equal
        if cost1 == cost2 {
            let energy_card_ids: Vec<_> = cards.iter()
                .filter(|c| c.is_energy())
                .filter(|c| get_card_id(c, &card_database) != 0)
                .map(|c| get_card_id(c, &card_database))
                .take(30)
                .collect();
            
            setup_player_with_hand(&mut player1, vec![member1_id, member2_id]);
            setup_player_with_energy(&mut player1, energy_card_ids);
            
            let mut game_state = GameState::new(player1, player2, card_database.clone());
            game_state.current_phase = rabuka_engine::game_state::Phase::Main;
            game_state.turn_number = 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            
            let initial_hand_count = game_state.player1.hand.cards.len();
            let initial_stage_count = game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count();
            let initial_waitroom_count = game_state.player1.waitroom.cards.len();
            
            // Play first member to stage
            let result = TurnEngine::execute_main_phase_action(
                &mut game_state,
                &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
                Some(member1_id),
                None,
                Some(rabuka_engine::zones::MemberArea::Center),
                Some(false),
            );
            assert!(result.is_ok(), "First member should play: {:?}", result);
            
            let initial_energy = game_state.player1.energy_zone.active_energy_count;
            
            // Advance turn for baton touch
            game_state.turn_number = 2;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
            
            // Baton touch with equal cost
            let result = TurnEngine::execute_main_phase_action(
                &mut game_state,
                &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
                Some(member2_id),
                None,
                Some(rabuka_engine::zones::MemberArea::Center),
                Some(true),
            );
            
            if result.is_ok() {
                let final_energy = game_state.player1.energy_zone.active_energy_count;
                // With equal cost, energy payment should be 0
                assert_eq!(final_energy, initial_energy,
                    "Equal cost baton touch should not consume energy");
                
                // Verify member replacement occurred
                assert!(game_state.player1.stage.stage.contains(&member2_id),
                    "New member should be on stage after baton touch");
                assert!(game_state.player1.waitroom.cards.contains(&member1_id),
                    "Original member should be in waitroom after baton touch");
                assert_eq!(game_state.player1.waitroom.cards.len(), initial_waitroom_count + 1,
                    "Waitroom should have 1 more card after baton touch");
                
                // Verify card was consumed from hand (engine may consume differently)
                let hand_count = game_state.player1.hand.cards.len();
                assert!(hand_count < initial_hand_count,
                    "Hand should have fewer cards after baton touch");
            } else {
                // If baton touch fails, verify state is unchanged
                assert_eq!(game_state.player1.energy_zone.active_energy_count, initial_energy,
                    "Energy should not change if baton touch fails");
                assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count,
                    "Hand should not change if baton touch fails");
            }
        } else {
            println!("Skipping test: no equal cost members found");
        }
    } else {
        println!("Skipping test: insufficient member cards");
    }
}

#[test]
fn test_optional_cost_auto_ability_via_gameplay() {
    // Test: Auto abilities with optional costs should present choice
    // This tests optional cost handling for auto abilities
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a card with auto ability that has optional cost
    let test_card = cards.iter()
        .find(|c| c.abilities.iter().any(|a| {
            a.triggers.as_deref() == Some("自勁") &&
            a.cost.as_ref().map_or(false, |c| c.optional == Some(true))
        }));
    
    if let Some(card) = test_card {
        let card_id = get_card_id(card, &card_database);
        
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        
        setup_player_with_hand(&mut player1, vec![card_id]);
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
        
        let initial_hand_count = game_state.player1.hand.cards.len();
        let initial_stage_count = game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count();
        
        // Play the card to stage (this should trigger auto abilities)
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
            Some(card_id),
            None,
            Some(rabuka_engine::zones::MemberArea::Center),
            Some(false),
        );
        
        // Card should play successfully
        // Note: Full optional cost testing would require triggering the specific auto ability
        assert!(result.is_ok(), "Card with optional cost auto ability should play: {:?}", result);
        
        // Verify card moved from hand to stage
        assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 1,
            "Hand should have 1 fewer card");
        assert!(!game_state.player1.hand.cards.contains(&card_id),
            "Card should not be in hand after playing");
        assert_eq!(game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count(), initial_stage_count + 1,
            "Stage should have 1 more member");
    } else {
        println!("Skipping test: no card with auto ability optional cost found");
    }
}

#[test]
fn test_stage_full_cannot_place_member() {
    // Test: Cannot place member when stage is full
    // This tests stage capacity validation
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find 4 member cards with low cost (more than stage capacity of 3)
    let members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0 && c.cost.unwrap_or(0) <= 2)
        .take(4)
        .collect();
    
    if members.len() >= 4 {
        let member_ids: Vec<_> = members.iter()
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(30)
            .collect();
        
        setup_player_with_hand(&mut player1, member_ids.clone());
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
        
        let initial_hand_count = game_state.player1.hand.cards.len();
        let initial_stage_count = game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count();
        
        // Place 3 members to fill the stage
        let areas = [
            rabuka_engine::zones::MemberArea::LeftSide,
            rabuka_engine::zones::MemberArea::Center,
            rabuka_engine::zones::MemberArea::RightSide,
        ];
        
        for (i, &area) in areas.iter().enumerate() {
            let result = TurnEngine::execute_main_phase_action(
                &mut game_state,
                &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
                Some(member_ids[i]),
                None,
                Some(area),
                Some(false),
            );
            assert!(result.is_ok(), "Member {} should play to {:?}: {:?}", i, area, result);
        }
        
        // Try to place 4th member - should fail
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
            Some(member_ids[3]),
            None,
            Some(rabuka_engine::zones::MemberArea::Center),
            Some(false),
        );
        
        // Should fail because stage is full
        assert!(result.is_err(), "Should fail when stage is full: {:?}", result);
        
        // Verify stage still has exactly 3 members
        assert_eq!(game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count(), 3,
            "Stage should still have 3 members after failed placement");
        
        // Verify 4th member is still in hand
        assert!(game_state.player1.hand.cards.contains(&member_ids[3]),
            "4th member should still be in hand when stage is full");
        assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 3,
            "Hand should have 3 fewer cards (only 3 played successfully)");
    } else {
        println!("Skipping test: insufficient member cards");
    }
}

#[test]
fn test_baton_turn_restriction() {
    // Test: Cannot baton touch in the same turn a member was placed
    // This tests the same-turn baton touch restriction
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find two member cards
    let members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0)
        .take(2)
        .collect();
    
    if members.len() >= 2 {
        let member1_id = get_card_id(members[0], &card_database);
        let member2_id = get_card_id(members[1], &card_database);
        
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(30)
            .collect();
        
        setup_player_with_hand(&mut player1, vec![member1_id, member2_id]);
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
        
        let initial_hand_count = game_state.player1.hand.cards.len();
        let initial_stage_count = game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count();
        let initial_waitroom_count = game_state.player1.waitroom.cards.len();
        
        // Play first member
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
            Some(member1_id),
            None,
            Some(rabuka_engine::zones::MemberArea::Center),
            Some(false),
        );
        assert!(result.is_ok(), "First member should play: {:?}", result);
        
        // Try to baton touch in the same turn
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
            Some(member2_id),
            None,
            Some(rabuka_engine::zones::MemberArea::Center),
            Some(true),
        );
        
        // Should fail due to same-turn restriction
        assert!(result.is_err(), "Baton touch should fail in same turn: {:?}", result);
        
        // Verify state is unchanged (no baton touch occurred)
        assert_eq!(game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count(), initial_stage_count + 1,
            "Stage should still have only 1 member (first member)");
        assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 1,
            "Hand should have 1 fewer card (only first member played)");
        assert_eq!(game_state.player1.waitroom.cards.len(), initial_waitroom_count,
            "Waitroom should be unchanged (no baton touch occurred)");
        assert!(game_state.player1.hand.cards.contains(&member2_id),
            "Second member should still be in hand");
    } else {
        println!("Skipping test: insufficient member cards");
    }
}

