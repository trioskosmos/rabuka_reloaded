use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use rabuka_engine::bot::encoding::{ActionEncoding, EncodedState, ACTION_ENC_DIM};
use rabuka_engine::bot::neural::PolicyNet;
use rabuka_engine::bot::{registry::BotKind, strategy_v2, strategy_v3, PublicObservation};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder::Deck;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::turn::TurnEngine;
use rand::Rng;

const TRAJECTORY_MAGIC: u32 = 0x5242504B;
const TRAJECTORY_VERSION: u32 = 2;

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
    // Which seat made this decision. RL trains on BOTH seats' self-play steps
    // from one trajectory, so the terminal reward must be sign-flipped for P2's
    // steps - otherwise every P2 decision would be optimised toward P1 winning.
    seat: u8,
}

fn main() {
    rabuka_engine::core::runtime_flags::set_training_mode(true);
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let mut db = Arc::new(CardDatabase::load_or_create(cards));

    // Deck to imitate on. Defaults to the deck the arena scores against
// (`5CP3Z idou`); the previous hardcoded `fade deck` meant every trajectory
// collected here described a different game from the one the learned policy is
// ever evaluated on, which is the same class of mistake `bin_common::load_deck`
// panics on ("a typo used to measure a different game than the one named").
let deck_name = std::env::var("PPO_DECK").unwrap_or_else(|_| "5CP3Z idou".to_string());
    let deck_path = std::path::Path::new("../web_ui/decks").join(format!("{deck_name}.txt"));
    let deck = DeckParser::parse_deck_file(&deck_path)
        .unwrap_or_else(|e| panic!("parse deck {}: {e}", deck_path.display()));
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    // `u32`, not `u8`: at u8 a request for 1500 games parsed as 244 and the run
    // stopped at 200 games without any error, so a corpus silently ~7x smaller
    // than asked for looks exactly like a successful run.
let num_games: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let imitation = std::env::var("PPO_BEHAVIOR").is_ok_and(|value| value == "v7");
    let opponent_v7 = std::env::var("PPO_OPPONENT").is_ok_and(|value| value == "v7");
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../training/ppo_trajectories.bin".into());
    let weights_path = std::env::args().nth(3);

    let mut network = PolicyNet::new();
    if let Some(ref wp) = weights_path {
        network.load_weights(wp).expect("load weights");
        eprintln!("Loaded weights from {}", wp);
    } else {
        eprintln!("No weights  -- random policy");
    }

    let (t1, t2) =
        game_setup::build_two_decks(&mut db, &card_numbers, &card_numbers).unwrap();

    let mut out = File::create(&out_path).expect("create output");
    let state_dim = EncodedState::state_dim();
    let state_dim_u32 = state_dim as u32;
    out.write_all(&TRAJECTORY_MAGIC.to_le_bytes()).unwrap();
    out.write_all(&TRAJECTORY_VERSION.to_le_bytes()).unwrap();
    out.write_all(&state_dim_u32.to_le_bytes()).unwrap();
    out.write_all(&(ACTION_ENC_DIM as u32).to_le_bytes()).unwrap();

    let mut total_steps: u64 = 0;
    let mut p1_wins = 0u32;
    let start = std::time::Instant::now();

    for game_idx in 0..num_games {
        let mut gs = setup_game(&db, &t1, &t2);
        let plan_p1 = strategy_v3::V3Plan::detect(&gs, 0, &db);
        let plan_p2 = strategy_v3::V3Plan::detect(&gs, 1, &db);
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

            // Which seat is acting. A pending CHOICE can belong to the player who is not
            // the active player, so `active_player()` alone would mis-attribute
            // steps - the same trap `bot_arena::decision_player` exists to avoid.
            let acting_p1 = if gs.can_player_act(0) {
                true
            } else if gs.can_player_act(1) {
                false
            } else {
                gs.active_player().id == gs.player1.id
            };
            let me = u8::from(!acting_p1);
            let plan = if acting_p1 { &plan_p1 } else { &plan_p2 };

            // RL self-play: BOTH seats play the network and both are recorded,
            // so a round of self-play yields two trajectories' worth of signal
            // and the agent learns from its own games rather than from a
            // teacher's choices. This is the collection mode PPO needs; the
            // imitation mode above is kept for behaviour-cloning comparison.
            let record_seat = std::env::var("PPO_SEATS")
                .ok()
                .and_then(|v| v.parse::<u8>().ok())
                .unwrap_or(2); // 1 = P1 only, 2 = both
            let should_record = !imitation && (record_seat >= 2 || acting_p1);

            let action = if should_record && actions.len() > 1 {
                let obs = PublicObservation::from_state(&gs, me);
                let enc_state = network.encode_state(&obs);
                let action_encs: Vec<ActionEncoding> = actions
                    .iter()
                    .map(|a| ActionEncoding::from_action(a, &obs))
                    .collect();
                let (logits, value) = network.evaluate_actions(&enc_state, &action_encs);
                let probs = softmax(&logits);
                // Sample from the policy. `PPO_EPS` mixes in a uniform action so
                // a fresh random policy still explores every legal move - with a
                // near-zero policy the sampler never leaves the argmax and RL has
                // nothing to improve from.
                let eps: f32 = std::env::var("PPO_EPS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .filter(|e: &f32| e.is_finite() && *e >= 0.0 && *e <= 1.0)
                    .unwrap_or(0.10);
                let (chosen_idx, log_prob) = if rand::thread_rng().gen::<f32>() < eps {
                    let idx = rand::thread_rng().gen_range(0..actions.len());
                    let behavior_prob = (1.0 - eps) * probs[idx] + eps / actions.len() as f32;
                    (idx, behavior_prob.max(1e-10).ln())
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
                    seat: me,
                });
                actions[chosen_idx].clone()
            } else if imitation && acting_p1 && actions.len() > 1 {
                let obs = PublicObservation::from_state(&gs, 0);
                let enc_state = network.encode_state(&obs);
                let action_encs: Vec<ActionEncoding> = actions
                    .iter()
                    .map(|a| ActionEncoding::from_action(a, &obs))
                    .collect();
                let (logits, value) = network.evaluate_actions(&enc_state, &action_encs);
                let target = choose_v7_action(&gs, &actions, 0, &plan_p1);
                let idx = action_index(&actions, &target);
                trajectory.push(Step {
                    state_flat: enc_state.flatten(),
                    actions: action_encs,
                    chosen_idx: idx as u16,
                    old_log_prob: 0.0,
                    old_value: value,
                    reward: 0.0,
                    done: false,
                    seat: me,
                });
                actions[idx].clone()
            } else if opponent_v7 {
                choose_v7_action(&gs, &actions, me, plan)
            } else {
                let idx = rand::thread_rng().gen_range(0..actions.len());
                actions[idx].clone()
            };

            execute_action(&mut gs, &action);
        }

        // Reward propagation.
        //
        // A win is only known at the end of the game, so the terminal reward
        // lands on the LAST step and everything before it is discounted back
        // from there. This is the standard terminal-MDP convention and PPO's
        // GAE handles it, but it does mean every step's `reward` is a function
        // of the outcome rather than something local - so the value head has to
        // learn the discounting, which is exactly what it is for.
        //
        // `PPO_DISCOUNT` (default 0.99) is applied backwards from the terminal
        // step. At 1.0 every step would carry the full game outcome, which
        // makes the problem a bandit with no credit assignment at all.
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

        let discount: f32 = std::env::var("PPO_DISCOUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|d: &f32| d.is_finite() && *d > 0.0 && *d <= 1.0)
            .unwrap_or(0.99);
        if let Some(last) = trajectory.last_mut() {
            last.reward = final_reward;
            last.done = true;
        }
        // Walk backwards so step t carries gamma^(n-1-t) * outcome. Kept as a
        // running product rather than a powf per step.
        let mut carry = final_reward;
        for step in trajectory.iter_mut().rev() {
            carry *= discount;
            step.reward = carry;
        }
        // The terminal step keeps the raw outcome (not gamma-damped), which is
        // what "done" means to GAE.
        if let Some(last) = trajectory.last_mut() {
            last.reward = final_reward;
        }

        // Write trajectory: [n_steps:u32] [step_1] [step_2] ...
        let n_steps = trajectory.len() as u32;
        let _ = out.write_all(&n_steps.to_le_bytes());
        for step in &trajectory {
            let n_actions = step.actions.len() as u32;
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
                let _ = out.write_all(&[act.target_zone, act.position, act.ability_index]);
                let _ = out.write_all(&act.choice_option.to_le_bytes());
                let _ = out.write_all(&[act.flags, act.final_cost]);
                let _ = out.write_all(&act.card_index.to_le_bytes());
                let _ = out.write_all(&[act.card_indices_count, 0, 0, 0]);
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

fn choose_v7_fallback(actions: &[rabuka_engine::game_setup::Action]) -> rabuka_engine::game_setup::Action {
    actions.first().cloned().unwrap_or(rabuka_engine::game_setup::Action {
        description: "pass".into(),
        description_ja: None,
        action_type: rabuka_engine::game_setup::ActionType::Pass,
        parameters: None,
        selected: None,
    })
}

fn choose_v7_action(
    gs: &GameState,
    actions: &[rabuka_engine::game_setup::Action],
    me: u8,
    plan: &strategy_v3::V3Plan,
) -> rabuka_engine::game_setup::Action {
    let policy = strategy_v2::V2Policy::default();
    match gs.current_phase {
        rabuka_engine::game_state::Phase::RockPaperScissors => {
            let wanted = if me == 0 {
                rabuka_engine::game_setup::ActionType::RockChoice
            } else {
                rabuka_engine::game_setup::ActionType::PaperChoice
            };
            actions
                .iter()
                .find(|action| action.action_type == wanted)
                .cloned()
                .unwrap_or_else(|| choose_v7_fallback(actions))
        }
        rabuka_engine::game_state::Phase::ChooseFirstAttacker => actions
            .iter()
            .find(|action| {
                action.action_type == rabuka_engine::game_setup::ActionType::ChooseFirstAttacker
            })
            .cloned()
            .unwrap_or_else(|| choose_v7_fallback(actions)),
        rabuka_engine::game_state::Phase::MulliganFirstAttacker
        | rabuka_engine::game_state::Phase::MulliganSecondAttacker => {
            BotKind::V7.choose_mulligan(gs, actions, &gs.card_database)
        }
        rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
        | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker => {
            BotKind::V7.choose_live_set(gs, actions, &gs.card_database, &policy, plan)
        }
        _ => BotKind::V7.choose_action(gs, actions, me, &policy, plan),
    }
}

fn action_index(actions: &[rabuka_engine::game_setup::Action], target: &rabuka_engine::game_setup::Action) -> usize {
    let target = serde_json::to_string(target).ok();
    actions
        .iter()
        .position(|action| serde_json::to_string(action).ok() == target)
        .unwrap_or(0)
}

fn execute_action(gs: &mut GameState, action: &rabuka_engine::game_setup::Action) {
    let _ = rabuka_engine::bin_common::execute_and_settle(gs, action);
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