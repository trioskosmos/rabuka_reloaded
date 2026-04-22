use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rayon::prelude::*;
use std::sync::{Arc, OnceLock};

static GLOBAL_CARD_DATABASE: OnceLock<Arc<CardDatabase>> = OnceLock::new();

fn setup_test_game_state() -> GameState {
    let cards = CardDatabase::new();
    let card_db = Arc::new(cards);
    
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2, card_db);
    game_state.current_phase = Phase::Main;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Add some cards to player1's hand for benchmarking
    for i in 0..10 {
        game_state.player1.hand.cards.push(i);
    }
    
    game_state
}

fn benchmark_generate_possible_actions(c: &mut Criterion) {
    let game_state = setup_test_game_state();
    
    c.bench_function("generate_possible_actions", |b| {
        b.iter(|| {
            game_setup::generate_possible_actions(black_box(&game_state));
        });
    });
}

fn benchmark_check_timing(c: &mut Criterion) {
    c.bench_function("check_timing", |b| {
        b.iter(|| {
            let mut game_state = setup_test_game_state();
            TurnEngine::check_timing(black_box(&mut game_state));
        });
    });
}

fn setup_full_game(deck_name: &str) -> GameState {
    // Load cards from the cards.json file
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    let card_database = Arc::new(CardDatabase::load_or_create(cards));
    
    // Load actual decks from game/decks folder
    let decks = deck_parser::DeckParser::parse_all_decks()
        .expect("Failed to load decks");
    
    // Find the requested deck
    let deck = decks.iter()
        .find(|d| d.name == deck_name)
        .unwrap_or_else(|| {
            eprintln!("Deck '{}' not found, using first available deck", deck_name);
            &decks[0]
        });
    
    let card_numbers = deck_parser::DeckParser::deck_list_to_card_numbers(deck);
    
    let mut player1_deck = deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers.clone())
        .expect("Failed to build deck for Player 1");
    let mut player2_deck = deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers)
        .expect("Failed to build deck for Player 2");
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck, &card_database);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck, &card_database);
    
    player1_deck.shuffle_main_deck();
    player1_deck.shuffle_energy_deck();
    player2_deck.shuffle_main_deck();
    player2_deck.shuffle_energy_deck();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    let mut game_state = GameState::new(player1, player2, card_database);
    game_setup::setup_game(&mut game_state);
    
    game_state
}

#[derive(Debug, Clone, Copy)]
enum GameEndReason {
    Victory,
    Stuck,
}

fn run_single_game_to_completion(mut game_state: GameState) -> (GameState, u64, GameEndReason) {
    let max_iterations = 10000; // High limit to allow long games
    let mut iteration_count = 0;
    let mut action_count = 0;
    let mut end_reason = GameEndReason::Stuck;
    let mut last_turn_number = 0;
    let mut stuck_counter = 0;
    
    while iteration_count < max_iterations {
        iteration_count += 1;
        
        // Check if game is finished
        let game_result = game_state.check_victory();
        if game_result != rabuka_engine::game_state::GameResult::Ongoing {
            end_reason = GameEndReason::Victory;
            break;
        }
        
        // Detect stuck state (very lenient)
        if game_state.turn_number == last_turn_number {
            stuck_counter += 1;
            if stuck_counter > 5000 { // Very high threshold
                end_reason = GameEndReason::Stuck;
                break;
            }
        } else {
            stuck_counter = 0;
            last_turn_number = game_state.turn_number;
        }
        
        // Auto-advance automatic phases
        match game_state.current_phase {
            Phase::Active | Phase::Energy | Phase::Draw | 
            Phase::FirstAttackerPerformance | Phase::SecondAttackerPerformance | 
            Phase::LiveVictoryDetermination => {
                TurnEngine::advance_phase(&mut game_state);
                continue;
            }
            _ => {}
        }
        
        // Get available actions
        let actions = game_setup::generate_possible_actions(&game_state);
        
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut game_state);
            continue;
        }
        
        // Pick random action to avoid repeating same action
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let action = actions.choose(&mut rng).unwrap();
        
        let _ = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &action.action_type,
            action.parameters.as_ref().and_then(|p| p.card_id),
            action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
            action.parameters.as_ref().and_then(|p| p.stage_area),
            action.parameters.as_ref().and_then(|p| p.use_baton_touch),
        );
        action_count += 1;
    }
    
    (game_state, action_count, end_reason)
}

fn run_games_for_duration(card_database: Arc<CardDatabase>, player1_deck_template: deck_builder::Deck, player2_deck_template: deck_builder::Deck, duration_secs: u64) -> (u64, u64, u64) {
    use std::time::{Duration, Instant};
    
    let max_time = Duration::from_secs(duration_secs);
    let start_time = Instant::now();
    let mut total_actions = 0;
    let mut victory_count = 0;
    let mut stuck_count = 0;
    let mut game_count = 0;
    
    while start_time.elapsed() < max_time {
        // Clone and shuffle decks for new game
        let mut p1_deck = player1_deck_template.clone();
        let mut p2_deck = player2_deck_template.clone();
        p1_deck.shuffle_main_deck();
        p1_deck.shuffle_energy_deck();
        p2_deck.shuffle_main_deck();
        p2_deck.shuffle_energy_deck();
        
        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
        let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        player1.set_main_deck(p1_deck.main_deck);
        player1.set_energy_deck(p1_deck.energy_deck);
        player2.set_main_deck(p2_deck.main_deck);
        player2.set_energy_deck(p2_deck.energy_deck);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_setup::setup_game(&mut game_state);
        
        let (_state, action_count, end_reason) = run_single_game_to_completion(game_state);
        
        total_actions += action_count;
        game_count += 1;
        
        match end_reason {
            GameEndReason::Victory => victory_count += 1,
            GameEndReason::Stuck => stuck_count += 1,
        }
    }
    
    (total_actions, victory_count, game_count)
}

