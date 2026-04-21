// Interactive headless game mode for manual gameplay
// This provides a CLI interface to play the game without the web UI
#![allow(dead_code)]

use crate::card_loader;
use crate::deck_builder::DeckBuilder;
use crate::deck_parser;
use crate::game_setup;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use std::io::{self, Write};

fn print_game_state(game_state: &GameState) {
    println!("\n=== GAME STATE ===");
    println!("Turn: {} | Phase: {:?} | Turn Phase: {:?}", 
        game_state.turn_number, game_state.current_phase, game_state.current_turn_phase);
    
    println!("\n--- PLAYER 1 ({}) ---", game_state.player1.name);
    print_player_state(&game_state.player1, &game_state.card_database);
    
    println!("\n--- PLAYER 2 ({}) ---", game_state.player2.name);
    print_player_state(&game_state.player2, &game_state.card_database);
    
    println!("\n--- VICTORY STATUS ---");
    match game_state.check_victory() {
        crate::game_state::GameResult::FirstAttackerWins => {
            let winner = if game_state.player1.is_first_attacker { "Player 1" } else { "Player 2" };
            println!("🏆 {} WINS!", winner);
        }
        crate::game_state::GameResult::SecondAttackerWins => {
            let winner = if game_state.player1.is_first_attacker { "Player 2" } else { "Player 1" };
            println!("🏆 {} WINS!", winner);
        }
        crate::game_state::GameResult::Draw => {
            println!("🤝 GAME IS A DRAW");
        }
        crate::game_state::GameResult::Ongoing => {
            println!("Game in progress...");
            println!("P1 Success Cards: {} | P2 Success Cards: {}", 
                game_state.player1.success_live_card_zone.len(),
                game_state.player2.success_live_card_zone.len());
        }
    }
    println!("==================\n");
}

fn print_player_state(player: &Player, card_db: &crate::card::CardDatabase) {
    println!("Hand ({} cards):", player.hand.cards.len());
    for (i, card_id) in player.hand.cards.iter().enumerate() {
        if let Some(card) = card_db.get_card(*card_id) {
            let card_type_emoji = match card.card_type {
                crate::card::CardType::Member => "👤",
                crate::card::CardType::Live => "🎤",
                crate::card::CardType::Energy => "⚡",
            };
            println!("  [{}] {} {} - {} (Cost: {:?}, Hearts: {:?})", 
                i, card_type_emoji, card.name, card.card_no, card.cost, card.base_heart);
        } else {
            println!("  [{}] Unknown card {}", i, card_id);
        }
    }
    
    println!("Energy Zone ({} cards, {} active):", player.energy_zone.cards.len(), player.energy_zone.active_energy_count);
    for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
        let state = if i < player.energy_zone.active_energy_count {
            "✓ (active)"
        } else {
            "✗ (wait)"
        };
        if let Some(card) = card_db.get_card(*card_id) {
            println!("  [{}] {} {} - {} [{}]", i, "⚡", card.name, card.card_no, state);
        } else {
            println!("  [{}] Unknown card {} [{}]", i, card_id, state);
        }
    }
    
    println!("Stage:");
    // Orientation is now tracked in GameState modifiers
    // For now, assume all stage cards are active
    let state = "✓";
    if player.stage.stage[0] != -1 {
        if let Some(card) = card_db.get_card(player.stage.stage[0]) {
            println!("  Left: {} {} - {} [{}]",
                "👤", card.name, card.card_no, state);
        } else {
            println!("  Left: Unknown card {} [{}]", player.stage.stage[0], state);
        }
    } else {
        println!("  Left: (empty)");
    }
    if player.stage.stage[1] != -1 {
        if let Some(card) = card_db.get_card(player.stage.stage[1]) {
            println!("  Center: {} {} - {} [{}]",
                "👤", card.name, card.card_no, state);
        } else {
            println!("  Center: Unknown card {} [{}]", player.stage.stage[1], state);
        }
    } else {
        println!("  Center: (empty)");
    }
    if player.stage.stage[2] != -1 {
        if let Some(card) = card_db.get_card(player.stage.stage[2]) {
            println!("  Right: {} {} - {} [{}]",
                "👤", card.name, card.card_no, state);
        } else {
            println!("  Right: Unknown card {} [{}]", player.stage.stage[2], state);
        }
    } else {
        println!("  Right: (empty)");
    }
    
    println!("Live Card Zone: {} cards", player.live_card_zone.cards.len());
    for (i, card_id) in player.live_card_zone.cards.iter().enumerate() {
        if let Some(card) = card_db.get_card(*card_id) {
            println!("  [{}] {} {} - {} (Score: {:?}, Need: {:?})", 
                i, "🎤", card.name, card.card_no, card.score, card.need_heart);
        } else {
            println!("  [{}] Unknown card {}", i, card_id);
        }
    }
    
    println!("Success Live Card Zone: {} cards", player.success_live_card_zone.len());
    for (i, card_id) in player.success_live_card_zone.cards.iter().enumerate() {
        if let Some(card) = card_db.get_card(*card_id) {
            println!("  [{}] {} {} - {} (Score: {:?})", 
                i, "🎤", card.name, card.card_no, card.score);
        } else {
            println!("  [{}] Unknown card {}", i, card_id);
        }
    }
    
    println!("Waitroom: {} cards", player.waitroom.cards.len());
    println!("Main Deck: {} cards", player.main_deck.len());
    println!("Energy Deck: {} cards", player.energy_deck.cards.len());
}

