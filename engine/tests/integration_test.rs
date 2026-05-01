#[cfg(test)]
mod integration_tests {
    use rabuka_engine::game_state::GameState;
    use rabuka_engine::player::Player;
    use rabuka_engine::card_loader::CardLoader;
    use rabuka_engine::card::{Ability, AbilityEffect};
    use std::sync::Arc;

    #[test]
    fn test_full_ability_integration() {
        println!("=== Full Ability Integration Test ===");
        
        // Create realistic game scenario
        let cards = CardLoader::load_cards_from_file(
            std::path::Path::new("../cards/cards.json")
        ).expect("Failed to load cards");
        let card_db = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));
        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), false);
        let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        let mut game_state = GameState::new(player1, player2, card_db);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
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
        let ruby_ability = create_ruby_ability();
        
        // Step 1: Execute cost (move Ruby to discard) - scoped borrow
        let (result, has_pending) = {
            let mut executor = rabuka_engine::ability::resolver::AbilityResolver::new(&mut game_state);
            let r = executor.resolve_ability(&ruby_ability, Some(ruby_id), 0);
            let p = executor.get_pending_choice().is_some();
            (r, p)
        };
        assert!(result.is_ok(), "Cost execution should succeed");
        
        // Verify Ruby moved to discard
        assert!(game_state.player1.waitroom.cards.contains(&ruby_id), "Ruby should be in discard");
        assert!(!game_state.player1.stage.stage.contains(&ruby_id), "Ruby should not be on stage");
        
        // Step 2: Should have pending choice for live card selection
        assert!(has_pending, "Should have pending choice");
        
        // Step 3: Simulate user selecting live card
        let result2 = {
            let mut executor = rabuka_engine::ability::resolver::AbilityResolver::new(&mut game_state);
            let choice_result = rabuka_engine::ability::types::ChoiceResult::CardSelected { indices: vec![0] };
            executor.provide_choice_result(choice_result)
        };
        assert!(result2.is_ok(), "Choice should be accepted");
        
        // Step 4: Execute effect (move selected live card to hand)
        let result3 = {
            let mut executor = rabuka_engine::ability::resolver::AbilityResolver::new(&mut game_state);
            executor.resolve_ability(&ruby_ability, Some(ruby_id), 0)
        };
        assert!(result3.is_ok(), "Effect execution should succeed");
        
        // Verify final state
        let live_cards_in_hand = game_state.player1.hand.cards.iter()
            .any(|id| {
            if let Some(card) = game_state.card_database.get_card(*id) {
                    card.is_live()
                } else {
                    false
                }
            });
        
        assert!(live_cards_in_hand, "Live card should be in hand");
        
        println!("✅ Final state:");
        println!("  Ruby in discard: {}", game_state.player1.waitroom.cards.contains(&ruby_id));
        println!("  Live cards remaining in discard: {:?}", game_state.player1.waitroom.cards.iter().filter(|id| {
            if let Some(card) = game_state.card_database.get_card(**id) {
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
            cost: Some(rabuka_engine::card::AbilityCost {
                text: "このメンバーをステージから控え室に置く".to_string(),
                cost_type: Some("move_cards".to_string()),
                source: Some("stage".to_string()),
                destination: Some("discard".to_string()),
                card_type: Some("member_card".to_string()),
                count: Some(1),
                self_cost: Some(true),
                ..Default::default()
            }),
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
            use_limit: None,
            ..Default::default()
        }
    }
}
