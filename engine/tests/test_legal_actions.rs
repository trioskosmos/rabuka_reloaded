// Tests for legal action generation
// This module contains test functions separated from main game logic

use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser;
use rabuka_engine::game_state;
use rabuka_engine::game_setup;
use rabuka_engine::player::Player;
use rabuka_engine::game_state::GameState;

#[test]
fn test_legal_actions() {
    println!("=== Testing Legal Action Filtering ===\n");
    
    // Load cards
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    // Load decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    let deck1 = &deck_lists[0];
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck: {}", e);
            return;
        }
    };
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    // Build a simple deck for player 2
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck: {}", e);
            return;
        }
    };
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    use rabuka_engine::card::CardDatabase;
    use std::sync::Arc;
    let card_database = Arc::new(CardDatabase::load_or_create(vec![]));
    let mut game_state = GameState::new(player1, player2, card_database);
    game_setup::setup_game(&mut game_state);
    
    // Test legal actions in different phases
    let test_phases = vec![
        (game_state::Phase::Active, "Active Phase"),
        (game_state::Phase::Energy, "Energy Phase"),
        (game_state::Phase::Draw, "Draw Phase"),
        (game_state::Phase::Main, "Main Phase"),
        (game_state::Phase::LiveCardSet, "Live Card Set Phase"),
    ];
    
    for (phase, phase_name) in test_phases {
        game_state.current_phase = phase.clone();
        let actions = game_setup::generate_possible_actions(&game_state);
        
        println!("--- {} ({:?}) ---", phase_name, phase);
        println!("Legal actions available: {}", actions.len());
        
        if actions.is_empty() {
            println!("  No legal actions in this phase");
        } else {
            for (i, action) in actions.iter().enumerate() {
                println!("  {}. {}", i + 1, action.description);
            }
        }
        println!();
    }
    
    println!("Test complete!");
}