fn print_actions(actions: &[crate::game_setup::Action]) {
    println!("\n=== AVAILABLE ACTIONS ({}) ===", actions.len());
    for (i, action) in actions.iter().enumerate() {
        let action_type = &action.action_type;
        let icon = match action_type.to_string().as_str() {
            "pass" => "⏭",
            "play_member" => "👤",
            "activate_energy" => "⚡",
            "play_live" => "🎤",
            "activate_ability" => "✨",
            "cheer" => "💖",
            "baton_touch" => "🔄",
            _ => "▶",
        };
        println!("  [{}] {} {}", i, icon, action.description);
    }
    println!("===========================\n");
}

fn validate_game_state(game_state: &GameState) -> Vec<String> {
    let mut issues = Vec::new();
    
    // Rule 1.2.1.1: Victory condition
    let p1_success = game_state.player1.success_live_card_zone.len();
    let p2_success = game_state.player2.success_live_card_zone.len();
    
    if p1_success >= 3 && p2_success <= 2 {
        if game_state.game_result != crate::game_state::GameResult::FirstAttackerWins {
            issues.push(format!("Game should be over: P1 has {} success cards, P2 has {}", p1_success, p2_success));
        }
    }
    
    if p2_success >= 3 && p1_success <= 2 {
        if game_state.game_result != crate::game_state::GameResult::SecondAttackerWins {
            issues.push(format!("Game should be over: P2 has {} success cards, P1 has {}", p2_success, p1_success));
        }
    }
    
    // Rule 6.1.1: Deck composition
    let p1_stage_members = count_stage_members(&game_state.player1.stage);
    if game_state.player1.main_deck.len() + game_state.player1.hand.len() + 
       game_state.player1.waitroom.len() + p1_stage_members != 48 {
        issues.push(format!("P1 card count mismatch: deck={}, hand={}, waitroom={}, stage={}",
            game_state.player1.main_deck.len(), game_state.player1.hand.len(),
            game_state.player1.waitroom.len(), p1_stage_members));
    }
    
    // Rule 4.5: Stage should have max 3 members
    let p1_stage_members = count_stage_members(&game_state.player1.stage);
    if p1_stage_members > 3 {
        issues.push(format!("P1 stage has too many members: {}", p1_stage_members));
    }
    
    let p2_stage_members = count_stage_members(&game_state.player2.stage);
    if p2_stage_members > 3 {
        issues.push(format!("P2 stage has too many members: {}", p2_stage_members));
    }
    
    // Check for duplicate cards in hand (should not happen in normal play)
    let p1_hand_card_ids: Vec<i16> = game_state.player1.hand.cards.to_vec();
    let p1_hand_unique: std::collections::HashSet<_> = p1_hand_card_ids.iter().collect();
    if p1_hand_card_ids.len() != p1_hand_unique.len() {
        issues.push("P1 hand has duplicate cards".to_string());
    }
    
    issues
}

