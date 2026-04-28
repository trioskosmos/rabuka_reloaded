// Tournament mode for running automated deck matchups
// This is separated from core game logic to keep the engine clean

use crate::card_loader;
use crate::deck_builder;
use crate::deck_parser;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use crate::game_setup;
use crate::bot::ai;
use std::vec::Vec;
use std::string::String;

pub fn run_tournament() {
    println!("=== Deck Tournament ===\n");

    // Load cards
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    // Load all decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    println!("Found {} decks:\n", deck_lists.len());
    for (i, deck) in deck_lists.iter().enumerate() {
        println!("  {}: {}", i, deck.name);
    }
    println!();
    
    // Run all matchups (each deck vs each other deck)
    let mut results: Vec<(String, String, String)> = Vec::new();
    
    for i in 0..deck_lists.len() {
        for j in 0..deck_lists.len() {
            let deck1 = &deck_lists[i];
            let deck2 = &deck_lists[j];
            
            println!("=== {} vs {} ===", deck1.name, deck2.name);

            match run_single_game(cards.clone(), deck1, deck2) {
                Ok(result) => {
                    println!("Result: {}", result);
                    results.push((deck1.name.clone(), deck2.name.clone(), result));
                }
                Err(e) => {
                    println!("Error: {}", e);
                    results.push((deck1.name.clone(), deck2.name.clone(), format!("Error: {}", e)));
                }
            }
            println!();
        }
    }
    
    // Print summary
    println!("\n=== Tournament Summary ===\n");
    for (deck1, deck2, result) in &results {
        println!("{} vs {}: {}", deck1, deck2, result);
    }
}

