// Interactive headless game mode - shows game state and actions
use crate::card_loader;
use crate::deck_builder::DeckBuilder;
use crate::deck_parser;
use crate::game_setup;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use std::io::{self, Write};

pub fn run_interactive_headless() {
    println!("=== INTERACTIVE HEADLESS GAME MODE ===\n");

    // Load cards
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };

    let card_database = std::sync::Arc::new(crate::card::CardDatabase::load_or_create(cards.clone()));

    // Load decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };

    // Use first deck for both players
    let deck1 = deck_lists[0].clone();
    let deck2 = deck_lists[0].clone();

    // Build decks
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck2);

    let mut player1_deck = match DeckBuilder::build_deck_from_database(&card_database, card_numbers1) {
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

    let mut player2_deck = match DeckBuilder::build_deck_from_database(&card_database, card_numbers2) {
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

    let _ = DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck, &card_database);
    let _ = DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck, &card_database);

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);

    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(player1, player2, card_database);
    game_setup::setup_game(&mut game_state);
    
    println!("Game initialized!");
    
    // Main game loop
    loop {
        // Show game state
        println!("\n=== GAME STATE ===");
        println!("Turn: {} | Phase: {:?} | Turn Phase: {:?}", 
            game_state.turn_number, game_state.current_phase, game_state.current_turn_phase);
        
        println!("\n--- PLAYER 1 ---");
        println!("Hand: {} cards", game_state.player1.hand.cards.len());
        println!("Energy: {} cards, {} active", game_state.player1.energy_zone.cards.len(), game_state.player1.energy_zone.active_energy_count);
        println!("Stage: {} members", count_stage_members(&game_state.player1.stage));
        
        println!("\n--- PLAYER 2 ---");
        println!("Hand: {} cards", game_state.player2.hand.cards.len());
        println!("Energy: {} cards, {} active", game_state.player2.energy_zone.cards.len(), game_state.player2.energy_zone.active_energy_count);
        println!("Stage: {} members", count_stage_members(&game_state.player2.stage));
        
        // Check for game over
        if game_state.game_result != crate::game_state::GameResult::Ongoing {
            println!("\n=== GAME OVER ===");
            break;
        }
        
        // Auto-advance automatic phases
        match game_state.current_phase {
            crate::game_state::Phase::Active |
            crate::game_state::Phase::Energy |
            crate::game_state::Phase::Draw => {
                println!("Auto-advancing {:?} phase...", game_state.current_phase);
                turn::TurnEngine::advance_phase(&mut game_state);
                continue;
            }
            _ => {}
        }
        
        // Show available actions
        let actions = game_setup::generate_possible_actions(&game_state);
        println!("\n=== AVAILABLE ACTIONS ({}) ===", actions.len());
        for (i, action) in actions.iter().enumerate() {
            println!("  [{}] {}", i, action.description);
        }
        
        if actions.is_empty() {
            println!("No actions available, advancing phase...");
            turn::TurnEngine::advance_phase(&mut game_state);
            continue;
        }
        
        // Get action choice
        print!("Enter action number: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        match input.parse::<usize>() {
            Ok(index) if index < actions.len() => {
                let action = &actions[index];
                println!("Executing: {}", action.description);
                
                match turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &action.action_type,
                    actions[index].parameters.as_ref().and_then(|p| p.card_id),
                    action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    action.parameters.as_ref().and_then(|p| p.stage_area),
                    action.parameters.as_ref().and_then(|p| p.use_baton_touch),
                ) {
                    Ok(_) => {
                        println!("Action executed successfully");
                    }
                    Err(e) => {
                        println!("Action failed: {}", e);
                    }
                }
            }
            _ => {
                println!("Invalid input. Please enter a number between 0 and {}", actions.len() - 1);
            }
        }
    }
}

fn count_stage_members(stage: &crate::zones::Stage) -> usize {
    let mut count = 0;
    if stage.stage[0] != -1 { count += 1; }
    if stage.stage[1] != -1 { count += 1; }
    if stage.stage[2] != -1 { count += 1; }
    count
}
