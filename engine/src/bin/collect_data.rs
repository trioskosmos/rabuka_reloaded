use rabuka_engine::core::constants::U8Count;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::turn::TurnEngine;

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let mut db = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).unwrap();
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let num_games: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../train_data.bin".into());

    let (t1, t2) =
        game_setup::build_two_decks(&mut db, &card_numbers, &card_numbers).unwrap();

    let mut out = File::create(&out_path).unwrap();
    let mut total: u64 = 0;
    let start = std::time::Instant::now();
    let program_seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    for game_idx in 0..num_games {
        let mut gs = rabuka_engine::bin_common::deal_game(
            &db,
            &t1,
            &t2,
            "p1",
            "P1",
            "p2",
            "P2",
        );

        let mut pending: Option<Example> = None;
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        let mut rng_state: u64 =
            program_seed ^ ((game_idx as u64).wrapping_mul(0x9E3779B97F4A7C15));

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

            let is_p1 = gs.active_player().id == "p1";
            let record = is_p1 && !matches!(gs.current_phase, Phase::RockPaperScissors);

            // Random action
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            let idx = (rng_state as usize) % actions.len();
            let action = actions[idx].clone();

            if record {
                let ex = Example::capture(&gs, &action);

                // Write previous example with shaped reward
                if let Some(prev) = pending.take() {
                    let success = gs.player1.success_live_card_zone.cards.len() as f32;
                    let base_reward = success - prev.success_before as f32;
                    let shaped = match prev.action_type_idx {
                        14 => 0.1f32,
                        15 => 0.05f32,
                        _ => 0.0f32,
                    };
                    let next_state = State::capture(&gs);
                    write_example(&mut out, &prev, base_reward + shaped, Some(&next_state));
                    total += 1;
                }
                pending = Some(ex);
            }

            // Execute action
            let _ = rabuka_engine::bin_common::execute_and_settle(&mut gs, &action);
        }

        // Write final example
        if let Some(ex) = pending.take() {
            let success = gs.player1.success_live_card_zone.cards.len();
            let state = State::capture(&gs);
            let shaped = get_shaped_reward(ex.action_type_idx);
            write_example(&mut out, &ex, success as f32 + shaped, Some(&state));
            total += 1;
        }

        if game_idx % 50 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            eprintln!(
                "Game {}/{}: {} examples ({:.0}/s)",
                game_idx + 1,
                num_games,
                total,
                total as f64 / elapsed.max(0.001)
            );
        }
    }
    eprintln!("\nDone: {} games, {} examples", num_games, total);
}

fn get_shaped_reward(action_type_idx: u8) -> f32 {
    match action_type_idx {
        14 => 0.1,  // PlayMemberToStage
        15 => 0.05, // UseAbility
        _ => 0.0,
    }
}

struct State {
    hand: Vec<i16>,
    stage: [i16; 3],
    opp_stage: [i16; 3],
}
impl State {
    fn capture(gs: &GameState) -> Self {
        Self {
            hand: gs.player1.hand.cards.to_vec(),
            stage: gs.player1.stage.stage,
            opp_stage: gs.player2.stage.stage,
        }
    }
}

struct Example {
    state: State,
    action_card_id: i16,
    action_type_idx: u8,
    success_before: u8,
}

impl Example {
    fn capture(gs: &GameState, action: &game_setup::Action) -> Self {
        let action_card_id = action
            .parameters
            .as_ref()
            .and_then(|p| p.card_id)
            .unwrap_or(0);
        let at = rabuka_engine::bot::encoding::action_type_index(&action.action_type);
        Self {
            state: State::capture(gs),
            action_card_id,
            action_type_idx: at,
            success_before: gs.player1.success_live_card_zone.cards.len().u8_count(),
        }
    }
}

fn write_example(f: &mut File, ex: &Example, reward: f32, next_state: Option<&State>) {
    let s = &ex.state;
    let _ = f.write_all(&[s.hand.len().u8_count()]);
    for &c in &s.hand {
        let _ = f.write_all(&c.to_le_bytes());
    }
    for &c in &s.stage {
        let _ = f.write_all(&c.to_le_bytes());
    }
    for &c in &s.opp_stage {
        let _ = f.write_all(&c.to_le_bytes());
    }
    let _ = f.write_all(&ex.action_card_id.to_le_bytes());
    let _ = f.write_all(&[ex.action_type_idx]);
    let _ = f.write_all(&reward.to_le_bytes());
    let ns = next_state.unwrap_or(s);
    let _ = f.write_all(&[ns.hand.len().u8_count()]);
    for &c in &ns.hand {
        let _ = f.write_all(&c.to_le_bytes());
    }
    for &c in &ns.stage {
        let _ = f.write_all(&c.to_le_bytes());
    }
    for &c in &ns.opp_stage {
        let _ = f.write_all(&c.to_le_bytes());
    }
}