fn count_stage_members(stage: &crate::zones::Stage) -> usize {
    let mut count = 0;
    if stage.stage[0] != -1 { count += 1; }
    if stage.stage[1] != -1 { count += 1; }
    if stage.stage[2] != -1 { count += 1; }
    count
}

pub fn run_interactive_headless() {
    println!("=== INTERACTIVE HEADLESS GAME MODE ===\n");

    // Load cards
    println!("Loading cards...");
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            println!("Loaded {} cards", cards.len());
            cards
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };

    // Create CardDatabase from loaded cards
    let card_database = std::sync::Arc::new(crate::card::CardDatabase::load_or_create(cards.clone()));

    // Load decks
    println!("Loading decks...");
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => {
            println!("Loaded {} decks:", decks.len());
            for (i, deck) in decks.iter().enumerate() {
                println!("  [{}] {}", i, deck.name);
            }
            decks
        }
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };

    // Let user choose decks
    let deck1 = choose_deck_interactive(&deck_lists, "Player 1");
    let deck2 = choose_deck_interactive(&deck_lists, "Player 2");

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
    
    println!("\nGame initialized!");
    
    // Main game loop
    loop {
        // Print game state
        print_game_state(&game_state);
        
        // Validate game state
        let issues = validate_game_state(&game_state);
        if !issues.is_empty() {
            println!("⚠️  VALIDATION ISSUES:");
            for issue in &issues {
                println!("  - {}", issue);
            }
        }
        
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
            crate::game_state::Phase::LiveCardSet |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                println!("Auto-advancing live phase...");
                turn::TurnEngine::advance_phase(&mut game_state);
                continue;
            }
            _ => {}
        }
        
        // Generate and show actions
        let actions = game_setup::generate_possible_actions(&game_state);
        print_actions(&actions);
        
        if actions.is_empty() {
            println!("No actions available, advancing phase...");
            turn::TurnEngine::advance_phase(&mut game_state);
            continue;
        }
        
        // Get user input
        print!("Enter action number (or 'q' to quit, 'v' to validate): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        if input == "q" || input == "quit" {
            println!("Quitting game...");
            break;
        }
        
        if input == "v" || input == "validate" {
            let issues = validate_game_state(&game_state);
            if issues.is_empty() {
                println!("✅ No validation issues found");
            } else {
                println!("⚠️  Validation issues:");
                for issue in &issues {
                    println!("  - {}", issue);
                }
            }
            continue;
        }
        
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
                        println!("✓ Action executed successfully");
                    }
                    Err(e) => {
                        println!("✗ Action failed: {}", e);
                    }
                }
            }
            _ => {
                println!("Invalid input. Please enter a number between 0 and {}", actions.len() - 1);
            }
        }
    }
}

fn choose_deck_interactive(deck_lists: &[deck_parser::DeckList], player_name: &str) -> deck_parser::DeckList {
    println!("\n{} - Choose a deck:", player_name);
    for (i, deck) in deck_lists.iter().enumerate() {
        println!("  [{}] {}", i, deck.name);
    }
    
    loop {
        print!("Enter deck number: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        match input.trim().parse::<usize>() {
            Ok(index) if index < deck_lists.len() => {
                println!("{} chose: {}", player_name, deck_lists[index].name);
                return deck_lists[index].clone();
            }
            _ => {
                println!("Invalid input. Please enter a number between 0 and {}", deck_lists.len() - 1);
            }
        }
    }
}
