#[cfg(test)]
mod comprehensive_ability_tests {
    use crate::game_state::GameState;
    use crate::player::Player;
    use crate::card_loader;
    use crate::card::{Ability, AbilityEffect, Card};
    use crate::ability::executor::AbilityExecutor;

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
        
        println!("✅ All comprehensive ability tests completed!");
    }

    fn test_draw_and_discard_ability() {
        let mut game_state = create_test_game_state();
        let mut executor = AbilityExecutor::new();
        
        // Add cards to hand
        game_state.player1.hand.add_card(1501);
        game_state.player1.hand.add_card(1502);
        
        // Create ability: draw 2 cards, discard 1 card
        let ability = Ability {
            full_text: "テスト：カードを2枚引き、手札を1枚控え室に置く".to_string(),
            cost: Some(crate::card::AbilityCost {
                text: "手札を1枚控え室に置く".to_string(),
                cost_type: Some("move_cards".to_string()),
                source: Some("hand".to_string()),
                destination: Some("discard".to_string()),
                count: Some(1),
                r#type: Some("move_cards".to_string()),
            }),
            effect: Some(AbilityEffect {
                text: "カードを2枚引き".to_string(),
                action: "draw_card".to_string(),
                source: Some("deck".to_string()),
                destination: Some("hand".to_string()),
                count: Some(2),
            })),
            triggers: Some("常時".to_string()),
            ..Default::default()
        };
        
        // Execute first effect (draw cards)
        let result = executor.resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Draw effect should succeed");
        
        // Execute second effect (discard card) - should create pending choice
        let result2 = executor.resolve_ability(&ability, None, 0);
        assert!(result2.is_err(), "Should need choice for discard effect");
        
        println!("✅ Draw and discard ability test passed");
    }

    fn test_look_and_select_ability() {
        let mut game_state = create_test_game_state();
        let mut executor = AbilityExecutor::new();
        
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
            })),
            select_action: Some(AbilityEffect {
                text: "1枚を手札に加える".to_string(),
                action: "move_cards".to_string(),
                source: Some("looked_at".to_string()),
                destination: Some("hand".to_string()),
                count: Some(1),
            })),
            triggers: Some("常時".to_string()),
            ..Default::default()
        };
        
        // Execute look_at effect
        let result = executor.resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Look at effect should succeed");
        assert!(!executor.get_pending_choice().is_none(), "Should have pending choice");
        
        // Simulate user choice
        let choice_result = crate::ability::executor::ChoiceResult::CardSelected { indices: vec![0] };
        let result2 = executor.provide_choice_result(choice_result);
        assert!(result2.is_ok(), "Choice should be accepted");
        
        // Execute select action
        let result3 = executor.resolve_ability(&ability, None, 0);
        assert!(result3.is_ok(), "Select effect should succeed");
        assert!(executor.get_pending_choice().is_none(), "Choice should be resolved");
        
        println!("✅ Look and select ability test passed");
    }

    fn test_gain_resource_ability() {
        let mut game_state = create_test_game_state();
        let mut executor = AbilityExecutor::new();
        
        // Create gain resource ability
        let ability = Ability {
            full_text: "テスト：ブレードを2得る".to_string(),
            effect: Some(AbilityEffect {
                text: "ブレードを2得る".to_string(),
                action: "gain_resource".to_string(),
                resource: Some("blade".to_string()),
                count: Some(2),
                target: Some("self".to_string()),
            })),
            triggers: Some("常時".to_string()),
            ..Default::default()
        };
        
        let initial_blades = game_state.player1.blade_count;
        let result = executor.resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Gain resource should succeed");
        assert_eq!(game_state.player1.blade_count, initial_blades + 2, "Should gain 2 blades");
        
        println!("✅ Gain resource ability test passed");
    }

    fn test_conditional_ability() {
        let mut game_state = create_test_game_state();
        let mut executor = AbilityExecutor::new();
        
        // Create conditional ability (only if heart count > 5)
        let ability = Ability {
            full_text: "テスト：ハートが5以上の場合、ライブの合計スコアを＋1する".to_string(),
            effect: Some(AbilityEffect {
                text: "ライブの合計スコアを＋1する".to_string(),
                action: "set_score".to_string(),
                value: Some(1),
                target: Some("self".to_string()),
                condition: Some(crate::card::Condition {
                    text: "ハートが5以上の場合".to_string(),
                    r#type: "card_count_condition".to_string(),
                    count: Some(5),
                    comparison_type: Some(">=".to_string()),
                    aggregate: Some("total".to_string()),
                }),
            })),
            triggers: Some("常時".to_string()),
            ..Default::default()
        };
        
        // Test when condition not met (hearts < 5)
        game_state.player1.heart_count = 3;
        let result = executor.resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Should execute without effect when condition not met");
        assert_eq!(game_state.score, 0, "Score should not change when condition not met");
        
        // Test when condition is met (hearts >= 5)
        game_state.player1.heart_count = 6;
        let result2 = executor.resolve_ability(&ability, None, 0);
        assert!(result2.is_ok(), "Should execute with effect when condition met");
        assert_eq!(game_state.score, 1, "Score should increase by 1 when condition met");
        
        println!("✅ Conditional ability test passed");
    }

    fn test_duration_effect() {
        let mut game_state = create_test_game_state();
        let mut executor = AbilityExecutor::new();
        
        // Create duration effect (live_end: gain blades)
        let ability = Ability {
            full_text: "テスト：ライブ終了時までブレードを2得る".to_string(),
            effect: Some(AbilityEffect {
                text: "ライブ終了時までブレードを2得る".to_string(),
                action: "gain_resource".to_string(),
                resource: Some("blade".to_string()),
                count: Some(2),
                duration: Some("live_end".to_string()),
                target: Some("self".to_string()),
            })),
            triggers: Some("常時".to_string()),
            ..Default::default()
        };
        
        let initial_blades = game_state.player1.blade_count;
        let result = executor.resolve_ability(&ability, None, 0);
        assert!(result.is_ok(), "Duration effect should succeed");
        assert_eq!(game_state.player1.blade_count, initial_blades, "Blades should not increase immediately (duration effect)");
        
        println!("✅ Duration effect test passed");
    }

    fn create_test_game_state() -> GameState {
        let card_db = card_loader::load_card_database();
        let mut game_state = GameState::new(1, 2, card_db);
        game_state.current_phase = crate::game_state::Phase::Main;
        game_state.turn_number = 1;
        game_state.player1.id = "test_player".to_string();
        game_state.player2.id = "test_opponent".to_string();
        game_state
    }
}
