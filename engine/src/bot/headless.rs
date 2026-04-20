// Headless game mode for automated testing
// This is separated from core game logic to keep the engine clean

use crate::card_loader;
use crate::deck_builder;
use crate::deck_parser;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use crate::game_setup;
use crate::bot::ai;

pub fn run_headless_game() {
    println!("=== Running Headless Game ===\n");
    
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
    
    // Use first deck for both players
    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];
    
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 1: {}", e);
            return;
        }
    };
    
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 2: {}", e);
            return;
        }
    };
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    let mut game_state = GameState::new(player1, player2);
    game_setup::setup_game(&mut game_state);
    
    // Run the game automatically
    let mut turn_count = 0;
    let max_iterations = 1000;
    let mut last_turn_number = 0;
    let mut stuck_counter = 0;
    
    while turn_count < max_iterations {
        turn_count += 1;
        
        // Detect if we're stuck on the same turn for too long
        if game_state.turn_number == last_turn_number {
            stuck_counter += 1;
            if stuck_counter > 100 {
                println!("ERROR: Game appears stuck on turn {}", game_state.turn_number);
                println!("Current phase: {:?}", game_state.current_phase);
                println!("Turn phase: {:?}", game_state.current_turn_phase);
                println!("P1 hand: {}, P1 energy: {}", game_state.player1.hand.len(), game_state.player1.energy_zone.cards.len());
                println!("P2 hand: {}, P2 energy: {}", game_state.player2.hand.len(), game_state.player2.energy_zone.cards.len());
                println!("P1 deck: {}, P2 deck: {}", game_state.player1.main_deck.len(), game_state.player2.main_deck.len());
                println!("P1 energy deck: {}, P2 energy deck: {}", game_state.player1.energy_deck.cards.len(), game_state.player2.energy_deck.cards.len());
                break;
            }
        } else {
            stuck_counter = 0;
            last_turn_number = game_state.turn_number;
        }
        
        // Only print every 10 iterations to reduce output
        if turn_count % 10 == 1 || game_state.current_phase == crate::game_state::Phase::Main {
            println!("--- Turn {} (iteration {}) ---", game_state.turn_number, turn_count);
            println!("Phase: {:?}", game_state.current_phase);
            println!("Turn Phase: {:?}", game_state.current_turn_phase);
        }
        
        // Check for victory
        match game_state.check_victory() {
            crate::game_state::GameResult::FirstAttackerWins => {
                println!("\n=== Player 1 Wins! ===");
                println!("Success Live Cards: {}", game_state.player1.success_live_card_zone.len());
                return;
            }
            crate::game_state::GameResult::SecondAttackerWins => {
                println!("\n=== Player 2 Wins! ===");
                println!("Success Live Cards: {}", game_state.player2.success_live_card_zone.len());
                return;
            }
            crate::game_state::GameResult::Draw => {
                println!("\n=== Game Draw! ===");
                return;
            }
            crate::game_state::GameResult::Ongoing => {}
        }
        
        // Auto-advance automatic phases
        match game_state.current_phase {
            crate::game_state::Phase::Active | 
            crate::game_state::Phase::Energy | 
            crate::game_state::Phase::Draw => {
                println!("Auto-advancing automatic phase...");
                turn::TurnEngine::advance_phase(&mut game_state);
            }
            crate::game_state::Phase::Main => {
                // In Main phase, use AI to choose an action
                let actions = game_setup::generate_possible_actions(&game_state);
                if actions.is_empty() {
                    println!("No actions available, passing...");
                    turn::TurnEngine::advance_phase(&mut game_state);
                } else {
                    // Use AI module to choose action
                    let ai = ai::AIPlayer::new("HeadlessAI".to_string());
                    let action_descriptions: Vec<String> = actions.iter().map(|a| a.description.clone()).collect();
                    let chosen_index = ai.choose_action(&action_descriptions);
                    
                    println!("Actions available: {}", actions.len());
                    for (i, action) in actions.iter().enumerate() {
                        println!("  [{}] {}", i, action.description);
                    }
                    println!("Choosing: {}", actions[chosen_index].description);
                    
                    // Execute the chosen action
                    turn::TurnEngine::execute_main_phase_action(&mut game_state, &actions[chosen_index].action_type);
                }
            }
            crate::game_state::Phase::LiveCardSet |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                println!("Auto-advancing live phase...");
                turn::TurnEngine::advance_phase(&mut game_state);
            }
        }
        
        println!();
    }
    
    println!("Game stopped after {} iterations (max reached)", max_iterations);
}
