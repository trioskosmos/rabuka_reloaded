#[cfg(test)]
mod real_gameplay_tests {
    use crate::game_state::GameState;
    use crate::player::Player;
    use crate::card_loader;
    use crate::card::{Ability, AbilityEffect};
    use crate::ability::executor::AbilityExecutor;
    use crate::turn::{handle_main_phase_action, Action};
    use crate::game_setup::{generate_possible_actions, setup_game_from_decks};

    #[test]
    fn test_ruby_ability_in_real_gameplay() {
        println!("=== REAL GAMEPLAY TEST: Ruby Ability ===");
        
        // Setup actual game with real decks
        let mut game_state = setup_real_game();
        let mut executor = AbilityExecutor::new();
        
        // Phase 1: Setup Ruby on stage with live cards in discard
        println!("📋 Phase 1: Setup");
        setup_ruby_scenario(&mut game_state);
        
        // Verify initial state
        assert!(game_state.player1.stage.stage.contains(&1392), "Ruby should be on stage");
        let live_cards_in_discard = count_live_cards(&game_state.player1.waitroom.cards, &game_state.card_database);
        assert!(live_cards_in_discard >= 1, "Should have live cards in discard");
        
        println!("✅ Ruby on stage: {}", game_state.player1.stage.stage[0]);
        println!("✅ Live cards in discard: {}", live_cards_in_discard);
        
        // Phase 2: Generate possible actions (should include Ruby's ability)
        println!("\n📋 Phase 2: Generate Actions");
        let actions = generate_possible_actions(&game_state, "player1");
        
        let ruby_ability_action = actions.iter().find(|action| {
            matches!(action, Action::UseAbility { card_index: Some(0), .. })
        });
        
        assert!(ruby_ability_action.is_some(), "Ruby's ability should be available");
        println!("✅ Ruby's ability found in possible actions");
        
        // Phase 3: Execute Ruby's ability
        println!("\n📋 Phase 3: Execute Ability");
        if let Some(Action::UseAbility { card_index: Some(0), .. }) = ruby_ability_action {
            let result = handle_main_phase_action(&mut game_state, ruby_ability_action.unwrap());
            
            match result {
                Ok(_) => {
                    println!("✅ Ability execution started successfully");
                    
                    // Phase 4: Check if choice was created
                    println!("\n📋 Phase 4: Check Choice System");
                    assert!(game_state.pending_ability.is_some(), "Should have pending ability");
                    assert!(game_state.pending_choice.is_some(), "Should have pending choice");
                    
                    println!("✅ Pending ability created");
                    println!("✅ Choice system activated");
                    
                    // Phase 5: Simulate user choice
                    println!("\n📋 Phase 5: Simulate User Choice");
                    simulate_user_choice(&mut game_state, &mut executor);
                    
                    // Phase 6: Verify final state
                    println!("\n📋 Phase 6: Verify Final State");
                    verify_ruby_ability_result(&game_state);
                    
                    println!("🎉 RUBY ABILITY TEST PASSED!");
                }
                Err(e) => {
                    panic!("❌ Ability execution failed: {}", e);
                }
            }
        }
    }

    #[test] 
    fn test_multiple_ability_types_in_gameplay() {
        println!("=== REAL GAMEPLAY TEST: Multiple Ability Types ===");
        
        let mut game_state = setup_real_game();
        
        // Test 1: Draw ability
        test_draw_ability_in_gameplay(&mut game_state);
        
        // Test 2: Resource gain ability  
        test_resource_gain_ability_in_gameplay(&mut game_state);
        
        // Test 3: Sequential ability
        test_sequential_ability_in_gameplay(&mut game_state);
        
        println!("🎉 MULTIPLE ABILITY TESTS PASSED!");
    }

    fn setup_real_game() -> GameState {
        let card_db = card_loader::load_card_database();
        let mut game_state = GameState::new(1, 2, card_db);
        game_state.current_phase = crate::game_state::Phase::Main;
        game_state.turn_number = 1;
        game_state.player1.id = "player1".to_string();
        game_state.player2.id = "player2".to_string();
        
        // Add some cards to hand for testing
        for i in 1000..1010 {
            game_state.player1.hand.add_card(i);
        }
        
        game_state
    }

    fn setup_ruby_scenario(game_state: &mut GameState) {
        // Clear stage
        game_state.player1.stage.stage = [-1, -1, -1];
        
        // Place Ruby on stage
        game_state.player1.stage.stage[0] = 1392;
        
        // Add live cards to discard
        game_state.player1.waitroom.add_card(1401); // Live card 1
        game_state.player1.waitroom.add_card(1402); // Live card 2  
        game_state.player1.waitroom.add_card(1403); // Live card 3
    }

    fn count_live_cards(card_ids: &[i16], card_db: &crate::card::CardDatabase) -> usize {
        card_ids.iter().filter(|&&id| {
            if let Some(card) = card_db.get_card(id) {
                card.is_live()
            } else {
                false
            }
        }).count()
    }

    fn simulate_user_choice(game_state: &mut GameState, executor: &mut AbilityExecutor) {
        // Get available choices
        if let Some(choice) = &game_state.pending_choice {
            println!("Available choice: {:?}", choice);
            
            // Simulate selecting first live card
            let choice_result = crate::ability::executor::ChoiceResult::CardSelected { indices: vec![0] };
            
            // Process the choice
            let result = executor.provide_choice_result(choice_result);
            assert!(result.is_ok(), "Choice should be accepted");
            
            // Resume ability execution
            if let Some(pending) = game_state.pending_ability.clone() {
                game_state.execute_card_ability(&pending.card_no, &pending.player_id);
            }
            
            println!("✅ User choice processed");
        }
    }

