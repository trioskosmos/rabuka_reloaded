use rabuka_engine::card_loader;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state as game_state_mod;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn;
use std::sync::Arc;

fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();

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

    let player1_deck;
    let player2_deck;
    match game_setup::build_two_decks(&card_database, &card_numbers1, &card_numbers2) {
        Ok((mut d1, mut d2)) => {
            d1.shuffle_main_deck();
            d1.shuffle_energy_deck();
            d2.shuffle_main_deck();
            d2.shuffle_energy_deck();
            player1_deck = d1;
            player2_deck = d2;
        }
        Err(e) => {
            eprintln!("Failed to build decks: {}", e);
            return;
        }
    }

    let mut p1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let mut p2 = Player::new("p2".to_string(), "Player 2".to_string(), false);

    p1.set_main_deck(player1_deck.main_deck);
    p1.set_energy_deck(player1_deck.energy_deck);
    p2.set_main_deck(player2_deck.main_deck);
    p2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(p1, p2, card_database);
    game_setup::setup_game(&mut game_state);
    println!("Initial game_result: {:?}", game_state.game_result);
    let max_iterations = 10_000;
    let mut log_mod = 100;

    for i in 0..max_iterations {
        if i % log_mod == 0 {
            println!(
                "[iter {}] Phase: {}, P1 hand: {}, P2 hand: {}, result: {:?}",
                i,
                game_state.current_phase,
                game_state.player1.hand.cards.len(),
                game_state.player2.hand.cards.len(),
                game_state.game_result
            );

            if i >= 1000 {
                log_mod = 1000;
            }
            if i >= 5000 {
                log_mod = 5000;
            }
        }

        let actions = game_setup::generate_possible_actions(&game_state);
        if actions.is_empty() {
            println!("No legal actions. Stopping at iteration {}.", i);
            break;
        }

        let idx = pick_action_for_phase(&game_state, &actions);

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
                game_state.reset_loop_detection();
                settle_automatic(&mut game_state);
            }
            Err(e) => {
                eprintln!("Action failed at iteration {}: {}", i, e);
                break;
            }
        }

        if game_state.game_result != game_state_mod::GameResult::Ongoing {
            if i % log_mod != 0 {
                println!(
                    "[iter {}] Phase: {}, P1 hand: {}, P2 hand: {}, result: {:?}",
                    i + 1,
                    game_state.current_phase,
                    game_state.player1.hand.cards.len(),
                    game_state.player2.hand.cards.len(),
                    game_state.game_result
                );
            }
            println!("Game ended. Result: {:?}", game_state.game_result);
            break;
        }
    }

    if game_state.game_result == game_state_mod::GameResult::Ongoing {
        println!("Reached max iterations without game ending.");
    }
}

fn pick_action_for_phase(game_state: &GameState, actions: &[game_setup::Action]) -> usize {
    match game_state.current_phase {
        game_state_mod::Phase::Main => {
            if actions.len() > 2 {
                0
            } else {
                0
            }
        }
        game_state_mod::Phase::RockPaperScissors => {
            if game_state.player1_rps_choice.is_none() {
                0
            } else {
                1
            }
        }
        game_state_mod::Phase::ChooseFirstAttacker => 0,
        game_state_mod::Phase::MulliganFirstAttacker
        | game_state_mod::Phase::MulliganSecondAttacker => actions.len() - 1,
        game_state_mod::Phase::LiveCardSetFirstAttacker
        | game_state_mod::Phase::LiveCardSetSecondAttacker => {
            if actions.len() <= 2 {
                0
            } else if game_state.live_card_selected_indices.is_empty() {
                1
            } else {
                actions.len() - 1
            }
        }
        _ => 0,
    }
}

fn settle_automatic(game_state: &mut GameState) {
    loop {
        if game_state.has_pending_choice() {
            break;
        }
        if game_state.game_result != game_state_mod::GameResult::Ongoing {
            break;
        }
        if matches!(
            game_state.current_phase,
            game_state_mod::Phase::Active
                | game_state_mod::Phase::Energy
                | game_state_mod::Phase::Draw
                | game_state_mod::Phase::FirstAttackerPerformance
                | game_state_mod::Phase::SecondAttackerPerformance
                | game_state_mod::Phase::LiveVictoryDetermination
        ) {
            let _ = turn::TurnEngine::advance_phase(game_state);
        } else {
            break;
        }
    }
}
