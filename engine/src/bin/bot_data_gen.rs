use rand::Rng;
use std::fs::File;
use std::io::Write;
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
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).expect("cards.json");
    let card_database = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).expect("fade deck");
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let cn1 = card_numbers.clone();
    let cn2 = card_numbers.clone();

    let mut t1 =
        deck_builder::DeckBuilder::build_deck_from_database(&mut Arc::clone(&card_database), cn1)
            .expect("p1 deck");
    let mut t2 =
        deck_builder::DeckBuilder::build_deck_from_database(&mut Arc::clone(&card_database), cn2)
            .expect("p2 deck");
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut t1,
        &mut Arc::clone(&card_database),
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut t2,
        &mut Arc::clone(&card_database),
    );

    let num_games: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../training_data.bin".into());
    let weights_path = std::env::args().nth(3);
    let mut out = File::create(&out_path).expect("create output file");

    let bot = weights_path.as_ref().map(|wp| {
        let mut b = Bot::new(Arc::clone(&card_database), 0, &card_numbers, &card_numbers);
        b.network.load_weights(wp).expect("load weights");
        eprintln!("Using bot with weights from {}", wp);
        b
    });

    let mut total_examples: u64 = 0;
    let mut p1_wins = 0u32;
    let mut p2_wins = 0u32;
    let mut draws = 0u32;

    for game_idx in 0..num_games {
        let mut d1 = t1.clone();
        let mut d2 = t2.clone();
        d1.shuffle_main_deck();
        d1.shuffle_energy_deck();
        d2.shuffle_main_deck();
        d2.shuffle_energy_deck();

        let mut p1 = Player::new("p1".into(), "P1".into(), true);
        let mut p2 = Player::new("p2".into(), "P2".into(), false);
        p1.set_main_deck(d1.main_deck);
        p1.set_energy_deck(d1.energy_deck);
        p2.set_main_deck(d2.main_deck);
        p2.set_energy_deck(d2.energy_deck);

        let mut gs = GameState::new(p1, p2, Arc::clone(&card_database));
        game_setup::setup_game(&mut gs);

        let mut examples: Vec<Example> = Vec::with_capacity(200);
        let mut step_count = 0u32;
        let mut last_turn = 0u32;
        let mut stuck = 0u32;

        for _t in 0..500 {
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

            step_count += 1;
            let ex = Example::capture(&gs);
            examples.push(ex);

            let action = if let Some(ref _b) = bot {
                if gs.active_player().id == "player1" && gs.current_phase == Phase::Main {
                    // Use the smart heuristic — no clones, no NN
                    let heuristic =
                        rabuka_engine::bot::evaluation::pick_rollout_action(&actions, &gs);
                    // We record the state for training; the action choice is NOT random
                    heuristic
                } else {
                    actions[rand::thread_rng().gen_range(0..actions.len())].clone()
                }
            } else {
                actions[rand::thread_rng().gen_range(0..actions.len())].clone()
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
        }

        let p1z = gs.player1.success_live_card_zone.cards.len();
        let p2z = gs.player2.success_live_card_zone.cards.len();
        let margin = ((p1z as f32 - p2z as f32) / 3.0).clamp(-1.0, 1.0);

        if p1z >= 3 && p2z <= 2 {
            p1_wins += 1;
        } else if p2z >= 3 && p1z <= 2 {
            p2_wins += 1;
        } else {
            draws += 1;
        }

        for (i, ex) in examples.iter().enumerate() {
            let steps_rem = (step_count - i as u32) as u16;
            let hand_len = ex.my_hand.len() as u8;
            let _ = out.write_all(&[hand_len]);
            for &c in &ex.my_hand {
                let _ = out.write_all(&c.to_le_bytes());
            }
            for &c in &ex.my_stage {
                let _ = out.write_all(&c.to_le_bytes());
            }
            for &c in &ex.opp_stage {
                let _ = out.write_all(&c.to_le_bytes());
            }
            let _ = out.write_all(&margin.to_le_bytes());
            let _ = out.write_all(&steps_rem.to_le_bytes());
        }
        total_examples += examples.len() as u64;

        if game_idx % 50 == 0 || game_idx == num_games - 1 || p1z >= 3 || p2z >= 3 {
            eprintln!(
                "Game {}: success {}–{} (examples={}, total={})",
                game_idx + 1,
                p1z,
                p2z,
                examples.len(),
                total_examples
            );
        }
    }

    eprintln!("\nDone: {} games, {} examples", num_games, total_examples);
    eprintln!("P1 {} P2 {} Draw {}", p1_wins, p2_wins, draws);
    eprintln!("Data written to {}", out_path);
}

struct Example {
    my_hand: Vec<i16>,
    my_stage: [i16; 3],
    opp_stage: [i16; 3],
}

impl Example {
    fn capture(gs: &GameState) -> Self {
        Self {
            my_hand: gs.player1.hand.cards.to_vec(),
            my_stage: gs.player1.stage.stage,
            opp_stage: gs.player2.stage.stage,
        }
    }
}

fn parse_area(s: &str) -> Option<rabuka_engine::zones::MemberArea> {
    match s {
        "left" => Some(rabuka_engine::zones::MemberArea::LeftSide),
        "center" => Some(rabuka_engine::zones::MemberArea::Center),
        "right" => Some(rabuka_engine::zones::MemberArea::RightSide),
        _ => None,
    }
}