fn run_single_game(
    cards: Vec<crate::card::Card>,
    deck1: &deck_parser::DeckList,
    deck2: &deck_parser::DeckList,
) -> Result<String, String> {
    // Create CardDatabase from loaded cards
    let card_database = std::sync::Arc::new(crate::card::CardDatabase::load_or_create(cards));

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => return Err(format!("Failed to build deck for {}: {}", deck1.name, e)),
    };

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => return Err(format!("Failed to build deck for {}: {}", deck2.name, e)),
    };

    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck, &card_database);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck, &card_database);

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);

    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(player1, player2, card_database);
    game_setup::setup_game(&mut game_state);
    
    // Run the game with a limit
    let max_iterations = 2000; // Increased to allow games more time to complete
    let mut turn_count = 0;
    let mut last_turn_number = 0;
    let mut stuck_counter = 0;
    let mut action_count = 0;
    let start_time = std::time::Instant::now();
    
    while turn_count < max_iterations {
        turn_count += 1;
        
        // Detect if stuck
        if game_state.turn_number == last_turn_number {
            stuck_counter += 1;
            if stuck_counter > 50 {
                let elapsed = start_time.elapsed();
                let actions_per_sec = action_count as f64 / elapsed.as_secs_f64();
                return Ok(format!("Draw (stuck at turn {}) - {} actions in {:.2}s ({:.2} actions/sec)", 
                    game_state.turn_number, action_count, elapsed.as_secs_f64(), actions_per_sec));
            }
        } else {
            stuck_counter = 0;
            last_turn_number = game_state.turn_number;
        }
        
        // Check victory
        match game_state.check_victory() {
            crate::game_state::GameResult::FirstAttackerWins => {
                let winner = if game_state.player1.is_first_attacker {
                    deck1.name.clone()
                } else {
                    deck2.name.clone()
                };
                let elapsed = start_time.elapsed();
                let actions_per_sec = action_count as f64 / elapsed.as_secs_f64();
                return Ok(format!("{} wins (turn {}) - {} actions in {:.2}s ({:.2} actions/sec)", 
                    winner, game_state.turn_number, action_count, elapsed.as_secs_f64(), actions_per_sec));
            }
            crate::game_state::GameResult::SecondAttackerWins => {
                let winner = if !game_state.player1.is_first_attacker {
                    deck1.name.clone()
                } else {
                    deck2.name.clone()
                };
                let elapsed = start_time.elapsed();
                let actions_per_sec = action_count as f64 / elapsed.as_secs_f64();
                return Ok(format!("{} wins (turn {}) - {} actions in {:.2}s ({:.2} actions/sec)", 
                    winner, game_state.turn_number, action_count, elapsed.as_secs_f64(), actions_per_sec));
            }
            crate::game_state::GameResult::Draw => {
                let elapsed = start_time.elapsed();
                let actions_per_sec = action_count as f64 / elapsed.as_secs_f64();
                return Ok(format!("Draw (turn {}) - {} actions in {:.2}s ({:.2} actions/sec)", 
                    game_state.turn_number, action_count, elapsed.as_secs_f64(), actions_per_sec));
            }
            crate::game_state::GameResult::Ongoing => {
                // Debug: Print success card and live card zone counts every 50 turns
                if turn_count % 50 == 0 {
                    println!("Turn {}: P1 success: {}, P1 live: {}, P2 success: {}, P2 live: {}", 
                        game_state.turn_number, 
                        game_state.player1.success_live_card_zone.len(),
                        game_state.player1.live_card_zone.len(),
                        game_state.player2.success_live_card_zone.len(),
                        game_state.player2.live_card_zone.len());
                }
            }
        }
        
        // Auto-advance automatic phases
        match game_state.current_phase {
            crate::game_state::Phase::RockPaperScissors |
            crate::game_state::Phase::ChooseFirstAttacker |
            crate::game_state::Phase::MulliganP1Turn |
            crate::game_state::Phase::MulliganP2Turn |
            crate::game_state::Phase::Mulligan |
            crate::game_state::Phase::LiveCardSetP1Turn |
            crate::game_state::Phase::LiveCardSetP2Turn |
            crate::game_state::Phase::LiveCardSet |
            crate::game_state::Phase::Main => {
                // Manual phases - stop
                break;
            }
            crate::game_state::Phase::Active | 
            crate::game_state::Phase::Energy | 
            crate::game_state::Phase::Draw |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                turn::TurnEngine::advance_phase(&mut game_state);
            }
            crate::game_state::Phase::LiveStart | crate::game_state::Phase::LiveSuccess | crate::game_state::Phase::Cheer => {
                // Handle new phases - skip for now
                println!("Skipping new phase: {:?}", game_state.current_phase);
            }
        }
        
        // Handle manual phases with AI
        match game_state.current_phase {
            crate::game_state::Phase::RockPaperScissors => {
                // Q16: Play RPS to determine turn order
                let actions = crate::game_setup::generate_possible_actions(&game_state);
                action_count += 1;
                
                let result = turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &actions[0].action_type,
                    actions[0].parameters.as_ref().and_then(|p| p.card_id),
                    actions[0].parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    actions[0].parameters.as_ref().and_then(|p| p.stage_area),
                    actions[0].parameters.as_ref().and_then(|p| p.use_baton_touch),
                );
                
                if let Err(e) = result {
                    println!("RPS action failed: {}", e);
                }
            }
            crate::game_state::Phase::ChooseFirstAttacker => {
                // Q16: RPS winner chooses turn order (simplified: always choose first)
                let actions = crate::game_setup::generate_possible_actions(&game_state);
                action_count += 1;
                
                let result = turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &actions[0].action_type,
                    actions[0].parameters.as_ref().and_then(|p| p.card_id),
                    actions[0].parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    actions[0].parameters.as_ref().and_then(|p| p.stage_area),
                    actions[0].parameters.as_ref().and_then(|p| p.use_baton_touch),
                );
                
                if let Err(e) = result {
                    println!("ChooseFirstAttacker action failed: {}", e);
                }
            }
            crate::game_state::Phase::MulliganP1Turn |
            crate::game_state::Phase::MulliganP2Turn |
            crate::game_state::Phase::Mulligan => {
                // Mulligan phase - let the AI choose
                let ai = ai::AIPlayer::new("TournamentAI".to_string());
                let actions = crate::game_setup::generate_possible_actions(&game_state);
                let chosen_index = ai.choose_action(&actions);
                action_count += 1;
                
                let result = turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &actions[chosen_index].action_type,
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.card_id),
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.stage_area),
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.use_baton_touch),
                );
                
                if let Err(e) = result {
                    println!("Mulligan action failed: {}", e);
                }
            }
            crate::game_state::Phase::Main => {
                let actions = game_setup::generate_possible_actions(&game_state);
                if actions.is_empty() {
                    turn::TurnEngine::advance_phase(&mut game_state);
                } else {
                    let ai = ai::AIPlayer::new("TournamentAI".to_string());
                    
                    // Try to execute the chosen action
                    let chosen_index = ai.choose_action(&actions);
                    let action_type = &actions[chosen_index].action_type;
                    action_count += 1;
                    
                    // For play_member_to_stage, select a random available area
                    let (stage_area, use_baton_touch) = if *action_type == game_setup::ActionType::PlayMemberToStage {
                        if let Some(ref params) = actions[chosen_index].parameters {
                            if let Some(ref available_areas) = params.available_areas {
                                use rand::Rng;
                                let mut rng = rand::thread_rng();
                                let available: Vec<_> = available_areas.iter().filter(|a| a.available).collect();
                                if !available.is_empty() {
                                    let chosen = available[rng.gen_range(0..available.len())];
                                    (Some(chosen.area), params.use_baton_touch)
                                } else {
                                    (None, None)
                                }
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        }
                    } else {
                        (actions[chosen_index].parameters.as_ref().and_then(|p| p.stage_area), actions[chosen_index].parameters.as_ref().and_then(|p| p.use_baton_touch))
                    };
                    
                    // Execute action with error handling
                    let result = turn::TurnEngine::execute_main_phase_action(
                        &mut game_state,
                        action_type,
                        actions[chosen_index].parameters.as_ref().and_then(|p| p.card_id),
                        actions[chosen_index].parameters.as_ref().and_then(|p| p.card_indices.clone()),
                        stage_area,
                        use_baton_touch,
                    );
                    
                    if let Err(e) = result {
                        println!("Main phase action failed: {}", e);
                    }
                }
            }
            crate::game_state::Phase::LiveCardSetP1Turn |
            crate::game_state::Phase::LiveCardSetP2Turn |
            crate::game_state::Phase::LiveCardSet => {
                // Rule 8.2: Both players set live cards (automatic)
                let p1_cards = ai::AIPlayer::choose_live_cards_to_set(game_state.first_attacker());
                let p2_cards = ai::AIPlayer::choose_live_cards_to_set(game_state.second_attacker());
                let card_db = game_state.card_database.clone();
                turn::TurnEngine::player_set_live_cards(game_state.first_attacker_mut(), p1_cards, &*card_db);
                turn::TurnEngine::player_set_live_cards(game_state.second_attacker_mut(), p2_cards, &*card_db);
                game_state.current_phase = crate::game_state::Phase::FirstAttackerPerformance;
            }
            _ => {}
        }
    }
    
    let elapsed = start_time.elapsed();
    let actions_per_sec = action_count as f64 / elapsed.as_secs_f64();
    Ok(format!("Draw (max iterations reached at turn {}) - {} actions in {:.2}s ({:.2} actions/sec)", 
        game_state.turn_number, action_count, elapsed.as_secs_f64(), actions_per_sec))
}