fn benchmark_full_game(c: &mut Criterion) {
    let decks = ["aqours_cup", "fade_deck", "hasunosora_cup", "liella_cup", "muse_cup", "nijigaku_cup"];
    
    // Load cards once outside the benchmark loop
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    let card_database = Arc::new(CardDatabase::load_or_create(cards));
    
    // Load all decks once outside the benchmark loop
    let deck_lists = deck_parser::DeckParser::parse_all_decks()
        .expect("Failed to load decks");
    
    for deck_name in decks.iter() {
        // Find the requested deck
        let deck = deck_lists.iter()
            .find(|d| d.name == *deck_name)
            .unwrap_or_else(|| {
                eprintln!("Deck '{}' not found, using first available deck", deck_name);
                &deck_lists[0]
            });
        
        let card_numbers = deck_parser::DeckParser::deck_list_to_card_numbers(deck);
        
        // Pre-build decks for both players
        let mut player1_deck_template = deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers.clone())
            .expect("Failed to build deck for Player 1");
        let mut player2_deck_template = deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers)
            .expect("Failed to build deck for Player 2");
        
        let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck_template, &card_database);
        let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck_template, &card_database);
        
        // Run a quick test to get approximate actions per second for throughput
        let (test_actions, test_victories, test_games) = run_games_for_duration(card_database.clone(), player1_deck_template.clone(), player2_deck_template.clone(), 1);
        
        println!("Deck {}: {} actions, {} victories in {} games (1 second test)", deck_name, test_actions, test_victories, test_games);
        
        let mut group = c.benchmark_group("full_game_10sec");
        group.throughput(Throughput::Elements(test_actions * 10)); // Scale to 10 seconds
        group.bench_with_input(BenchmarkId::from_parameter(deck_name), deck_name, |b, &_deck_name| {
            b.iter(|| {
                let (actions, victories, games) = run_games_for_duration(
                    card_database.clone(),
                    player1_deck_template.clone(),
                    player2_deck_template.clone(),
                    10
                );
                println!("  Run: {} actions, {} victories in {} games", actions, victories, games);
                actions
            });
        });
        group.finish();
    }
}

fn benchmark_full_game_parallel(c: &mut Criterion) {
    let decks = ["aqours_cup", "fade_deck", "hasunosora_cup", "liella_cup", "muse_cup", "nijigaku_cup"];
    
    // Load cards once and store in static global to avoid Arc cloning
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    let card_database = Arc::new(CardDatabase::load_or_create(cards));
    GLOBAL_CARD_DATABASE.set(card_database).expect("Failed to set global card database");
    
    // Get reference to global database
    let card_database = GLOBAL_CARD_DATABASE.get().unwrap();
    
    // Load all decks once outside the benchmark loop
    let deck_lists = deck_parser::DeckParser::parse_all_decks()
        .expect("Failed to load decks");
    
    for deck_name in decks.iter() {
        // Find the requested deck
        let deck = deck_lists.iter()
            .find(|d| d.name == *deck_name)
            .unwrap_or_else(|| {
                eprintln!("Deck '{}' not found, using first available deck", deck_name);
                &deck_lists[0]
            });
        
        let card_numbers = deck_parser::DeckParser::deck_list_to_card_numbers(deck);
        
        // Pre-build decks for both players
        let mut player1_deck_template = deck_builder::DeckBuilder::build_deck_from_database(card_database, card_numbers.clone())
            .expect("Failed to build deck for Player 1");
        let mut player2_deck_template = deck_builder::DeckBuilder::build_deck_from_database(card_database, card_numbers)
            .expect("Failed to build deck for Player 2");
        
        let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck_template, card_database);
        let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck_template, card_database);
        
        // Run a quick test to get approximate actions per second for throughput
        let (test_actions, test_victories, test_games) = run_games_for_duration(Arc::clone(card_database), player1_deck_template.clone(), player2_deck_template.clone(), 1);
        
        println!("Deck {}: {} actions, {} victories in {} games (1 second test)", deck_name, test_actions, test_victories, test_games);
        
        let num_threads = rayon::current_num_threads();
        
        let mut group = c.benchmark_group("full_game_parallel_10sec");
        group.throughput(Throughput::Elements(test_actions * 10)); // Scale to 10 seconds
        group.bench_with_input(BenchmarkId::from_parameter(deck_name), deck_name, |b, &_deck_name| {
            b.iter(|| {
                // Run games in parallel using rayon for 10 seconds total
                let total_actions: u64 = (0..num_threads)
                    .into_par_iter()
                    .map(|_| {
                        run_games_for_duration(
                            Arc::clone(card_database),
                            player1_deck_template.clone(),
                            player2_deck_template.clone(),
                            10 / num_threads as u64 // Divide time among threads
                        ).0
                    })
                    .sum();
                total_actions
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    benchmark_generate_possible_actions,
    benchmark_check_timing,
    benchmark_full_game,
    benchmark_full_game_parallel
);
criterion_main!(benches);
