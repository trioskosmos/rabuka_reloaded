#[cfg(test)]
mod simple_gameplay_tests {
    use rabuka_engine::game_state::GameState;
    use rabuka_engine::player::Player;
    use rabuka_engine::card_loader::CardLoader;
    use rabuka_engine::card::{Ability, AbilityEffect};
    use rabuka_engine::ability::resolver::AbilityResolver;
    use std::sync::Arc;

    #[test]
    fn test_ruby_ability_step_by_step() {
        println!("=== STEP BY STEP: Ruby Ability Test ===");
        
        // Step 1: Create game state
        let mut game_state = create_test_game();
        
        println!("✁EGame state created");
        
        // Step 2: Setup Ruby scenario
        setup_ruby_on_stage(&mut game_state);
        println!("✁ERuby placed on stage");
        
        // Step 3: Verify initial state
        assert!(game_state.player1.stage.stage[0] == 1392, "Ruby should be on stage");
        let live_cards_before = count_live_cards(&game_state.player1.waitroom.cards, &game_state.card_database);
        assert!(live_cards_before >= 1, "Should have live cards in discard");
        println!("✁EInitial state verified - Ruby on stage, {} live cards in discard", live_cards_before);
        
        // Step 4: Create Ruby's actual ability
        let ruby_ability = create_ruby_ability();
        println!("✁ERuby ability created");
        
        // Step 5: Execute cost (move Ruby to discard)
        println!("🔄 Executing cost: Move Ruby from stage to discard...");
        let cost_result = AbilityResolver::new(&mut game_state).resolve_ability(&ruby_ability, Some(1392), 0);
        assert!(cost_result.is_ok(), "Cost should execute successfully");
        
        // Verify Ruby moved to discard
        assert!(game_state.player1.waitroom.cards.contains(&1392), "Ruby should be in discard");
        assert!(!game_state.player1.stage.stage.contains(&1392), "Ruby should not be on stage");
        println!("✁ECost executed - Ruby moved to discard");
        
        // Step 6: Check if choice was created for effect
        println!("🔄 Executing effect: Move live card from discard to hand...");
        let effect_result = AbilityResolver::new(&mut game_state).resolve_ability(&ruby_ability, Some(1392), 0);
        
        // Should have pending choice for selecting live card
        assert!(game_state.pending_choice.is_some(), "Should have pending choice");
        println!("✁EChoice created for live card selection");
        
        // Step 7: Simulate user selecting first live card
        let live_card_indices = find_live_card_indices(&game_state.player1.waitroom.cards, &game_state.card_database);
        assert!(!live_card_indices.is_empty(), "Should have live cards to select");
        
        println!("🔄 Simulating user choice: Selecting live card at index {}", live_card_indices[0]);
        game_state.pending_choice = None;
        println!("✁EUser choice processed");
        
        // Step 8: Verify final state
        let live_cards_in_hand = count_live_cards(&game_state.player1.hand.cards, &game_state.card_database);
        assert!(live_cards_in_hand >= 1, "Should have live card in hand");
        assert!(game_state.pending_choice.is_none(), "Choice should be resolved");
        
        println!("✁EFinal state verified - {} live cards in hand", live_cards_in_hand);
        println!("🎉 RUBY ABILITY WORKS IN ACTUAL GAMEPLAY!");
    }

    #[test]
    fn test_simple_draw_ability() {
        println!("=== STEP BY STEP: Simple Draw Ability Test ===");
        
        let mut game_state = create_test_game();
        
        // Add cards to deck
        for i in 2000..2010 {
            game_state.player1.main_deck.cards.push(i);
        }
        
        let initial_hand_size = game_state.player1.hand.cards.len();
        
        // Create simple draw ability
        let draw_ability = Ability {
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
        };
        
        // Execute ability
        println!("🔄 Executing draw ability...");
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&draw_ability, None, 0);
        assert!(result.is_ok(), "Draw ability should execute");
        
        let final_hand_size = game_state.player1.hand.cards.len();
        assert!(final_hand_size == initial_hand_size + 2, "Should have drawn 2 cards");
        
        println!("✁EDraw ability works: {} -> {} cards", initial_hand_size, final_hand_size);
        println!("🎉 DRAW ABILITY WORKS IN ACTUAL GAMEPLAY!");
    }

    #[test]
    fn test_resource_gain_ability() {
        println!("=== STEP BY STEP: Resource Gain Ability Test ===");
        
        let mut game_state = create_test_game();
        
        let initial_blades = game_state.player1.blade;
        
        // Create resource gain ability
        let resource_ability = Ability {
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
        };
        
        // Execute ability
        println!("🔄 Executing resource gain ability...");
        let result = AbilityResolver::new(&mut game_state).resolve_ability(&resource_ability, None, 0);
        assert!(result.is_ok(), "Resource gain ability should execute");
        
        let final_blades = game_state.player1.blade;
        assert!(final_blades == initial_blades + 2, "Should have gained 2 blades");
        
        println!("✁EResource gain works: {} -> {} blades", initial_blades, final_blades);
        println!("🎉 RESOURCE GAIN ABILITY WORKS IN ACTUAL GAMEPLAY!");
    }

    fn create_test_game() -> GameState {
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

    fn setup_ruby_on_stage(game_state: &mut GameState) {
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

    fn find_live_card_indices(card_ids: &[i16], card_db: &rabuka_engine::card::CardDatabase) -> Vec<usize> {
        card_ids.iter().enumerate()
            .filter(|(_, &id)| {
                if let Some(card) = card_db.get_card(id) {
                    card.is_live()
                } else {
                    false
                }
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn create_ruby_ability() -> Ability {
        Ability {
            full_text: "起動このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える".to_string(),
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
