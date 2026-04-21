// Test a real card's ability using actual card data
// This tests PL!-sd1-005-SD 星空 凛 which has the ability:
// "このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。"
// Translation: "Move this member from stage to discard: Add 1 live card from discard to hand"

use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::zones::{CardInZone, Orientation, FaceState};
use std::path::Path;

#[test]
fn test_real_card_ability_rin_sd005() {
    // Load real card data
    let cards_path = Path::new("../cards/cards.json");
    let cards = CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards");
    
    // Find PL!-sd1-005-SD 星空 凛
    let rin_card = cards.iter()
        .find(|c| c.card_no == "PL!-sd1-005-SD")
        .expect("Could not find PL!-sd1-005-SD card");
    
    // Find a live card for the discard pile
    let live_card = cards.iter()
        .find(|c| c.is_live())
        .expect("Could not find a live card");
    
    println!("Testing card: {} - {}", rin_card.card_no, rin_card.name);
    println!("Using live card: {} - {}", live_card.card_no, live_card.name);
    println!("Ability text: {}", rin_card.ability);
    println!("Has {} abilities", rin_card.abilities.len());
    
    // Verify the ability was loaded from abilities.json
    if let Some(ability) = rin_card.abilities.get(0) {
        println!("\nAbility #0:");
        println!("  Full text: {}", ability.full_text);
        println!("  Triggers: {:?}", ability.triggers);
        println!("  Has cost: {}", ability.cost.is_some());
        println!("  Has effect: {}", ability.effect.is_some());
        
        if let Some(ref cost) = ability.cost {
            println!("  Cost type: {:?}", cost.cost_type);
            println!("  Cost source: {:?}", cost.source);
            println!("  Cost destination: {:?}", cost.destination);
        }
        
        if let Some(ref effect) = ability.effect {
            println!("  Effect action: {}", effect.action);
            println!("  Effect source: {:?}", effect.source);
            println!("  Effect destination: {:?}", effect.destination);
            println!("  Effect count: {:?}", effect.count);
            println!("  Effect card_type: {:?}", effect.card_type);
        }
        
        // Set up game state with Rin on stage and live card in discard
        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
        let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        
        // Place Rin on stage center
        let rin_in_zone = CardInZone {
            card: rin_card.clone(),
            orientation: Some(Orientation::Active),
            face_state: FaceState::FaceUp,
            energy_underneath: Vec::new(),
            played_via_ability: false,
            turn_played: 1,
        };
        player1.stage.center = Some(rin_in_zone);
        
        // Place live card in discard
        player1.waitroom.add_card(live_card.clone());
        
        println!("\nBefore ability execution:");
        println!("  Stage center: {:?}", player1.stage.center.as_ref().map(|c| &c.card.name));
        println!("  Discard count: {}", player1.waitroom.len());
        println!("  Hand count: {}", player1.hand.len());
        
        // Create game state directly to preserve pre-configured player state
        use rabuka_engine::game_state::{GameState, TurnPhase, Phase, GameResult};
        use rabuka_engine::zones::ResolutionZone;
        
        let mut game_state = GameState {
            player1,
            player2,
            current_turn_phase: TurnPhase::FirstAttackerNormal,
            current_phase: Phase::Active,
            turn_number: 1,
            resolution_zone: ResolutionZone::new(),
            is_first_turn: true,
            live_cheer_count: 0,
            turn1_abilities_played: std::collections::HashSet::new(),
            turn2_abilities_played: std::collections::HashMap::new(),
            player1_cheer_blade_heart_count: 0,
            player2_cheer_blade_heart_count: 0,
            temporary_effects: Vec::new(),
            game_result: GameResult::Ongoing,
            pending_auto_abilities: Vec::new(),
            cheer_check_completed: false,
            cheer_checks_required: 0,
            cheer_checks_done: 0,
            prohibition_effects: Vec::new(),
            turn_limited_abilities_used: std::collections::HashSet::new(),
            mulligan_player1_done: false,
            mulligan_player2_done: false,
            current_mulligan_player: String::new(),
            mulligan_selected_indices: Vec::new(),
        };
        
        let initial_discard_count = game_state.player1.waitroom.len();
        let initial_hand_count = game_state.player1.hand.len();
        
        // Execute ability using mutable resolver
        let mut resolver = AbilityResolver::new(&mut game_state);
        let result = resolver.resolve_ability(ability);
        
        println!("\nAbility execution result: {:?}", result);
        assert!(result.is_ok(), "Ability should resolve successfully");
        
        // Verify state changes persisted
        println!("\nAfter ability execution:");
        println!("  Stage center: {:?}", game_state.player1.stage.center.as_ref().map(|c| &c.card.name));
        println!("  Discard count: {}", game_state.player1.waitroom.len());
        println!("  Hand count: {}", game_state.player1.hand.len());
        
        // Rin should have moved from stage to discard
        assert!(game_state.player1.stage.center.is_none(), "Rin should have moved from stage");
        // Discard count stays the same (Rin replaces the live card that moved to hand)
        assert_eq!(game_state.player1.waitroom.len(), initial_discard_count, "Discard should have Rin replacing the live card");
        
        // Live card should have moved from discard to hand
        assert_eq!(game_state.player1.hand.len(), initial_hand_count + 1, "Hand should have 1 more card (live card)");
        
        println!("\n✓ Real card ability loaded from abilities.json");
        println!("✓ Ability structure is correct");
        println!("✓ Ability executes without errors");
        println!("✓ Ability effects persist to game state");
        println!("✓ Rin moved from stage to discard");
        println!("✓ Live card moved from discard to hand");
    } else {
        panic!("Rin card should have at least one ability");
    }
}

