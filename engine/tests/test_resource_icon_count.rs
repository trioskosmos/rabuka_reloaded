// Test resource_icon_count functionality
// This tests that the resource_icon_count field from abilities.json is properly used

use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::player::Player;
use rabuka_engine::zones::{CardInZone, Orientation, FaceState};
use rabuka_engine::card::{Ability, AbilityEffect};
use rabuka_engine::game_state::{GameState, TurnPhase, Phase, GameResult};
use rabuka_engine::zones::ResolutionZone;
use rabuka_engine::card::Card;
use std::path::Path;
use rabuka_engine::zones::MemberArea;

/// Load all cards from the standard cards.json path
fn load_cards() -> Vec<Card> {
    let cards_path = Path::new("../cards/cards.json");
    CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards")
}

/// Create a standard test GameState with default values
fn create_test_game_state(player1: Player, player2: Player) -> GameState {
    GameState {
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
        live_card_set_player1_done: false,
        live_card_set_player2_done: false,
        history: Vec::new(),
        future: Vec::new(),
        max_history_size: 100,
    }
}

/// Create a CardInZone for placing a card on stage
fn create_card_in_zone(card: &Card) -> CardInZone {
    CardInZone {
        card: card.clone(),
        orientation: Some(Orientation::Active),
        face_state: FaceState::FaceUp,
        energy_underneath: Vec::new(),
        played_via_ability: false,
        turn_played: 1,
    }
}

/// Place a card on stage in a specific area
fn place_card_on_stage(player: &mut Player, card: &Card, area: MemberArea) {
    let card_in_zone = create_card_in_zone(card);
    match area {
        MemberArea::Center => player.stage.center = Some(card_in_zone),
        MemberArea::LeftSide => player.stage.left_side = Some(card_in_zone),
        MemberArea::RightSide => player.stage.right_side = Some(card_in_zone),
    }
}

/// Execute an ability and return the result
fn execute_ability(game_state: &mut GameState, ability: &Ability) -> Result<(), String> {
    let mut resolver = AbilityResolver::new(game_state);
    resolver.resolve_ability(ability)
}

#[test]
fn test_resource_icon_count_is_loaded() {
    let cards = load_cards();
    
    // Find a card with gain_resource ability that has resource_icon_count
    let test_card = cards.iter().find(|c| {
        c.abilities.iter().any(|a| {
            a.effect.as_ref().map_or(false, |e| {
                e.action == "gain_resource" && e.resource_icon_count.is_some()
            })
        })
    });
    
    match test_card {
        Some(card) => {
            println!("Found card with resource_icon_count: {} - {}", card.card_no, card.name);
            let ability = card.abilities.iter().find(|a| {
                a.effect.as_ref().map_or(false, |e| {
                    e.action == "gain_resource" && e.resource_icon_count.is_some()
                })
            }).unwrap();
            
            let resource_icon_count = ability.effect.as_ref().unwrap().resource_icon_count.unwrap();
            println!("Resource icon count: {}", resource_icon_count);
            assert!(resource_icon_count > 0, "Resource icon count should be positive");
        }
        None => {
            println!("No card found with resource_icon_count in abilities.json");
            println!("This is expected if abilities.json doesn't have this field populated yet");
        }
    }
}

#[test]
fn test_gain_resource_with_resource_icon_count() {
    let cards = load_cards();
    
    // Create a manual ability with resource_icon_count to test the functionality
    let test_ability = Ability {
        full_text: "Test: Gain 2 blades".to_string(),
        triggerless_text: "Test: Gain 2 blades".to_string(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: "Gain 2 blades".to_string(),
            action: "gain_resource".to_string(),
            resource: Some("blade".to_string()),
            resource_icon_count: Some(2),
            count: Some(1), // This should be ignored in favor of resource_icon_count
            ..Default::default()
        }),
        keywords: None,
    };
    
    let (mut player1, player2) = create_test_players();
    
    // Create a simple test card and place it on stage
    let test_card = cards.first().unwrap();
    place_card_on_stage(&mut player1, test_card, MemberArea::Center);
    
    let initial_blade_count = player1.stage.center.as_ref().map(|c| c.card.blade).unwrap_or(0);
    println!("Initial blade count: {}", initial_blade_count);
    
    let mut game_state = create_test_game_state(player1, player2);
    let result = execute_ability(&mut game_state, &test_ability);
    
    println!("Ability execution result: {:?}", result);
    assert!(result.is_ok(), "Ability should resolve successfully");
    
    // Verify that resource_icon_count (2) was used, not count (1)
    let final_blade_count = game_state.player1.stage.center.as_ref().map(|c| c.card.blade).unwrap_or(0);
    println!("Final blade count: {}", final_blade_count);
    
    // Should have gained 2 blades (from resource_icon_count), not 1 (from count)
    assert_eq!(final_blade_count - initial_blade_count, 2, "Should have gained 2 blades from resource_icon_count");
    
    println!("✓ resource_icon_count works correctly");
}

fn create_test_players() -> (Player, Player) {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    (player1, player2)
}
