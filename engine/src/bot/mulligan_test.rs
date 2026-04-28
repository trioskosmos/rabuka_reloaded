// Test mulligan phase specifically
use crate::card_loader;
use crate::deck_builder::DeckBuilder;
use crate::deck_parser;
use crate::game_setup;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use crate::bot::ai;

pub fn test_mulligan_flow() {
    println!("=== TESTING MULLIGAN FLOW ===\n");

    // Load cards
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

    let card_database = std::sync::Arc::new(crate::card::CardDatabase::load_or_create(cards.clone()));

    // Load decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };

    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);

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

    println!("Game setup complete");
    println!("Phase: {:?}", game_state.current_phase);
    println!("Mulligan player index: {}", game_state.current_mulligan_player_idx);
    println!("P1 hand: {} cards", game_state.player1.hand.cards.len());
    println!("P2 hand: {} cards", game_state.player2.hand.cards.len());

    // Play through RPS
    println!("\n--- Playing RPS ---");
    let result = turn::TurnEngine::execute_main_phase_action(
        &mut game_state,
        &crate::game_setup::ActionType::RockChoice,
        None,
        None,
        None,
        None,
    );
    match result {
        Ok(_) => println!("RPS successful"),
        Err(e) => eprintln!("RPS failed: {}", e),
    }
    println!("Phase: {:?}", game_state.current_phase);
    println!("RPS winner: {:?}", game_state.rps_winner);

    // If Player 2 won, they auto-choose turn order, otherwise Player 1 chooses
    if game_state.current_phase == crate::game_state::Phase::Mulligan {
        println!("Player 2 won RPS, auto-chose turn order");
    } else if game_state.current_phase == crate::game_state::Phase::ChooseFirstAttacker {
        println!("Player 1 won RPS, choosing turn order");
        let result = turn::TurnEngine::execute_main_phase_action(
            &mut game_state,
            &crate::game_setup::ActionType::ChooseFirstAttacker,
            None,
            None,
            None,
            None,
        );
        match result {
            Ok(_) => println!("Turn order chosen"),
            Err(e) => eprintln!("Turn order choice failed: {}", e),
        }
    }

    println!("Phase: {:?}", game_state.current_phase);
    println!("P1 hand: {} cards", game_state.player1.hand.cards.len());
    println!("P2 hand: {} cards", game_state.player2.hand.cards.len());
    println!("P1 energy: {} cards", game_state.player1.energy_zone.cards.len());
    println!("P2 energy: {} cards", game_state.player2.energy_zone.cards.len());

    // Test 1: First player skips mulligan
    println!("\n--- Test 1: First player skips mulligan ---");
    println!("Before: Mulligan player index: {}", game_state.current_mulligan_player_idx);
    println!("Before: Active player: {}", game_state.active_player().id);
    let result = turn::TurnEngine::execute_main_phase_action(
        &mut game_state,
        &crate::game_setup::ActionType::SkipMulligan,
        None,
        None,
        None,
        None,
    );
    match result {
        Ok(_) => println!("First player skip successful"),
        Err(e) => eprintln!("First player skip failed: {}", e),
    }
    println!("After: Mulligan player index: {}", game_state.current_mulligan_player_idx);
    println!("After: Phase: {:?}", game_state.current_phase);
    println!("After: Active player: {}", game_state.active_player().id);

    // Test 2: Second player skips mulligan - only if mulligan not complete
    println!("\n--- Test 2: Second player skips mulligan ---");
    if game_state.current_mulligan_player_idx >= 2 {
        println!("Mulligan already complete (index >= 2), skipping second player test");
    } else {
        println!("Before: Mulligan player index: {}", game_state.current_mulligan_player_idx);
        println!("Before: Active player: {}", game_state.active_player().id);
        let actions = game_setup::generate_possible_actions(&game_state);
        println!("Available actions: {}", actions.len());
        for action in &actions {
            println!("  - {}", action.description);
        }

        let ai = ai::AIPlayer::new("Player2AI".to_string());
        let chosen_index = ai.choose_action(&actions);
        let chosen_action = &actions[chosen_index];
        println!("AI chose: {}", chosen_action.description);

        let result = turn::TurnEngine::execute_main_phase_action(
            &mut game_state,
            &chosen_action.action_type,
            chosen_action.parameters.as_ref().and_then(|p| p.card_id),
            chosen_action.parameters.as_ref().and_then(|p| p.card_indices.as_ref()).cloned(),
            chosen_action.parameters.as_ref().and_then(|p| p.stage_area),
            chosen_action.parameters.as_ref().and_then(|p| p.use_baton_touch),
        );
        match result {
            Ok(_) => println!("Second player action successful"),
            Err(e) => eprintln!("Second player action failed: {}", e),
        }
        println!("After: Mulligan player index: {}", game_state.current_mulligan_player_idx);
        println!("After: Phase: {:?}", game_state.current_phase);
        println!("After: Active player: {}", game_state.active_player().id);
    }

    // Test 3: Auto-advance after both done (simulate web server loop)
    println!("\n--- Test 3: Auto-advance after both done (simulate web server loop) ---");
    if game_state.current_mulligan_player_idx >= 2 {
        println!("Before auto-advance: Phase = {:?}", game_state.current_phase);
        
        // Simulate web server's auto-advance loop (EXACTLY as in web_server.rs)
        let mut loop_count = 0;
        let max_loops = 10;
        while loop_count < max_loops {
            loop_count += 1;
            let old_phase = game_state.current_phase.clone();
            
            // Check if we should break (manual phases)
            // Mulligan is handled specially - auto-advance if both players are done
            if matches!(game_state.current_phase,
                crate::game_state::Phase::RockPaperScissors |
                crate::game_state::Phase::ChooseFirstAttacker
            ) {
                println!("Breaking at manual phase: {:?}", game_state.current_phase);
                break;
            }
            
            // Auto-advance automatic phases
            match game_state.current_phase {
                crate::game_state::Phase::Mulligan => {
                    // Mulligan is manual, but auto-advance if both players are done
                    if game_state.current_mulligan_player_idx >= 2 {
                        turn::TurnEngine::advance_phase(&mut game_state);
                    } else {
                        println!("Breaking at Mulligan (not complete)");
                        break;
                    }
                }
                crate::game_state::Phase::Active |
                crate::game_state::Phase::Energy |
                crate::game_state::Phase::Draw |
                crate::game_state::Phase::FirstAttackerPerformance |
                crate::game_state::Phase::SecondAttackerPerformance |
                crate::game_state::Phase::LiveVictoryDetermination => {
                    turn::TurnEngine::advance_phase(&mut game_state);
                }
                crate::game_state::Phase::Main |
                crate::game_state::Phase::LiveCardSet => {
                    println!("Breaking at manual phase: {:?}", game_state.current_phase);
                    break;
                }
                _ => {
                    println!("Breaking at phase: {:?}", game_state.current_phase);
                    break;
                }
            }
            
            // Detect phase not changing (potential infinite loop)
            if game_state.current_phase == old_phase {
                println!("WARNING: Phase not changing after advance_phase: {:?}", game_state.current_phase);
                break;
            }
            
            println!("  Loop {}: {:?} -> {:?}", loop_count, old_phase, game_state.current_phase);
        }
        
        if loop_count >= max_loops {
            println!("ERROR: Infinite loop detected (reached max loops)");
        } else {
            println!("Auto-advance completed in {} iterations", loop_count);
        }
        
        println!("After auto-advance: Phase = {:?}", game_state.current_phase);
        println!("Turn phase: {:?}", game_state.current_turn_phase);
        println!("P1 energy: {} cards", game_state.player1.energy_zone.cards.len());
        println!("P2 energy: {} cards", game_state.player2.energy_zone.cards.len());
    } else {
        println!("Mulligan not complete, index: {}", game_state.current_mulligan_player_idx);
    }

    // Test 4: Simulate web server auto-play loop to detect infinite loops
    println!("\n--- Test 4: Simulate web server auto-play loop ---");
    let mut loop_count = 0;
    let max_loops = 100;
    let mut last_phase = game_state.current_phase.clone();
    
    while loop_count < max_loops {
        loop_count += 1;
        
        // Check if we should break (manual phases)
        if matches!(game_state.current_phase,
            crate::game_state::Phase::RockPaperScissors |
            crate::game_state::Phase::ChooseFirstAttacker |
            crate::game_state::Phase::Mulligan
        ) {
            println!("Breaking at manual phase: {:?}", game_state.current_phase);
            break;
        }
        
        // Auto-advance automatic phases
        match game_state.current_phase {
            crate::game_state::Phase::Active |
            crate::game_state::Phase::Energy |
            crate::game_state::Phase::Draw |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                turn::TurnEngine::advance_phase(&mut game_state);
            }
            crate::game_state::Phase::Main => {
                println!("Reached Main phase, breaking");
                break;
            }
            _ => {
                println!("Breaking at phase: {:?}", game_state.current_phase);
                break;
            }
        }
        
        // Detect phase not changing (potential infinite loop)
        if game_state.current_phase == last_phase {
            println!("WARNING: Phase not changing after advance_phase: {:?}", game_state.current_phase);
            break;
        }
        last_phase = game_state.current_phase.clone();
    }
    
    if loop_count >= max_loops {
        println!("ERROR: Infinite loop detected (reached max loops)");
    } else {
        println!("Auto-play loop completed in {} iterations", loop_count);
        println!("Final phase: {:?}", game_state.current_phase);
    }

    println!("\n=== MULLIGAN TEST COMPLETE ===");
}