    fn verify_ruby_ability_result(game_state: &GameState) {
        // Ruby should be in discard
        assert!(game_state.player1.waitroom.cards.contains(&1392), "Ruby should be in discard");
        println!("✅ Ruby in discard: {}", game_state.player1.waitroom.cards.contains(&1392));
        
        // Ruby should not be on stage
        assert!(!game_state.player1.stage.stage.contains(&1392), "Ruby should not be on stage");
        println!("✅ Ruby not on stage: {}", !game_state.player1.stage.stage.contains(&1392));
        
        // Should have live card in hand
        let live_cards_in_hand = count_live_cards(&game_state.player1.hand.cards, &game_state.card_database);
        assert!(live_cards_in_hand >= 1, "Should have live card in hand");
        println!("✅ Live cards in hand: {}", live_cards_in_hand);
        
        // Choice should be resolved
        assert!(game_state.pending_choice.is_none(), "Choice should be resolved");
        println!("✅ Choice system resolved");
        
        // Ability should be completed
        assert!(game_state.pending_ability.is_none(), "Ability should be completed");
        println!("✅ Ability execution completed");
    }

    fn test_draw_ability_in_gameplay(game_state: &mut GameState) {
        println!("\n🎯 Testing Draw Ability");
        
        // Create a simple draw ability
        let draw_ability = create_draw_ability();
        
        // Execute ability
        let mut executor = AbilityExecutor::new();
        let initial_hand_size = game_state.player1.hand.cards.len();
        
        let result = executor.resolve_ability(&draw_ability, None, 0);
        assert!(result.is_ok(), "Draw ability should execute");
        
        let final_hand_size = game_state.player1.hand.cards.len();
        assert!(final_hand_size > initial_hand_size, "Should have drawn cards");
        
        println!("✅ Draw ability: {} -> {} cards", initial_hand_size, final_hand_size);
    }

    fn test_resource_gain_ability_in_gameplay(game_state: &mut GameState) {
        println!("\n🎯 Testing Resource Gain Ability");
        
        let resource_ability = create_resource_gain_ability();
        let mut executor = AbilityExecutor::new();
        
        let initial_blades = game_state.player1.blade_count;
        
        let result = executor.resolve_ability(&resource_ability, None, 0);
        assert!(result.is_ok(), "Resource gain ability should execute");
        
        let final_blades = game_state.player1.blade_count;
        assert!(final_blades > initial_blades, "Should have gained blades");
        
        println!("✅ Resource gain: {} -> {} blades", initial_blades, final_blades);
    }

    fn test_sequential_ability_in_gameplay(game_state: &mut GameState) {
        println!("\n🎯 Testing Sequential Ability");
        
        let sequential_ability = create_sequential_ability();
        let mut executor = AbilityExecutor::new();
        
        let initial_hand_size = game_state.player1.hand.cards.len();
        let initial_discard_size = game_state.player1.waitroom.cards.len();
        
        let result = executor.resolve_ability(&sequential_ability, None, 0);
        assert!(result.is_ok(), "Sequential ability should execute");
        
        let final_hand_size = game_state.player1.hand.cards.len();
        let final_discard_size = game_state.player1.waitroom.cards.len();
        
        // Should have drawn cards and discarded cards
        assert!(final_hand_size > initial_hand_size, "Should have drawn cards");
        assert!(final_discard_size > initial_discard_size, "Should have discarded cards");
        
        println!("✅ Sequential: Hand {}->{}, Discard {}->{}", 
            initial_hand_size, final_hand_size, initial_discard_size, final_discard_size);
    }

    fn create_draw_ability() -> Ability {
        Ability {
            full_text: "カードを2枚引く".to_string(),
            effect: Some(AbilityEffect {
                text: "カードを2枚引く".to_string(),
                action: "draw_card".to_string(),
                source: Some("deck".to_string()),
                destination: Some("hand".to_string()),
                count: Some(2),
            }),
            triggers: Some("常時".to_string()),
            ..Default::default()
        }
    }

    fn create_resource_gain_ability() -> Ability {
        Ability {
            full_text: "ブレードを2得る".to_string(),
            effect: Some(AbilityEffect {
                text: "ブレードを2得る".to_string(),
                action: "gain_resource".to_string(),
                resource: Some("blade".to_string()),
                count: Some(2),
                target: Some("self".to_string()),
            }),
            triggers: Some("常時".to_string()),
            ..Default::default()
        }
    }

    fn create_sequential_ability() -> Ability {
        Ability {
            full_text: "カードを2枚引き、手札を1枚控え室に置く".to_string(),
            effect: Some(AbilityEffect {
                text: "カードを2枚引き、手札を1枚控え室に置く".to_string(),
                action: "sequential".to_string(),
                actions: Some(vec![
                    AbilityEffect {
                        text: "カードを2枚引く".to_string(),
                        action: "draw_card".to_string(),
                        source: Some("deck".to_string()),
                        destination: Some("hand".to_string()),
                        count: Some(2),
                    },
                    AbilityEffect {
                        text: "手札を1枚控え室に置く".to_string(),
                        action: "move_cards".to_string(),
                        source: Some("hand".to_string()),
                        destination: Some("discard".to_string()),
                        count: Some(1),
                    }
                ]),
            }),
            triggers: Some("常時".to_string()),
            ..Default::default()
        }
    }
}
