// Headless game mode for automated testing
// This is separated from core game logic to keep the engine clean
#![allow(dead_code)]

use crate::card_loader;
use crate::deck_builder;
use crate::deck_parser;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use crate::game_setup;
use crate::bot::ai;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
struct CardDisplay {
    card_no: String,
    name: String,
    card_type: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ZoneDisplay {
    cards: Vec<CardDisplay>,
}

#[derive(Serialize, Deserialize, Clone)]
struct StageDisplay {
    left_side: Option<CardDisplay>,
    center: Option<CardDisplay>,
    right_side: Option<CardDisplay>,
}

#[derive(Serialize, Deserialize)]
struct PlayerDisplay {
    hand: ZoneDisplay,
    energy: ZoneDisplay,
    stage: StageDisplay,
    live_zone: ZoneDisplay,
    success_live_card_zone: ZoneDisplay,
    waitroom_count: usize,
    main_deck_count: usize,
    energy_deck_count: usize,
    stage_blades: u32,
    stage_hearts: std::collections::HashMap<String, u32>,
    success_live_score: u32,
}

#[derive(Serialize, Deserialize)]
struct GameStateDisplay {
    turn: u32,
    phase: String,
    turn_phase: String,
    game_result: String,
    player1: PlayerDisplay,
    player2: PlayerDisplay,
    resolution_zone_count: usize,
    p1_cheer_blade_heart_count: u32,
    p2_cheer_blade_heart_count: u32,
    first_attacker: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ActionDisplay {
    action_type: String,
    description: String,
    card_index: Option<usize>,
    card_indices: Option<Vec<usize>>,
    stage_area: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct GameResponse {
    game_state: GameStateDisplay,
    actions: Vec<ActionDisplay>,
    is_finished: bool,
}

fn card_to_display(card_id: i16, card_db: &crate::card::CardDatabase) -> CardDisplay {
    if let Some(card) = card_db.get_card(card_id) {
        CardDisplay {
            card_no: card.card_no.clone(),
            name: card.name.clone(),
            card_type: format!("{:?}", card.card_type),
        }
    } else {
        CardDisplay {
            card_no: format!("unknown:{}", card_id),
            name: format!("Unknown Card {}", card_id),
            card_type: "Unknown".to_string(),
        }
    }
}

fn zone_to_display(card_ids: &[i16], card_db: &crate::card::CardDatabase) -> ZoneDisplay {
    ZoneDisplay {
        cards: card_ids.iter().map(|&id| card_to_display(id, card_db)).collect(),
    }
}

fn stage_to_display(stage: &crate::zones::Stage, card_db: &crate::card::CardDatabase) -> StageDisplay {
    StageDisplay {
        left_side: if stage.stage[0] != -1 { Some(card_to_display(stage.stage[0], card_db)) } else { None },
        center: if stage.stage[1] != -1 { Some(card_to_display(stage.stage[1], card_db)) } else { None },
        right_side: if stage.stage[2] != -1 { Some(card_to_display(stage.stage[2], card_db)) } else { None },
    }
}

fn player_to_display(player: &crate::player::Player, card_db: &crate::card::CardDatabase) -> PlayerDisplay {
    let energy_cards: Vec<(i16, Option<crate::zones::Orientation>)> = player.energy_zone.cards.iter()
        .enumerate()
        .map(|(i, &card_id)| {
            // Simplified: first active_energy_count cards are active, rest are wait
            let orientation = if i < player.energy_zone.active_energy_count {
                Some(crate::zones::Orientation::Active)
            } else {
                Some(crate::zones::Orientation::Wait)
            };
            (card_id, orientation)
        })
        .collect();
    
    let energy_display = ZoneDisplay {
        cards: energy_cards.iter()
            .filter_map(|(card_id, _)| {
                Some(card_to_display(*card_id, card_db))
            })
            .collect(),
    };
    
    let stage_blades = player.stage.total_blades(card_db);
    
    let mut stage_hearts = std::collections::HashMap::new();
    for &card_id in &player.stage.stage[..] {
        if card_id != -1 {
            if let Some(card) = card_db.get_card(card_id) {
                if let Some(ref h) = card.base_heart {
                    for (color, count) in &h.hearts {
                        let color_str = match color {
                            crate::card::HeartColor::Heart00 => "heart00",
                            crate::card::HeartColor::Heart01 => "heart01",
                            crate::card::HeartColor::Heart02 => "heart02",
                            crate::card::HeartColor::Heart03 => "heart03",
                            crate::card::HeartColor::Heart04 => "heart04",
                            crate::card::HeartColor::Heart05 => "heart05",
                            crate::card::HeartColor::Heart06 => "heart06",
                            crate::card::HeartColor::BAll => "b_all",
                            crate::card::HeartColor::Draw => "draw",
                            crate::card::HeartColor::Score => "score",
                        };
                        *stage_hearts.entry(color_str.to_string()).or_insert(0) += count;
                    }
                }
            }
        }
    }
    
    let success_live_score: u32 = player.success_live_card_zone.cards.iter()
        .filter_map(|&id| card_db.get_card(id))
        .map(|c| c.score.unwrap_or(0))
        .sum();
    
    PlayerDisplay {
        hand: zone_to_display(&player.hand.cards, card_db),
        energy: energy_display,
        stage: stage_to_display(&player.stage, card_db),
        live_zone: zone_to_display(&player.live_card_zone.cards, card_db),
        success_live_card_zone: zone_to_display(&player.success_live_card_zone.cards, card_db),
        waitroom_count: player.waitroom.cards.len(),
        main_deck_count: player.main_deck.len(),
        energy_deck_count: player.energy_deck.cards.len(),
        stage_blades,
        stage_hearts,
        success_live_score,
    }
}

fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    let game_result = match game_state.check_victory() {
        crate::game_state::GameResult::FirstAttackerWins => "FirstAttackerWins".to_string(),
        crate::game_state::GameResult::SecondAttackerWins => "SecondAttackerWins".to_string(),
        crate::game_state::GameResult::Draw => "Draw".to_string(),
        crate::game_state::GameResult::Ongoing => "Ongoing".to_string(),
    };
    
    let first_attacker = if game_state.player1.is_first_attacker {
        game_state.player1.id.clone()
    } else {
        game_state.player2.id.clone()
    };
    
    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?}", game_state.current_phase),
        turn_phase: format!("{:?}", game_state.current_turn_phase),
        game_result,
        player1: player_to_display(&game_state.player1, &game_state.card_database),
        player2: player_to_display(&game_state.player2, &game_state.card_database),
        resolution_zone_count: game_state.resolution_zone.cards.len(),
        p1_cheer_blade_heart_count: game_state.player1_cheer_blade_heart_count,
        p2_cheer_blade_heart_count: game_state.player2_cheer_blade_heart_count,
        first_attacker,
    }
}

