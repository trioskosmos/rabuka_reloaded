#[cfg(test)]
mod real_gameplay_tests {
    use rabuka_engine::game_state::GameState;
    use rabuka_engine::player::Player;
    use rabuka_engine::card_loader::CardLoader;
    use rabuka_engine::card::{Ability, AbilityEffect};
    use rabuka_engine::ability::resolver::AbilityResolver;
    use std::sync::Arc;

    #[test]
    fn test_ruby_ability_in_real_gameplay() {
        println!("=== REAL GAMEPLAY TEST: Ruby Ability ===");
        
        // Setup actual game with real decks
        let mut game_state = setup_real_game();
        
        // Phase 1: Setup Ruby on stage with live cards in discard
        println!("📋 Phase 1: Setup");
        setup_ruby_scenario(&mut game_state);
        
        // Verify initial state
        assert!(game_state.player1.stage.stage.contains(&1392), "Ruby should be on stage");
        let live_cards_in_discard = count_live_cards(&game_state.player1.waitroom.cards, &game_state.card_database);
        assert!(live_cards_in_discard >= 1, "Should have live cards in discard");
        
        println!("✁ERuby on stage: {}", game_state.player1.stage.stage[0]);
        println!("✁ELive cards in discard: {}", live_cards_in_discard);
        
        // Phase 2: Execute Ruby's ability directly
        println!("\n📋 Phase 2: Execute Ruby Ability");
        let ruby_ability = create_ruby_ability();
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&ruby_ability, Some(1392), 0);
        
        match result {
            Ok(_) => {
                println!("✁EAbility execution started successfully");
                
                // Phase 3: Verify final state
                println!("\n📋 Phase 3: Verify Final State");
                verify_ruby_ability_result(&game_state);
                
                println!("🎉 RUBY ABILITY TEST PASSED!");
            }
            Err(e) => {
                panic!("❁EAbility execution failed: {}", e);
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
        let cards = CardLoader::load_cards_from_file(
            std::path::Path::new("../cards/cards.json")
        ).expect("Failed to load cards");
        let card_db = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));
        let player1 = Player::new("player1".to_string(), "Player 1".to_string(), false);
        let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        let mut game_state = GameState::new(player1, player2, card_db);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
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

    fn count_live_cards(card_ids: &[i16], card_db: &rabuka_engine::card::CardDatabase) -> usize {
        card_ids.iter().filter(|&&id| {
            if let Some(card) = card_db.get_card(id) {
                card.is_live()
            } else {
                false
            }
        }).count()
    }

    fn create_ruby_ability() -> Ability {
        Ability {
            full_text: "起動このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える".to_string(),
            effect: Some(AbilityEffect {
                text: "自分の控え室からライブカードを1枚手札に加える".to_string(),
                action: "move_cards".to_string(),
                source: Some("discard".to_string()),
                destination: Some("hand".to_string()),
                count: Some(1),
                card_type: Some("live_card".to_string()),
                target: Some("self".to_string()),
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::ACTIVATION.to_string()),
            ..Default::default()
        }
    }

    fn verify_ruby_ability_result(game_state: &GameState) {
        // Ruby should be in discard
        assert!(game_state.player1.waitroom.cards.contains(&1392), "Ruby should be in discard");
        println!("✁ERuby in discard: {}", game_state.player1.waitroom.cards.contains(&1392));
        
        // Ruby should not be on stage
        assert!(!game_state.player1.stage.stage.contains(&1392), "Ruby should not be on stage");
        println!("✁ERuby not on stage: {}", !game_state.player1.stage.stage.contains(&1392));
        
        // Should have live card in hand
        let live_cards_in_hand = count_live_cards(&game_state.player1.hand.cards, &game_state.card_database);
        assert!(live_cards_in_hand >= 1, "Should have live card in hand");
        println!("✁ELive cards in hand: {}", live_cards_in_hand);
        
        // Choice should be resolved
        assert!(game_state.pending_choice.is_none(), "Choice should be resolved");
        println!("✁EChoice system resolved");
        
        // Ability should be completed
        assert!(game_state.pending_ability.is_none(), "Ability should be completed");
        println!("✁EAbility execution completed");
    }

    fn test_draw_ability_in_gameplay(game_state: &mut GameState) {
        println!("\n🎯 Testing Draw Ability");
        
        // Create a simple draw ability
        let draw_ability = create_draw_ability();
        
        // Execute ability
        let initial_hand_size = game_state.player1.hand.cards.len();
        
        let result = {
            let mut resolver = AbilityResolver::new(game_state);
            let r = resolver.resolve_ability(&draw_ability, None, 0);
            drop(resolver);
            r
        };
        assert!(result.is_ok(), "Draw ability should execute");
        
        let final_hand_size = game_state.player1.hand.cards.len();
        assert!(final_hand_size > initial_hand_size, "Should have drawn cards");
        
        println!("✁EDraw ability: {} -> {} cards", initial_hand_size, final_hand_size);
    }

    fn test_resource_gain_ability_in_gameplay(game_state: &mut GameState) {
        println!("\n🎯 Testing Resource Gain Ability");
        
        let resource_ability = create_resource_gain_ability();
        
        let initial_blades = game_state.player1.blade;
        
        let result = {
            let mut resolver = AbilityResolver::new(game_state);
            let r = resolver.resolve_ability(&resource_ability, None, 0);
            drop(resolver);
            r
        };
        assert!(result.is_ok(), "Resource gain ability should execute");
        
        let final_blades = game_state.player1.blade;
        assert!(final_blades > initial_blades, "Should have gained blades");
        
        println!("✁EResource gain: {} -> {} blades", initial_blades, final_blades);
    }

    fn test_sequential_ability_in_gameplay(game_state: &mut GameState) {
        println!("\n🎯 Testing Sequential Ability");
        
        let sequential_ability = create_sequential_ability();
        
        let initial_hand_size = game_state.player1.hand.cards.len();
        let initial_discard_size = game_state.player1.waitroom.cards.len();
        
        let result = {
            let mut resolver = AbilityResolver::new(game_state);
            let r = resolver.resolve_ability(&sequential_ability, None, 0);
            drop(resolver);
            r
        };
        assert!(result.is_ok(), "Sequential ability should execute");
        
        let final_hand_size = game_state.player1.hand.cards.len();
        let final_discard_size = game_state.player1.waitroom.cards.len();
        
        // Should have drawn cards and discarded cards
        assert!(final_hand_size > initial_hand_size, "Should have drawn cards");
        assert!(final_discard_size > initial_discard_size, "Should have discarded cards");
        
        println!("✁ESequential: Hand {}->{}, Discard {}->{}", 
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
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::CONSTANT.to_string()),
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
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::CONSTANT.to_string()),
            ..Default::default()
        }
    }

    fn create_sequential_ability() -> Ability {
        Ability {
            full_text: "カードを2枚引き、手札1枚控え室に置く".to_string(),
            effect: Some(AbilityEffect {
                text: "カードを2枚引き、手札1枚控え室に置く".to_string(),
                action: "sequential".to_string(),
                actions: Some(vec![
                    AbilityEffect {
                        text: "カードを2枚引く".to_string(),
                        action: "draw_card".to_string(),
                        source: Some("deck".to_string()),
                        destination: Some("hand".to_string()),
                        count: Some(2),
                        ..Default::default()
                    },
                    AbilityEffect {
                        text: "手札1枚控え室に置く".to_string(),
                        action: "move_cards".to_string(),
                        source: Some("hand".to_string()),
                        destination: Some("discard".to_string()),
                        count: Some(1),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::CONSTANT.to_string()),
            ..Default::default()
        }
    }
}
