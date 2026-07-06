use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser;
use rabuka_engine::display;
use rabuka_engine::game_setup;
use rabuka_engine::game_state as game_state_mod;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn;
use std::io::{self, Write};
use std::sync::Arc;

fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();

    println!("Starting rabuka harness (desktop)\n");

    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };

    let card_database = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));

    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };

    let deck1 = deck_lists
        .get(0)
        .cloned()
        .unwrap_or_else(|| deck_lists[0].clone());
    let deck2 = deck_lists
        .get(0)
        .cloned()
        .unwrap_or_else(|| deck_lists[0].clone());

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck2);

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database.clone(),
        card_numbers1,
    ) {
        Ok(mut d) => {
            d.shuffle_main_deck();
            d.shuffle_energy_deck();
            d
        }
        Err(e) => {
            eprintln!("Failed to build deck1: {}", e);
            return;
        }
    };

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database.clone(),
        card_numbers2,
    ) {
        Ok(mut d) => {
            d.shuffle_main_deck();
            d.shuffle_energy_deck();
            d
        }
        Err(e) => {
            eprintln!("Failed to build deck2: {}", e);
            return;
        }
    };

    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player1_deck,
        &mut card_database.clone(),
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player2_deck,
        &mut card_database.clone(),
    );

    let mut p1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let mut p2 = Player::new("p2".to_string(), "Player 2".to_string(), false);

    p1.set_main_deck(player1_deck.main_deck);
    p1.set_energy_deck(player1_deck.energy_deck);
    p2.set_main_deck(player2_deck.main_deck);
    p2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(p1, p2, card_database);
    game_setup::setup_game(&mut game_state);

    loop {
        // Print a compact representation of the game state
        let display = display::game_state_to_display(&game_state);
        println!("\n--- Game State (turn {}) ---", game_state.turn_number);
        println!(
            "{}",
            serde_json::to_string_pretty(&display).unwrap_or_default()
        );

        let actions = game_setup::generate_possible_actions(&game_state);
        if actions.is_empty() {
            println!("No legal actions available. Exiting.");
            break;
        }

        println!("\nLegal actions:");
        for (i, a) in actions.iter().enumerate() {
            println!("  [{}] {}", i, a.description);
        }
        println!("Enter action index (or q to quit): ");
        print!("> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();
        if line == "q" || line == "quit" {
            break;
        }
        let idx: usize = match line.parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };
        if idx >= actions.len() {
            println!("Index out of range");
            continue;
        }

        let action = &actions[idx];
        let params = action.parameters.clone();

        let res = turn::TurnEngine::execute_main_phase_action(
            &mut game_state,
            &action.action_type,
            params.as_ref().and_then(|p| p.card_id),
            params.as_ref().and_then(|p| p.card_indices.clone()),
            params
                .as_ref()
                .and_then(|p| p.stage_area.as_ref().and_then(|s| s.parse().ok())),
            params.as_ref().and_then(|p| p.use_baton_touch),
        );

        match res {
            Ok(_) => {
                println!("Action applied.");
                // Auto-advance automatic phases until a human decision or pending choice
                let _ = settle_single_player_state(&mut game_state);
            }
            Err(e) => {
                println!("Action failed: {}", e);
            }
        }
    }

    println!("Exiting harness.");
}

fn is_automatic_phase(game_state: &GameState) -> bool {
    matches!(
        game_state.current_phase,
        game_state_mod::Phase::Active
            | game_state_mod::Phase::Energy
            | game_state_mod::Phase::Draw
            | game_state_mod::Phase::FirstAttackerPerformance
            | game_state_mod::Phase::SecondAttackerPerformance
            | game_state_mod::Phase::LiveVictoryDetermination
    )
}

fn is_live_card_set_phase(game_state: &GameState) -> bool {
    matches!(
        game_state.current_phase,
        game_state_mod::Phase::LiveCardSetFirstAttacker
            | game_state_mod::Phase::LiveCardSetSecondAttacker
    )
}

fn settle_single_player_state(game_state: &mut GameState) -> Result<(), String> {
    loop {
        if game_state.has_pending_choice() {
            break;
        }

        if is_automatic_phase(game_state) {
            turn::TurnEngine::advance_phase(game_state);
        } else if is_live_card_set_phase(game_state) {
            break;
        } else {
            break;
        }
    }
    Ok(())
}