fn print_game_state(game_state: &GameState) {
    println!("--- Game State ---");
    println!("Turn: {}, Phase: {:?}, Turn Phase: {:?}", 
        game_state.turn_number, game_state.current_phase, game_state.current_turn_phase);
    
    let card_db = &game_state.card_database;
    
    println!("\nPlayer 1 ({}):", game_state.player1.name);
    println!("  Hand: {} cards", game_state.player1.hand.cards.len());
    for (i, card_id) in game_state.player1.hand.cards.iter().enumerate() {
        if let Some(card) = card_db.get_card(*card_id) {
            println!("    [{}] {} ({})", i, card.name, card.card_no);
        } else {
            println!("    [{}] Unknown card {}", i, card_id);
        }
    }
    println!("  Energy Zone: {} cards", game_state.player1.energy_zone.cards.len());
    println!("  Stage:");
    if game_state.player1.stage.stage[0] != -1 {
        if let Some(card) = card_db.get_card(game_state.player1.stage.stage[0]) {
            println!("    Left: {} ({})", card.name, card.card_no);
        } else {
            println!("    Left: Unknown card {}", game_state.player1.stage.stage[0]);
        }
    } else {
        println!("    Left: (empty)");
    }
    if game_state.player1.stage.stage[1] != -1 {
        if let Some(card) = card_db.get_card(game_state.player1.stage.stage[1]) {
            println!("    Center: {} ({})", card.name, card.card_no);
        } else {
            println!("    Center: Unknown card {}", game_state.player1.stage.stage[1]);
        }
    } else {
        println!("    Center: (empty)");
    }
    if game_state.player1.stage.stage[2] != -1 {
        if let Some(card) = card_db.get_card(game_state.player1.stage.stage[2]) {
            println!("    Right: {} ({})", card.name, card.card_no);
        } else {
            println!("    Right: Unknown card {}", game_state.player1.stage.stage[2]);
        }
    } else {
        println!("    Right: (empty)");
    }
    println!("  Live Card Zone: {} cards", game_state.player1.live_card_zone.cards.len());
    println!("  Success Live Card Zone: {} cards", game_state.player1.success_live_card_zone.len());
    println!("  Waitroom: {} cards", game_state.player1.waitroom.cards.len());
    println!("  Main Deck: {} cards", game_state.player1.main_deck.len());
    println!("  Energy Deck: {} cards", game_state.player1.energy_deck.cards.len());
    
    println!("\nPlayer 2 ({}):", game_state.player2.name);
    println!("  Hand: {} cards", game_state.player2.hand.cards.len());
    for (i, card_id) in game_state.player2.hand.cards.iter().enumerate() {
        if let Some(card) = card_db.get_card(*card_id) {
            println!("    [{}] {} ({})", i, card.name, card.card_no);
        } else {
            println!("    [{}] Unknown card {}", i, card_id);
        }
    }
    println!("  Energy Zone: {} cards", game_state.player2.energy_zone.cards.len());
    println!("  Stage:");
    if game_state.player2.stage.stage[0] != -1 {
        if let Some(card) = card_db.get_card(game_state.player2.stage.stage[0]) {
            println!("    Left: {} ({})", card.name, card.card_no);
        } else {
            println!("    Left: Unknown card {}", game_state.player2.stage.stage[0]);
        }
    } else {
        println!("    Left: (empty)");
    }
    if game_state.player2.stage.stage[1] != -1 {
        if let Some(card) = card_db.get_card(game_state.player2.stage.stage[1]) {
            println!("    Center: {} ({})", card.name, card.card_no);
        } else {
            println!("    Center: Unknown card {}", game_state.player2.stage.stage[1]);
        }
    } else {
        println!("    Center: (empty)");
    }
    if game_state.player2.stage.stage[2] != -1 {
        if let Some(card) = card_db.get_card(game_state.player2.stage.stage[2]) {
            println!("    Right: {} ({})", card.name, card.card_no);
        } else {
            println!("    Right: Unknown card {}", game_state.player2.stage.stage[2]);
        }
    } else {
        println!("    Right: (empty)");
    }
    println!("  Live Card Zone: {} cards", game_state.player2.live_card_zone.cards.len());
    println!("  Success Live Card Zone: {} cards", game_state.player2.success_live_card_zone.len());
    println!("  Waitroom: {} cards", game_state.player2.waitroom.cards.len());
    println!("  Main Deck: {} cards", game_state.player2.main_deck.len());
    println!("  Energy Deck: {} cards", game_state.player2.energy_deck.cards.len());
    println!();
}

