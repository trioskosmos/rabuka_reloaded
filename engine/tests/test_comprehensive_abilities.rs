#[cfg(test)]
mod comprehensive_ability_tests {
    use rabuka_engine::game_state::GameState;
    use rabuka_engine::player::Player;
    use rabuka_engine::card_loader::CardLoader;
    use rabuka_engine::card::{Ability, AbilityEffect, Card};
    use rabuka_engine::ability::resolver::AbilityResolver;
    use std::sync::Arc;

    #[test]
    fn test_multiple_ability_scenarios() {
        println!("=== Comprehensive Ability Verification Tests ===");
        
        // Test 1: Sequential actions (draw + discard)
        test_draw_and_discard_ability();
        
        // Test 2: Look and select ability
        test_look_and_select_ability();
        
        // Test 3: Gain resource ability
        test_gain_resource_ability();
        
        // Test 4: Complex conditional ability
        test_conditional_ability();
        
        // Test 5: Duration effect
        test_duration_effect();
        
        println!("✁EAll comprehensive ability tests completed!");
    }

    fn test_draw_and_discard_ability() {
        let mut game_state = create_test_game_state();
        
        // Add cards to hand
        game_state.player1.hand.add_card(1501);
        game_state.player1.hand.add_card(1502);
        
        // Create ability: draw 2 cards, discard 1 card
        let ability = Ability {
            full_text: "テスト：カードを2枚引き、手札1枚控え室に置く".to_string(),
            cost: Some(rabuka_engine::card::AbilityCost {
                text: "手札1枚控え室に置く".to_string(),
                cost_type: Some("move_cards".to_string()),
                source: Some("hand".to_string()),
                destination: Some("discard".to_string()),
                count: Some(1),
                ..Default::default()
            }),
            effect: Some(AbilityEffect {
                text: "カードを2枚引き".to_string(),
                action: "draw_card".to_string(),
                source: Some("deck".to_string()),
                destination: Some("hand".to_string()),
                count: Some(2),
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::CONSTANT.to_string()),
            ..Default::default()
        };
        
        // Execute first effect (draw cards)
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Draw effect should succeed");
        
        // Execute second effect (discard card) - should create pending choice
        let result2 = AbilityResolver::new(&mut game_state).resolve_ability(&ability, None, 0);
        assert!(result2.is_err(), "Should need choice for discard effect");
        
        println!("✁EDraw and discard ability test passed");
    }

    fn test_look_and_select_ability() {
        let mut game_state = create_test_game_state();
        
        // Add live cards to discard
        game_state.player1.waitroom.add_card(1401); // Live card
        game_state.player1.waitroom.add_card(1402); // Another live card
        
        // Create look_and_select ability
        let ability = Ability {
            full_text: "テスト：デッキの上から3枚見て、1枚を手札に加える".to_string(),
            effect: Some(AbilityEffect {
                text: "デッキの上から3枚見る".to_string(),
                action: "look_at".to_string(),
                source: Some("deck_top".to_string()),
                count: Some(3),
                select_action: Some(Box::new(AbilityEffect {
                    text: "1枚を手札に加える".to_string(),
                    action: "move_cards".to_string(),
                    source: Some("looked_at".to_string()),
                    destination: Some("hand".to_string()),
                    count: Some(1),
                    ..Default::default()
                })),
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::CONSTANT.to_string()),
            ..Default::default()
        };
        
        // Execute look_at effect
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Look at effect should succeed");
        assert!(game_state.pending_choice.is_some(), "Should have pending choice");
        
        // Simulate user choice
        game_state.pending_choice = None;
        
        // Execute select action
        let result3 = AbilityResolver::new(&mut game_state).resolve_ability(&ability, None, 0);
        assert!(result3.is_ok(), "Select effect should succeed");
        assert!(game_state.pending_choice.is_none(), "Choice should be resolved");
        
        println!("✁ELook and select ability test passed");
    }

    fn test_gain_resource_ability() {
        let mut game_state = create_test_game_state();
        
        // Create gain resource ability
        let ability = Ability {
            full_text: "テスト：ブレードを2得る".to_string(),
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
        };
        
        let initial_blades = game_state.player1.blade;
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Gain resource should succeed");
        assert_eq!(game_state.player1.blade, initial_blades + 2, "Should gain 2 blades");
        
        println!("✁EGain resource ability test passed");
    }

    fn test_conditional_ability() {
        let mut game_state = create_test_game_state();
        
        // Create conditional ability (only if heart count > 5)
        let ability = Ability {
            full_text: "テスト：ハートが5以上の場合、ライブの合計スコアを+1する".to_string(),
            effect: Some(AbilityEffect {
                text: "ライブの合計スコアを+1する".to_string(),
                action: "set_score".to_string(),
                value: Some(1),
                target: Some("self".to_string()),
                condition: Some(rabuka_engine::card::Condition {
                    text: "ハートが5以上の場合".to_string(),
                    count: Some(5),
                    comparison_type: Some(">=".to_string()),
                    aggregate: Some("total".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::CONSTANT.to_string()),
            ..Default::default()
        };
        
        // Test ability execution
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Ability should execute");
        
        println!("✁EConditional ability test passed");
    }

    fn test_duration_effect() {
        let mut game_state = create_test_game_state();
        
        // Create duration effect (live_end: gain blades)
        let ability = Ability {
            full_text: "テスト：ライブ終了までブレードを2得る".to_string(),
            effect: Some(AbilityEffect {
                text: "ライブ終了までブレードを2得る".to_string(),
                action: "gain_resource".to_string(),
                resource: Some("blade".to_string()),
                count: Some(2),
                duration: Some("live_end".to_string()),
                target: Some("self".to_string()),
                ..Default::default()
            }),
            triggers: Some(rabuka_engine::triggers::CONSTANT.to_string()),
            ..Default::default()
        };
        
        let initial_blades = game_state.player1.blade;
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Duration effect should succeed");
        assert_eq!(game_state.player1.blade, initial_blades, "Blades should not increase immediately (duration effect)");
        
        println!("✁EDuration effect test passed");
    }

    fn create_test_game_state() -> GameState {
        let cards = CardLoader::load_cards_from_file(
            std::path::Path::new("../cards/cards.json")
        ).expect("Failed to load cards");
        let card_db = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));
        let player1 = Player::new("player1".to_string(), "Player 1".to_string(), false);
        let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        let mut game_state = GameState::new(player1, player2, card_db);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        game_state
    }
}
