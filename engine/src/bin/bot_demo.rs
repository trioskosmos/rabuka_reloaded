use rand::Rng;
use std::sync::Arc;

use rabuka_engine::bot::Bot;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, Phase};
use rabuka_engine::turn::TurnEngine;

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let card_database = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).unwrap();
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let (t1, t2) =
        game_setup::build_two_decks(&card_database, &card_numbers, &card_numbers).unwrap();

    let mut bot = Bot::new(Arc::clone(&card_database), 0, &card_numbers, &card_numbers);
    let weights_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../td_weights.bin".into());
    if std::path::Path::new(&weights_path).exists() {
        bot.network.load_weights(&weights_path).unwrap();
    }

    const NUM_GAMES: u8 = 100;
    let mut p1_wins = 0u32;
    let mut p2_wins = 0u32;
    let mut draws = 0u32;
    let mut total_actions = 0u64;
    let mut bot_moves = 0u64;
    let start = std::time::Instant::now();

    for game_idx in 0..NUM_GAMES {
        let mut gs = rabuka_engine::bin_common::deal_game(
            &card_database,
            &t1,
            &t2,
            "player1",
            "P1",
            "player2",
            "P2",
        );

        let mut stuck = 0u32;
        let mut last_turn = 0u8;

        for _ in 0..500 {
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                break;
            }

            if gs.turn_number == last_turn {
                stuck += 1;
                if stuck > 200 {
                    break;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
            }

            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            let is_bot = gs.active_player().id == "player1" && gs.current_phase == Phase::Main;
            if is_bot && actions.len() > 1 {
                eprintln!(
                    "turn={} {} actions: {}",
                    gs.turn_number,
                    actions.len(),
                    actions
                        .iter()
                        .map(|a| format!("{:?}", a.action_type))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            let active_id = gs.active_player().id.clone();
            let is_bot = active_id == "player1" && gs.current_phase == Phase::Main;
            if game_idx == 0 && total_actions < 10 {
                eprintln!(
                    "turn={} phase={:?} active={} is_bot={} nactions={}",
                    gs.turn_number,
                    gs.current_phase,
                    active_id,
                    is_bot,
                    actions.len()
                );
            }
            let action = if is_bot {
                bot_moves += 1;
                bot.choose_action(&gs)
            } else {
                actions[fastrand(0, actions.len())].clone()
            };

            let _ = rabuka_engine::bin_common::execute_and_settle(&mut gs, &action);
            total_actions += 1;
        }

        match rabuka_engine::bin_common::classify_winner(&gs) {
            rabuka_engine::bin_common::GameOutcome::P1Win => p1_wins += 1,
            rabuka_engine::bin_common::GameOutcome::P2Win => p2_wins += 1,
            _ => draws += 1,
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "\n{} games 遯ｶ繝ｻP1 {} P2 {} Draw {} ({} moves, {:.0}/s, bot_moves={})",
        NUM_GAMES,
        p1_wins,
        p2_wins,
        draws,
        total_actions,
        total_actions as f64 / elapsed.max(0.001),
        bot_moves
    );
}

fn fastrand(lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    rand::thread_rng().gen_range(lo..hi)
}