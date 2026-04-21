// Test mode for automated gameplay analysis
// Auto-plays several turns and validates game state against rules

use crate::card_loader;
use crate::deck_builder;
use crate::deck_parser;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use crate::game_setup;
use std::vec::Vec;

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
    
    // Rule 6.1.1: Deck composition - main deck should have 60 cards total (48 member + 12 live)
    // Must include live_card_zone since cards move there during gameplay
    let p1_stage_members = count_stage_members(&game_state.player1.stage);
    let p1_total = game_state.player1.main_deck.len() + game_state.player1.hand.len() + 
                    game_state.player1.waitroom.len() + game_state.player1.live_card_zone.len() +
                    game_state.player1.success_live_card_zone.len() + p1_stage_members;
    if p1_total != 60 {
        issues.push(format!("P1 card count mismatch: expected 60, got {} (deck={}, hand={}, waitroom={}, live={}, success={}, stage={})",
            p1_total, game_state.player1.main_deck.len(), game_state.player1.hand.len(),
            game_state.player1.waitroom.len(), game_state.player1.live_card_zone.len(),
            game_state.player1.success_live_card_zone.len(), p1_stage_members));
    }
    
    let p2_stage_members = count_stage_members(&game_state.player2.stage);
    let p2_total = game_state.player2.main_deck.len() + game_state.player2.hand.len() + 
                    game_state.player2.waitroom.len() + game_state.player2.live_card_zone.len() +
                    game_state.player2.success_live_card_zone.len() + p2_stage_members;
    if p2_total != 60 {
        issues.push(format!("P2 card count mismatch: expected 60, got {} (deck={}, hand={}, waitroom={}, live={}, success={}, stage={})",
            p2_total, game_state.player2.main_deck.len(), game_state.player2.hand.len(),
            game_state.player2.waitroom.len(), game_state.player2.live_card_zone.len(),
            game_state.player2.success_live_card_zone.len(), p2_stage_members));
    }
    
    // Rule 4.5: Stage should have max 3 members
    if p1_stage_members > 3 {
        issues.push(format!("P1 stage has too many members: {}", p1_stage_members));
    }
    
    if p2_stage_members > 3 {
        issues.push(format!("P2 stage has too many members: {}", p2_stage_members));
    }
    
    // Note: Duplicate cards are allowed in decks (see deck lists with "x 2", "x 3", etc.)
    // So we don't validate for duplicates in hand
    
    // Check energy cards should be in energy zone
    let p1_hand_energy = game_state.player1.hand.cards.iter().filter(|c| c.is_energy()).count();
    if p1_hand_energy > 0 {
        issues.push(format!("P1 has {} energy cards in hand (should be in energy zone)", p1_hand_energy));
    }
    
    let p2_hand_energy = game_state.player2.hand.cards.iter().filter(|c| c.is_energy()).count();
    if p2_hand_energy > 0 {
        issues.push(format!("P2 has {} energy cards in hand (should be in energy zone)", p2_hand_energy));
    }
    
    issues
}

fn count_stage_members(stage: &crate::zones::Stage) -> usize {
    let mut count = 0;
    if stage.left_side.is_some() { count += 1; }
    if stage.center.is_some() { count += 1; }
    if stage.right_side.is_some() { count += 1; }
    count
}

