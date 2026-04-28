// Full game test with AI playing both sides
use crate::card_loader;
use crate::deck_builder::DeckBuilder;
use crate::deck_parser;
use crate::game_setup;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use crate::bot::ai;

pub fn test_full_game() {
    println!("=== TESTING FULL GAME WITH AI ===\n");

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

    // Automated game loop with detailed logging
    let mut turn_count = 0;
    let max_turns = 50; // Lower limit for testing - AI is conservative
    let mut last_phase = game_state.current_phase.clone();
    let mut phase_stuck_count = 0;

    while turn_count < max_turns {
        turn_count += 1;

        // Check if game is finished
        let game_result = game_state.check_victory();
        if game_result != crate::game_state::GameResult::Ongoing {
            println!("\n=== GAME FINISHED ===");
            println!("Result: {:?}", game_result);
            println!("Total turns: {}", turn_count);
            println!("P1 success cards: {}", game_state.player1.success_live_card_zone.len());
            println!("P2 success cards: {}", game_state.player2.success_live_card_zone.len());
            return;
        }

        // Log phase every 10 turns
        if turn_count % 10 == 0 {
            println!("Turn {}: Phase = {:?}, TurnPhase = {:?}", 
                turn_count, game_state.current_phase, game_state.current_turn_phase);
        }

        // Log every action with details
        println!("Turn {}: Phase = {:?}, TurnPhase = {:?}, ActivePlayer = {}, LiveCardSetPlayer = {}", 
            turn_count, 
            game_state.current_phase, 
            game_state.current_turn_phase,
            game_state.active_player().id,
            game_state.current_live_card_set_player
        );

        // Detect phase stuck (potential infinite loop)
        if game_state.current_phase == last_phase {
            phase_stuck_count += 1;
            if phase_stuck_count > 20 {
                println!("\n=== ERROR: PHASE STUCK ===");
                println!("Stuck in phase: {:?}", game_state.current_phase);
                println!("Turn count: {}", turn_count);
                println!("P1 success cards: {}", game_state.player1.success_live_card_zone.len());
                println!("P2 success cards: {}", game_state.player2.success_live_card_zone.len());
                return;
            }
        } else {
            phase_stuck_count = 0;
            last_phase = game_state.current_phase.clone();
        }

        // Auto-advance automatic phases (matching web_server.rs logic exactly)
        match game_state.current_phase {
            crate::game_state::Phase::RockPaperScissors |
            crate::game_state::Phase::ChooseFirstAttacker => {
                // Manual phases - AI will play (don't auto-advance)
            }
            crate::game_state::Phase::MulliganP1Turn |
            crate::game_state::Phase::MulliganP2Turn |
            crate::game_state::Phase::Mulligan => {
                // Mulligan is manual - AI will play (don't auto-advance)
            }
            crate::game_state::Phase::LiveCardSetP1Turn |
            crate::game_state::Phase::LiveCardSetP2Turn |
            crate::game_state::Phase::LiveCardSet => {
                // LiveCardSet is manual - AI will play (don't auto-advance)
            }
            crate::game_state::Phase::Main => {
                // Main phase - manual phase, let AI play (don't auto-advance)
            }
            crate::game_state::Phase::Active |
            crate::game_state::Phase::Energy |
            crate::game_state::Phase::Draw |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                // Automatic phases - always auto-advance
                turn::TurnEngine::advance_phase(&mut game_state);
                continue;
            }
            _ => {}
        }

        // Get available actions and pick one using AI
        let actions = game_setup::generate_possible_actions(&game_state);

        if actions.is_empty() {
            println!("  No actions available - auto-advancing");
            // Try to advance phase
            turn::TurnEngine::advance_phase(&mut game_state);
            continue;
        }

        println!("  Available actions: {}", actions.len());
        for (i, action) in actions.iter().enumerate() {
            println!("    [{}] {}", i, action.description);
        }

        // Use AI to choose action
        let ai = ai::AIPlayer::new("AI".to_string());
        let chosen_index = ai.choose_action(&actions);
        let chosen_action = &actions[chosen_index];

        println!("  Chosen action: [{}] {}", chosen_index, chosen_action.description);

        // Execute action
        let result = turn::TurnEngine::execute_main_phase_action(
            &mut game_state,
            &chosen_action.action_type,
            chosen_action.parameters.as_ref().and_then(|p| p.card_id),
            chosen_action.parameters.as_ref().and_then(|p| p.card_indices.as_ref()).cloned(),
            chosen_action.parameters.as_ref().and_then(|p| p.stage_area),
            chosen_action.parameters.as_ref().and_then(|p| p.use_baton_touch),
        );

        if let Err(e) = result {
            eprintln!("Action execution error: {}", e);
            eprintln!("Action: {}", chosen_action.description);
            eprintln!("Phase: {:?}", game_state.current_phase);
            return;
        }

        println!("  After action: Phase = {:?}", game_state.current_phase);
    }

    println!("\n=== GAME STOPPED (MAX TURNS REACHED) ===");
    println!("Total turns: {}", turn_count);
    println!("Final phase: {:?}", game_state.current_phase);
    println!("P1 success cards: {}", game_state.player1.success_live_card_zone.len());
    println!("P2 success cards: {}", game_state.player2.success_live_card_zone.len());
}