#[test]
fn test_draw_ability() {
    let cards_path = Path::new("../cards/cards.json");
    let cards = CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards");
    
    // Find PL!N-bp1-006-R+ 近江彼方 which has draw ability
    let card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-006-R+")
        .expect("Could not find PL!N-bp1-006-R+ card");
    
    println!("Testing card: {} - {}", card.card_no, card.name);
    println!("Ability text: {}", card.ability);
    println!("Has {} abilities", card.abilities.len());
    
    if let Some(ability) = card.abilities.get(0) {
        println!("\nAbility #0:");
        println!("  Full text: {}", ability.full_text);
        println!("  Cost type: {:?}", ability.cost.as_ref().map(|c| c.cost_type.clone()));
        println!("  Effect action: {}", ability.effect.as_ref().map(|e| &e.action).unwrap_or(&String::new()));
        
        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
        let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        
        // Add some energy cards and make them active
        for _ in 0..3 {
            let energy_card = cards.iter().find(|c| c.is_energy()).expect("Need energy card");
            let energy_in_zone = CardInZone {
                card: energy_card.clone(),
                orientation: Some(Orientation::Active),
                face_state: FaceState::FaceUp,
                energy_underneath: Vec::new(),
                played_via_ability: false,
                turn_played: 1,
            };
            player1.energy_zone.add_card(energy_in_zone);
        }
        
        let initial_hand_count = player1.hand.len();
        let initial_deck_count = player1.main_deck.cards.len();
        
        let mut game_state = GameState::new(player1, player2);
        let mut resolver = AbilityResolver::new(&mut game_state);
        let result = resolver.resolve_ability(ability);
        
        println!("\nAbility execution result: {:?}", result);
        assert!(result.is_ok(), "Ability should resolve successfully");
        
        println!("\nAfter ability execution:");
        println!("  Hand count: {}", game_state.player1.hand.len());
        println!("  Deck count: {}", game_state.player1.main_deck.cards.len());
        
        assert_eq!(game_state.player1.hand.len(), initial_hand_count + 1, "Should have drawn 1 card");
        assert_eq!(game_state.player1.main_deck.cards.len(), initial_deck_count - 1, "Deck should have 1 fewer card");
        
        println!("\n✓ Draw ability works");
    } else {
        panic!("Card should have at least one ability");
    }
}
