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

    let deck1 = deck_lists.get(0).cloned().unwrap_or_else(|| deck_lists[0].clone());
    let deck2 = deck_lists.get(0).cloned().unwrap_or_else(|| deck_lists[0].clone());

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck2);

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database.clone(),
        card_numbers1,
    ) {
        Ok(mut d) => { d.shuffle_main_deck(); d.shuffle_energy_deck(); d }
        Err(e) => { eprintln!("Failed to build deck1: {}", e); return; }
    };

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database.clone(),
        card_numbers2,
    ) {
        Ok(mut d) => { d.shuffle_main_deck(); d.shuffle_energy_deck(); d }
        Err(e) => { eprintln!("Failed to build deck2: {}", e); return; }
    };

    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player1_deck, &mut card_database.clone(),
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player2_deck, &mut card_database.clone(),
    );

    let mut p1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let mut p2 = Player::new("p2".to_string(), "Player 2".to_string(), false);
    p1.set_main_deck(player1_deck.main_deck);
    p1.set_energy_deck(player1_deck.energy_deck);
    p2.set_main_deck(player2_deck.main_deck);
    p2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(p1, p2, card_database);
    game_setup::setup_game(&mut game_state);

    // How many consecutive pre-game actions we've auto-resolved (RPS / attacker
    // choice / mulligan).  Capped so we don't loop forever if something breaks.
    let mut auto_turns = 0;

    loop {
        // --- identical to web_server: drive through automatic phases ---
        game_setup::settle_single_player_state(&mut game_state);

        if game_state.game_result != game_state_mod::GameResult::Ongoing {
            println!("\nGame Over: {:?}", game_state.game_result);
            break;
        }

        let display = display::game_state_to_display(&game_state);
        println!(
            "\n--- Game State (turn {}) phase={:?} ---",
            game_state.turn_number, game_state.current_phase
        );
        println!("{}", serde_json::to_string_pretty(&display).unwrap_or_default());

        let actions = game_setup::generate_possible_actions(&game_state);
        if actions.is_empty() {
            println!(
                "No legal actions (no auto-phase either). phase={:?}. Exiting.",
                game_state.current_phase
            );
            break;
        }

        println!("\nLegal actions:");
        for (i, a) in actions.iter().enumerate() {
            println!("  [{}] {:?} — {}", i, a.action_type, a.description);
        }

        // Auto-pilot through the pre-game ceremony so the harness drops you
        // straight at Main phase ready to play.
        let pregame = matches!(
            game_state.current_phase,
            game_state_mod::Phase::RockPaperScissors
                | game_state_mod::Phase::ChooseFirstAttacker
                | game_state_mod::Phase::MulliganFirstAttacker
                | game_state_mod::Phase::MulliganSecondAttacker
        );
        if pregame && auto_turns < 20 {
            auto_turns += 1;
            // Always pick the last listed action:
            //   RPS         → ScissorsChoice
            //   Attacker    → ChooseFirstAttacker (or Second)
            //   Mulligan    → SkipMulligan
            let idx = actions.len() - 1;
            let action = &actions[idx];
            println!("[AUTO] {} — {}", idx, action.description);
            execute_action(&mut game_state, action);
            continue;
        }

        println!("Enter action index (or q to quit):");
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
            Err(_) => { println!("Invalid input"); continue; }
        };
        if idx >= actions.len() {
            println!("Index out of range"); continue;
        }

        let action = &actions[idx];
        if execute_action(&mut game_state, action) {
            auto_turns = 0;
        }
    }

    println!("Exiting harness.");
}

/// Execute one action against the game state — exactly as the web server does.
/// Returns true on success, false on error.
fn execute_action(game_state: &mut GameState, action: &rabuka_engine::game_setup::Action) -> bool {
    let params = action.parameters.clone();
    let res = turn::TurnEngine::execute_main_phase_action(
        game_state,
        &action.action_type,
        params.as_ref().and_then(|p| p.card_id),
        params.as_ref().and_then(|p| p.card_indices.clone()),
        params.as_ref().and_then(|p| p.stage_area.as_ref().and_then(|s| s.parse().ok())),
        params.as_ref().and_then(|p| p.use_baton_touch),
    );
    match res {
        Ok(_) => { println!("Action applied."); true }
        Err(e) => { println!("Action failed: {}", e); false }
    }
}