pub fn run_test_mode() {
    println!("=== AUTOMATED TEST MODE ===\n");
    
    // Load cards
    println!("Loading cards...");
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            println!("Loaded {} cards", card_map.len());
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    // Load decks
    println!("Loading decks...");
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => {
            println!("Loaded {} decks", decks.len());
            decks
        }
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
    
    println!("\nGame initialized with deck: {}", deck1.name);
    println!("\n--- INITIAL STATE ---");
    print_game_state_summary(&game_state);
    
    let initial_issues = validate_game_state(&game_state);
    if !initial_issues.is_empty() {
        println!("\n⚠️  INITIAL VALIDATION ISSUES:");
        for issue in &initial_issues {
            println!("  - {}", issue);
        }
    } else {
        println!("\n✅ Initial state valid");
    }
    
    // Play several turns
    let max_turns = 10;
    let mut total_issues = initial_issues.len();
    
    for turn in 1..=max_turns {
        eprintln!("\n--- TURN {} ---", turn);
        
        // Auto-advance automatic phases
        let mut phase_loop_count = 0;
        loop {
            phase_loop_count += 1;
            if phase_loop_count > 100 {
                eprintln!("⚠️  Phase loop exceeded 100 iterations, breaking to prevent infinite loop");
                eprintln!("Current phase: {:?}", game_state.current_phase);
                break;
            }
            
            let current_phase = game_state.current_phase.clone();
            match current_phase {
                crate::game_state::Phase::Active |
                crate::game_state::Phase::Energy |
                crate::game_state::Phase::Draw => {
                    eprintln!("Auto-advancing {:?}...", current_phase);
                    turn::TurnEngine::advance_phase(&mut game_state);
                }
                crate::game_state::Phase::LiveCardSet => {
                    eprintln!("Placing cards in LiveCardSet phase...");
                    // Per Rule 8.2.2, place up to 3 cards from hand (any cards, not just live)
                    // Then flip face-up later and non-live cards go to waitroom (Rule 8.3.4)
                    let player = game_state.active_player_mut();
                    let cards_to_place = std::cmp::min(3, player.hand.cards.len());
                    
                    if cards_to_place > 0 {
                        let indices: Vec<usize> = (0..cards_to_place).collect();
                        eprintln!("  Placing {} cards at indices {:?}", cards_to_place, indices);
                        
                        match turn::TurnEngine::execute_main_phase_action(
                            &mut game_state,
                            "place_live_cards",
                            None,
                            Some(indices),
                            None,
                            None,
                        ) {
                            Ok(_) => eprintln!("  Cards placed successfully"),
                            Err(e) => {
                                eprintln!("  Failed to place cards: {}", e);
                                turn::TurnEngine::advance_phase(&mut game_state);
                            }
                        }
                    } else {
                        eprintln!("No cards in hand, advancing phase");
                        turn::TurnEngine::advance_phase(&mut game_state);
                    }
                }
                crate::game_state::Phase::FirstAttackerPerformance |
                crate::game_state::Phase::SecondAttackerPerformance |
                crate::game_state::Phase::LiveVictoryDetermination => {
                    eprintln!("Auto-advancing live phase...");
                    turn::TurnEngine::advance_phase(&mut game_state);
                }
                crate::game_state::Phase::RockPaperScissors => {
                    eprintln!("Executing RPS choice...");
                    let _ = turn::TurnEngine::execute_main_phase_action(
                        &mut game_state,
                        "rps_choice",
                        None,
                        None,
                        Some("rock".to_string()),
                        None,
                    );
                }
                crate::game_state::Phase::Mulligan => {
                    eprintln!("Skipping mulligan...");
                    let _ = turn::TurnEngine::execute_main_phase_action(
                        &mut game_state,
                        "skip_mulligan",
                        None,
                        None,
                        None,
                        None,
                    );
                }
                crate::game_state::Phase::Main => {
                    eprintln!("Reached Main phase");
                    break;
                }
            }
        }
        
        // In Main phase, execute actions with simple strategy
        let actions = game_setup::generate_possible_actions(&game_state);
        println!("Phase: {:?}, Actions available: {}", game_state.current_phase, actions.len());
        
        if actions.is_empty() {
            println!("No actions, passing...");
            turn::TurnEngine::advance_phase(&mut game_state);
        } else {
            // Execute multiple actions per turn (up to 3)
            let actions_per_turn = 3;
            for i in 0..std::cmp::min(actions_per_turn, actions.len()) {
                let action = &actions[i];
                println!("Executing action {}/{}: {}", i + 1, actions.len(), action.description);
                
                match turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &action.action_type,
                    action.parameters.as_ref().and_then(|p| p.card_index),
                    action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    action.parameters.as_ref().and_then(|p| p.stage_area.clone()),
                    action.parameters.as_ref().and_then(|p| p.use_baton_touch),
                ) {
                    Ok(_) => {
                        println!("✓ Action executed");
                    }
                    Err(e) => {
                        println!("✗ Action failed: {}", e);
                    }
                }
                
                // Re-generate actions after each move
                let new_actions = game_setup::generate_possible_actions(&game_state);
                if new_actions.is_empty() {
                    break;
                }
            }
            
            // Pass after actions
            println!("Passing turn...");
            turn::TurnEngine::advance_phase(&mut game_state);
        }
        
        // Validate after action
        let issues = validate_game_state(&game_state);
        if !issues.is_empty() {
            println!("⚠️  Validation issues:");
            for issue in &issues {
                println!("  - {}", issue);
            }
            total_issues += issues.len();
        }
        
        // Check for game over
        if game_state.game_result != crate::game_state::GameResult::Ongoing {
            println!("\n🏆 Game over: {:?}", game_state.game_result);
            break;
        }
        
        print_game_state_summary(&game_state);
    }
    
    println!("\n=== TEST SUMMARY ===");
    println!("Turns played: {}", max_turns);
    println!("Total validation issues found: {}", total_issues);
    
    if total_issues == 0 {
        println!("✅ No issues detected in {} turns", max_turns);
    } else {
        println!("⚠️  Found {} issues during gameplay", total_issues);
    }
}

fn print_game_state_summary(game_state: &GameState) {
    println!("Turn: {} | Phase: {:?}", game_state.turn_number, game_state.current_phase);
    println!("P1: Hand={}, Stage={}, Energy={}, Success={}", 
        game_state.player1.hand.len(),
        count_stage_members(&game_state.player1.stage),
        game_state.player1.energy_zone.cards.len(),
        game_state.player1.success_live_card_zone.len());
    println!("P2: Hand={}, Stage={}, Energy={}, Success={}", 
        game_state.player2.hand.len(),
        count_stage_members(&game_state.player2.stage),
        game_state.player2.energy_zone.cards.len(),
        game_state.player2.success_live_card_zone.len());
}
