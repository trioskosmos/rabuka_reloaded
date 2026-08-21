use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use rabuka_engine::bot::encoding::{action_target_zone, ActionEncoding, EncodedState};
use rabuka_engine::bot::neural::PolicyNet;
use rabuka_engine::bot::PublicObservation;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder::Deck;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use rand::Rng;

/// A single step in a trajectory, with ALL legal actions recorded.
struct Step {
    state_flat: Vec<f32>,
    // All legal actions
    actions: Vec<ActionEncoding>,
    // Index of chosen action
    chosen_idx: u16,
    old_log_prob: f32,
    old_value: f32,
    reward: f32,
    done: bool,
}

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let mut db = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).unwrap();
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let num_games: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../training/ppo_trajectories.bin".into());
    let weights_path = std::env::args().nth(3);

    let mut network = PolicyNet::new();
    if let Some(ref wp) = weights_path {
        network.load_weights(wp).expect("load weights");
        eprintln!("Loaded weights from {}", wp);
    } else {
        eprintln!("No weights 窶・random policy");
    }

    let (mut t1, mut t2) =
        game_setup::build_two_decks(&mut db, &card_numbers, &card_numbers).unwrap();

    let mut out = File::create(&out_path).expect("create output");
    let state_dim = EncodedState::state_dim();
    let state_dim_u32 = state_dim as u8;

    let mut total_steps: u64 = 0;
    let mut p1_wins = 0u32;
    let start = std::time::Instant::now();

    for game_idx in 0..num_games {
        let mut gs = setup_game(&db, &mut t1, &mut t2);
        let mut trajectory: Vec<Step> = Vec::with_capacity(200);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;

        if game_idx == 0 {
            eprintln!(
                "  Game 0 after setup: turn={} phase={:?} result={:?}",
                gs.turn_number, gs.current_phase, gs.game_result
            );
        }

        for step_i in 0..500 {
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

            // Debug: log phase transitions for game 0
            if game_idx == 0 && step_i % 50 == 0 {
                eprintln!(
                    "  [{:3}] turn={} phase={:?} pending={} ap={}",
                    step_i,
                    gs.turn_number,
                    gs.current_phase,
                    gs.has_pending_choice(),
                    gs.active_player().id
                );
            }

            // Auto-advance phases that need no player choice
            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            // P1 decision: record trajectory for ALL phases P1 makes choices
            let p1_phase = gs.active_player().id == "p1"
                && matches!(
                    gs.current_phase,
                    Phase::RockPaperScissors
                        | Phase::ChooseFirstAttacker
                        | Phase::MulliganFirstAttacker
                        | Phase::MulliganSecondAttacker
                        | Phase::Main
                        | Phase::LiveCardSetFirstAttacker
                        | Phase::LiveCardSetSecondAttacker
                );

            let action = if p1_phase && actions.len() > 1 {
                // Use policy network for ALL P1 decisions
                let obs = PublicObservation::from_state(&gs, 0);
                let enc_state = network.encode_state(&obs);
                let action_encs: Vec<ActionEncoding> = actions
                    .iter()
                    .map(|a| {
                        let target = action_target_zone(a, &obs);
                        ActionEncoding {
                            action_type: action_type_idx(&a.action_type),
                            target_card_id: a
                                .parameters
                                .as_ref()
                                .and_then(|p| p.card_id)
                                .unwrap_or(0),
                            target_zone: target.zone as u8,
                            position: target.position,
                        }
                    })
                    .collect();

                let (logits, value) = network.evaluate_actions(&enc_state, &action_encs);
                let probs = softmax(&logits);
                let use_noise = weights_path.is_none() || rand::thread_rng().gen_range(0..100) < 10;
                let (chosen_idx, log_prob) = if use_noise {
                    let idx = rand::thread_rng().gen_range(0..actions.len());
                    (idx, (probs[idx] + 1e-10).ln())
                } else {
                    let idx = sample_from_probs(&probs);
                    (idx, (probs[idx] + 1e-10).ln())
                };

                trajectory.push(Step {
                    state_flat: enc_state.flatten(),
                    actions: action_encs,
                    chosen_idx: chosen_idx as u16,
                    old_log_prob: log_prob,
                    old_value: value,
                    reward: 0.0,
                    done: false,
                });
                actions[chosen_idx].clone()
            } else {
                let idx = rand::thread_rng().gen_range(0..actions.len());
                actions[idx].clone()
            };

            execute_action(&mut gs, &action);
        }

        // Final reward
        let p1z = gs.player1.success_live_card_zone.cards.len();
        let p2z = gs.player2.success_live_card_zone.cards.len();
        let final_reward = if p1z >= 3 && p2z <= 2 {
            1.0f32
        } else if p2z >= 3 && p1z <= 2 {
            -1.0f32
        } else {
            0.0f32
        };

        if p1z >= 3 {
            p1_wins += 1;
        }

        if let Some(last) = trajectory.last_mut() {
            last.reward = final_reward;
            last.done = true;
        }

        // Write trajectory: [n_steps:u8] [step_1] [step_2] ...
        let n_steps = trajectory.len() as u8;
        let _ = out.write_all(&n_steps.to_le_bytes());
        for step in &trajectory {
            let n_actions = step.actions.len() as u16;
            // state
            let _ = out.write_all(&state_dim_u32.to_le_bytes());
            for &v in &step.state_flat {
                let _ = out.write_all(&v.to_le_bytes());
            }
            // number of actions
            let _ = out.write_all(&n_actions.to_le_bytes());
            // each action: [action_type:u8, card_id:i16, zone:u8, pos:u8] = 5 bytes
            for act in &step.actions {
                let _ = out.write_all(&[act.action_type]);
                let _ = out.write_all(&act.target_card_id.to_le_bytes());
                let _ = out.write_all(&[act.target_zone, act.position]);
            }
            // chosen_idx, old_log_prob, old_value, reward, done
            let _ = out.write_all(&step.chosen_idx.to_le_bytes());
            let _ = out.write_all(&step.old_log_prob.to_le_bytes());
            let _ = out.write_all(&step.old_value.to_le_bytes());
            let _ = out.write_all(&step.reward.to_le_bytes());
            let _ = out.write_all(&[step.done as u8]);
        }
        total_steps += trajectory.len() as u64;

        if game_idx % 25 == 0 || game_idx == num_games - 1 {
            eprintln!(
                "Game {}: {} steps (total={}, p1_wins={})",
                game_idx + 1,
                trajectory.len(),
                total_steps,
                p1_wins
            );
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "\nDone: {} games, {} steps, {:.0} steps/s, p1_wins={}",
        num_games,
        total_steps,
        total_steps as f64 / elapsed.max(0.001),
        p1_wins
    );
    eprintln!("Trajectories written to {}", out_path);
}

fn setup_game(db: &Arc<CardDatabase>, t1: &Deck, t2: &Deck) -> GameState {
    rabuka_engine::bin_common::deal_game(db, t1, t2, "p1", "P1", "p2", "P2")
}

fn execute_action(gs: &mut GameState, action: &rabuka_engine::game_setup::Action) {
    let _ = rabuka_engine::bin_common::execute_and_settle(gs, action);
}

fn action_type_idx(t: &ActionType) -> u8 {
    rabuka_engine::bot::encoding::action_type_index(t)
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max_val = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|&l| (l - max_val).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

fn sample_from_probs(probs: &[f32]) -> usize {
    let r: f32 = rand::thread_rng().gen_range(0.0..1.0);
    let mut cum = 0.0f32;
    for (i, &p) in probs.iter().enumerate() {
        cum += p;
        if r < cum {
            return i;
        }
    }
    probs.len() - 1
}