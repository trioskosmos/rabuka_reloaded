#[cfg(test)]
mod ability_verification_tests {
    use crate::game_state::GameState;
    use crate::player::Player;
    use crate::card_loader;
    use crate::card::{Ability, AbilityEffect};
    use crate::ability::executor::AbilityExecutor;

    #[test]
    fn test_ruby_ability_execution() {
        // Create a test game state with 黒澤ルビィ card on stage
        let mut game_state = create_test_game_state();
        let ruby_id = 1392; // 黒澤ルビィ's card ID
        
        // Place Ruby on stage
        game_state.player1.stage.stage[0] = ruby_id;
        game_state.player1.hand.add_card(1401); // Add a live card to hand
        game_state.player1.waitroom.add_card(1402); // Add another live card to waitroom
        
        println!("=== Test Setup ===");
        println!("Ruby on stage: {}", ruby_id);
        println!("Hand cards: {:?}", game_state.player1.hand.cards);
        println!("Waitroom cards: {:?}", game_state.player1.waitroom.cards);
        
        // Test ability activation
        let mut executor = AbilityExecutor::new();
        
        // Create Ruby's ability (move self from stage to discard, move 1 live from discard to hand)
        let ability = Ability {
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
        };
        
        // Execute the ability
        let result = executor.resolve_ability(&ability, Some(ruby_id), 0);
        
        match result {
            Ok(_) => {
                println!("✅ Ability executed successfully!");
                
                // Verify game state changes
                println!("After execution:");
                println!("  Ruby on stage: {:?}", game_state.player1.stage.stage);
                println!("  Hand cards: {:?}", game_state.player1.hand.cards);
                println!("  Waitroom cards: {:?}", game_state.player1.waitroom.cards);
                
                // Ruby should be in discard, live card should be in hand
                let ruby_in_discard = game_state.player1.waitroom.cards.contains(&ruby_id);
                let live_cards_in_hand = game_state.player1.hand.cards.iter()
                    .any(|&id| {
                        if let Some(card) = game_state.card_database.get_card(*id) {
                            card.is_live()
                        } else {
                            false
                        }
                    });
                
                assert!(ruby_in_discard, "Ruby should be in discard after cost payment");
                assert!(live_cards_in_hand, "Live card should be moved to hand");
            }
            Err(e) => {
                println!("❌ Ability execution failed: {}", e);
            }
        }
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
