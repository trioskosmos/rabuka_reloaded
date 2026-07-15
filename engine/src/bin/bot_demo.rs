use std::sync::Arc;

use rabuka_engine::bot::Bot;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards =
        card_loader::CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards");
    let card_database = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).expect("Failed to load fade deck");
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let card_numbers_p1 = card_numbers.clone();
    let card_numbers_p2 = card_numbers.clone();

    let mut p1_template = deck_builder::DeckBuilder::build_deck_from_database(
        &mut Arc::clone(&card_database),
        card_numbers_p1,
    )
    .expect("Failed to build P1 deck");
    let mut p2_template = deck_builder::DeckBuilder::build_deck_from_database(
        &mut Arc::clone(&card_database),
        card_numbers_p2,
    )
    .expect("Failed to build P2 deck");
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut p1_template,
        &mut Arc::clone(&card_database),
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut p2_template,
        &mut Arc::clone(&card_database),
    );

    let mut bot = Bot::new(Arc::clone(&card_database), 0, &card_numbers, &card_numbers);
    // Load trained weights if available
    let weights_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../card_weights.bin".into());
    if std::path::Path::new(&weights_path).exists() {
        bot.network
            .load_weights(&weights_path)
            .expect("load weights");
        eprintln!("Loaded weights from {}", weights_path);
    } else {
        eprintln!("No weights file at {} — using random init", weights_path);
    }

    const NUM_GAMES: u32 = 20;
    let mut p1_wins = 0u32;
    let mut p2_wins = 0u32;
    let mut draws = 0u32;
    let mut total_actions = 0u64;
    let start = std::time::Instant::now();

    for game_idx in 0..NUM_GAMES {
        let mut p1_deck = p1_template.clone();
        let mut p2_deck = p2_template.clone();
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

        let mut gs = GameState::new(player1, player2, Arc::clone(&card_database));
        game_setup::setup_game(&mut gs);

        let mut turns = 0u32;
        let mut stuck = 0u32;
        let mut last_turn = 0u32;
        let mut prev_phase = Phase::RockPaperScissors;
        let mut phase_stuck = 0u32;

        loop {
            turns += 1;
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                break;
            }
            if turns > 500 {
                break;
            }

            if gs.turn_number == last_turn {
                stuck += 1;
                if gs.current_phase == prev_phase {
                    phase_stuck += 1;
                } else {
                    phase_stuck = 0;
                    prev_phase = gs.current_phase.clone();
                }
                if stuck > 300 || phase_stuck > 50 {
                    break;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
            }

            if !gs.has_pending_choice() {
                match gs.current_phase {
                    Phase::Active
                    | Phase::Energy
                    | Phase::Draw
                    | Phase::FirstAttackerPerformance
                    | Phase::SecondAttackerPerformance
                    | Phase::LiveVictoryDetermination => {
                        TurnEngine::advance_phase(&mut gs);
                        continue;
                    }
                    _ => {}
                }
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            let is_bot_move = gs.active_player().id == "player1" && gs.current_phase == Phase::Main;
            let action = if is_bot_move {
                bot.choose_action(&gs)
            } else {
                actions[fastrand(0, actions.len())].clone()
            };

            let params = action.parameters.clone();
            let _ = TurnEngine::execute_main_phase_action(
                &mut gs,
                &action.action_type,
                params.as_ref().and_then(|p| p.card_id),
                params.as_ref().and_then(|p| p.card_indices.clone()),
                params
                    .as_ref()
                    .and_then(|p| p.stage_area.as_deref().and_then(parse_area)),
                params.as_ref().and_then(|p| p.use_baton_touch),
            );
            game_setup::settle_single_player_state(&mut gs);
            total_actions += 1;
            if total_actions % 50 == 0 {
                let elapsed = start.elapsed().as_secs_f64();
                eprintln!(
                    "[{}s] {} actions ({:.0}/s)",
                    elapsed as u64,
                    total_actions,
                    total_actions as f64 / elapsed.max(0.001)
                );
            }
        }

        let p1_zone = gs.player1.success_live_card_zone.cards.len();
        let p2_zone = gs.player2.success_live_card_zone.cards.len();
        println!(
            "Game {}: t{} success {}–{}",
            game_idx + 1,
            gs.turn_number,
            p1_zone,
            p2_zone
        );
        if p1_zone >= 3 && p2_zone <= 2 {
            p1_wins += 1;
        } else if p2_zone >= 3 && p1_zone <= 2 {
            p2_wins += 1;
        } else {
            draws += 1;
        }
    }

    println!(
        "\n{} games — P1 {} P2 {} Draw {}",
        NUM_GAMES, p1_wins, p2_wins, draws
    );
}

fn parse_area(s: &str) -> Option<rabuka_engine::zones::MemberArea> {
    match s {
        "left" => Some(rabuka_engine::zones::MemberArea::LeftSide),
        "center" => Some(rabuka_engine::zones::MemberArea::Center),
        "right" => Some(rabuka_engine::zones::MemberArea::RightSide),
        _ => None,
    }
}

fn fastrand(lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    let n = hi - lo;
    let r = simple_rng();
    lo + (r as usize) % n
}

fn simple_rng() -> u64 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0xdead_beef_cafe_bab5u64) };
    }
    STATE.with(|s| {
        let mut x = s.get();
        if x == 0 {
            x = 1;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x
    })
}