pub fn run_headless_game() {
    println!("=== Running Headless Game ===\n");

    // Load cards
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };

    // Create CardDatabase from loaded cards
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
    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers1) {
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

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers2) {
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
    
    // Print initial game state
    print_game_state(&game_state);
    
    // Run the game automatically
    let mut turn_count = 0;
    let max_iterations = 100;
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
        
        // Print game state at key points
        if game_state.current_phase == crate::game_state::Phase::Main && turn_count <= 20 {
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
            crate::game_state::GameResult::Ongoing => {
                // Debug: show success live card counts
                if turn_count % 100 == 0 {
                    println!("P1 success cards: {}, P2 success cards: {}", 
                        game_state.player1.success_live_card_zone.len(),
                        game_state.player2.success_live_card_zone.len());
                }
            }
        }
        
        // Auto-advance automatic phases
        match game_state.current_phase {
            crate::game_state::Phase::RockPaperScissors => {
                // Q16: Play RPS to determine turn order
                let actions = crate::game_setup::generate_possible_actions(&game_state);

                println!("Playing RPS...");
                
                let _ = turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &actions[0].action_type,
                    actions[0].parameters.as_ref().and_then(|p| p.card_id),
                    actions[0].parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    actions[0].parameters.as_ref().and_then(|p| p.stage_area),
                    actions[0].parameters.as_ref().and_then(|p| p.use_baton_touch),
                );
            }
            crate::game_state::Phase::ChooseFirstAttacker => {
                // Q16: RPS winner chooses turn order (simplified: always choose first)
                let actions = crate::game_setup::generate_possible_actions(&game_state);

                println!("RPS winner choosing turn order...");
                
                let _ = turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &actions[0].action_type,
                    actions[0].parameters.as_ref().and_then(|p| p.card_id),
                    actions[0].parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    actions[0].parameters.as_ref().and_then(|p| p.stage_area),
                    actions[0].parameters.as_ref().and_then(|p| p.use_baton_touch),
                );
            }
            crate::game_state::Phase::Mulligan => {
                // Mulligan phase - let the AI choose
                let ai = ai::AIPlayer::new("HeadlessAI".to_string());
                let actions = crate::game_setup::generate_possible_actions(&game_state);
                let chosen_index = ai.choose_action(&actions);
                
                println!("Mulligan choice: {}", actions[chosen_index].description);
                
                let _ = turn::TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &actions[chosen_index].action_type,
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.card_id),
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.card_indices.clone()),
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.stage_area),
                    actions[chosen_index].parameters.as_ref().and_then(|p| p.use_baton_touch),
                );
            }
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
                    let chosen_index = ai.choose_action(&actions);
                    
                    println!("Actions available: {}", actions.len());
                    for (i, action) in actions.iter().enumerate() {
                        println!("  [{}] {}", i, action.description);
                    }
                    println!("Choosing: {}", actions[chosen_index].description);
                    
                    // Execute the chosen action
                    let _ = turn::TurnEngine::execute_main_phase_action(&mut game_state, &actions[chosen_index].action_type, actions[chosen_index].parameters.as_ref().and_then(|p| p.card_id), actions[chosen_index].parameters.as_ref().and_then(|p| p.card_indices.clone()), actions[chosen_index].parameters.as_ref().and_then(|p| p.stage_area.clone()), actions[chosen_index].parameters.as_ref().and_then(|p| p.use_baton_touch));
                    
                    // Print state after action for first few iterations
                    if turn_count <= 20 {
                        print_game_state(&game_state);
                    }
                }
            }
            crate::game_state::Phase::LiveCardSet => {
                let p1_live_count = game_state.first_attacker().hand.cards.iter().filter(|c| {
                    game_state.card_database.get_card(**c).map_or(false, |card| card.is_live())
                }).count();
                let p2_live_count = game_state.second_attacker().hand.cards.iter().filter(|c| {
                    game_state.card_database.get_card(**c).map_or(false, |card| card.is_live())
                }).count();
                println!("Auto-advancing live card set phase (P1 live cards: {}, P2 live cards: {})...", p1_live_count, p2_live_count);
                turn::TurnEngine::advance_phase(&mut game_state);
            }
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

