#[cfg(test)]
mod integration_tests {
    use crate::game_state::GameState;
    use crate::player::Player;
    use crate::card_loader;
    use crate::card::{Ability, AbilityEffect};
    use crate::ability::executor::AbilityExecutor;

    #[test]
    fn test_full_ability_integration() {
        println!("=== Full Ability Integration Test ===");
        
        // Create realistic game scenario
        let mut game_state = GameState::new(1, 2, card_loader::load_card_database());
        game_state.current_phase = crate::game_state::Phase::Main;
        game_state.turn_number = 1;
        game_state.player1.id = "player1".to_string();
        game_state.player2.id = "player2".to_string();
        
        // Setup: Ruby on stage, multiple live cards in discard
        let ruby_id = 1392;
        game_state.player1.stage.stage[0] = ruby_id;
        game_state.player1.waitroom.add_card(1401); // Live card 1
        game_state.player1.waitroom.add_card(1402); // Live card 2
        game_state.player1.waitroom.add_card(1403); // Live card 3
        
        println!("Initial state:");
        println!("  Ruby on stage: {}", ruby_id);
        println!("  Live cards in discard: {:?}", game_state.player1.waitroom.cards);
        
        // Execute Ruby's ability - should create choice
        let mut executor = AbilityExecutor::new();
        
        let ruby_ability = create_ruby_ability();
        
        // Step 1: Execute cost (move Ruby to discard)
        let result = executor.resolve_ability(&ruby_ability, Some(ruby_id), 0);
        assert!(result.is_ok(), "Cost execution should succeed");
        
        // Verify Ruby moved to discard
        assert!(game_state.player1.waitroom.cards.contains(&ruby_id), "Ruby should be in discard");
        assert!(!game_state.player1.stage.stage.contains(&ruby_id), "Ruby should not be on stage");
        
        // Step 2: Should have pending choice for live card selection
        assert!(executor.get_pending_choice().is_some(), "Should have pending choice");
        
        // Step 3: Simulate user selecting live card
        let choice_result = crate::ability::executor::ChoiceResult::CardSelected { indices: vec![0] };
        let result2 = executor.provide_choice_result(choice_result);
        assert!(result2.is_ok(), "Choice should be accepted");
        
        // Step 4: Execute effect (move selected live card to hand)
        let result3 = executor.resolve_ability(&ruby_ability, Some(ruby_id), 0);
        assert!(result3.is_ok(), "Effect execution should succeed");
        
        // Verify final state
        let live_cards_in_hand = game_state.player1.hand.cards.iter()
            .any(|&id| {
                if let Some(card) = game_state.card_database.get_card(*id) {
                    card.is_live()
                } else {
                    false
                }
            });
        
        assert!(live_cards_in_hand, "Live card should be in hand");
        assert!(executor.get_pending_choice().is_none(), "Choice should be resolved");
        
        println!("✅ Final state:");
        println!("  Ruby in discard: {}", game_state.player1.waitroom.cards.contains(&ruby_id));
        println!("  Live cards remaining in discard: {:?}", game_state.player1.waitroom.cards.iter().filter(|&&id| {
            if let Some(card) = game_state.card_database.get_card(*id) {
                card.is_live()
            } else {
                false
            }
        }).count());
        println!("  Live cards in hand: {}", live_cards_in_hand);
        
        println!("✅ Full ability integration test PASSED!");
    }

    fn create_ruby_ability() -> Ability {
        Ability {
            full_text: "起動このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。".to_string(),
            cost: Some(crate::card::AbilityCost {
                text: "このメンバーをステージから控え室に置く".to_string(),
                cost_type: Some("move_cards".to_string()),
                source: Some("stage".to_string()),
                destination: Some("discard".to_string()),
                card_type: Some("member_card".to_string()),
                count: Some(1),
                self_cost: Some(true),
                r#type: Some("move_cards".to_string()),
            }),
            effect: Some(AbilityEffect {
                text: "自分の控え室からライブカードを1枚手札に加える".to_string(),
                action: "move_cards".to_string(),
                source: Some("discard".to_string()),
                destination: Some("hand".to_string()),
                count: Some(1),
                card_type: Some("live_card".to_string()),
                target: Some("self".to_string()),
            })),
            triggers: Some("起動".to_string()),
            use_limit: None,
            ..Default::default()
        }
    }
}