pub fn run_interactive_headless() {
    println!("=== Running Automated Headless Mode ===");
    println!("This will play through the game automatically and log game states\n");

    // Load cards
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };

    // Create CardDatabase from loaded cards
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
    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers1) {
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

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers2) {
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

    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck, &card_database);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck, &card_database);

    // Run multiple games for 10 seconds
    let start_time = std::time::Instant::now();
    let duration = std::time::Duration::from_secs(10);
    let mut game_count = 0;
    let mut total_turns = 0;

    while start_time.elapsed() < duration {
        game_count += 1;

        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
        let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

        // Clone decks for each game
        let mut p1_deck = player1_deck.clone();
        let mut p2_deck = player2_deck.clone();
        p1_deck.shuffle_main_deck();
        p1_deck.shuffle_energy_deck();
        p2_deck.shuffle_main_deck();
        p2_deck.shuffle_energy_deck();

        player1.set_main_deck(p1_deck.main_deck);
        player1.set_energy_deck(p1_deck.energy_deck);

        player2.set_main_deck(p2_deck.main_deck);
        player2.set_energy_deck(p2_deck.energy_deck);

        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_setup::setup_game(&mut game_state);
        
        // Automated game loop
        let mut turn_count = 0;
        let mut last_turn_number = 0;
        let mut stuck_counter = 0;
        
        loop {
            turn_count += 1;
            
            // Check if game is finished
            let game_result = game_state.check_victory();
            if game_result != crate::game_state::GameResult::Ongoing {
                break;
            }
            
            // Detect stuck state
            if game_state.turn_number == last_turn_number {
                stuck_counter += 1;
                if stuck_counter > 50 {
                    break;
                }
            } else {
                stuck_counter = 0;
                last_turn_number = game_state.turn_number;
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
                    continue;
                }
                _ => {}
            }
            
            // Get available actions and pick random one
            let actions = game_setup::generate_possible_actions(&game_state);
            
            if actions.is_empty() {
                turn::TurnEngine::advance_phase(&mut game_state);
                continue;
            }
            
            // Pick random available action
            let random_idx = rand::random::<usize>() % actions.len();
            let action = &actions[random_idx];
            execute_action_and_log(&mut game_state, action);
        }
        
        total_turns += turn_count;
    }
    
    let avg_turns = if game_count > 0 { total_turns as f64 / game_count as f64 } else { 0.0 };
    println!("Games played: {}", game_count);
    println!("Average turns per game: {:.2}", avg_turns);
}

fn execute_action_and_log(game_state: &mut GameState, action: &crate::game_setup::Action) {
    let result = turn::TurnEngine::execute_main_phase_action(
        game_state,
        &action.action_type,
        action.parameters.as_ref().and_then(|p| p.card_id),
        action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
        action.parameters.as_ref().and_then(|p| p.stage_area),
        action.parameters.as_ref().and_then(|p| p.use_baton_touch),
    );
    
    match result {
        Ok(_) => {
            // Auto-advance automatic phases
            auto_advance_automatic_phases(game_state);
        }
        Err(e) => {
            println!("Error executing action {}: {}", action.description, e);
        }
    }
}

fn auto_advance_automatic_phases(game_state: &mut GameState) {
    loop {
        let current_phase = game_state.current_phase.clone();
        match current_phase {
            crate::game_state::Phase::Active |
            crate::game_state::Phase::Energy |
            crate::game_state::Phase::Draw |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                turn::TurnEngine::advance_phase(game_state);
            }
            _ => break,
        }
    }
}

fn output_state_and_actions(game_state: &GameState) {
    // Only print concise summary to avoid massive output
    println!("Turn {} | Phase: {:?} | P1 cards: {}/{}/{} | P2 cards: {}/{}/{} | Actions: {}", 
        game_state.turn_number,
        game_state.current_phase,
        game_state.player1.hand.len(),
        game_state.player1.stage.total_blades(&game_state.card_database),
        game_state.player1.success_live_card_zone.len(),
        game_state.player2.hand.len(),
        game_state.player2.stage.total_blades(&game_state.card_database),
        game_state.player2.success_live_card_zone.len(),
        game_setup::generate_possible_actions(game_state).len()
    );
}
