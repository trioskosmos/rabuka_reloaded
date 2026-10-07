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
// 3 = both seats recorded, with a trailing `seat` byte per step.
// Bumped from 2 when RL self-play started recording P2 decisions too; readers
// must accept both, because every v2 corpus on disk ends before that byte.
const TRAJECTORY_VERSION: u32 = 3;

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
    // The opponent is either the ladder bot, or - when `PPO_OPP_BOT=mirror` /
    // unset and no opponent is requested - nobody (pure self-play).
    let opp_name = std::env::var("PPO_OPP_BOT").unwrap_or_else(|_| "v7".to_string());
    let mirror = opp_name == "mirror" || opp_name == "self";
    let opp_kind = ladder_opponent();
    // A ladder rung must always be present: `PPO_OPP_BOT=v1` means "beat v1",
    // so the opponent seat is filled whether or not `PPO_OPPONENT` was set.
    let opponent_present = !mirror
        && (opponent_v7 || std::env::var_os("PPO_OPP_BOT").is_some());
    eprintln!(
        "PPO opponent: {} (mirror={}, engaged={})",
        opp_name,
        mirror,
        opponent_present
    );
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../training/ppo_trajectories.bin".into());
    let weights_path = std::env::args().nth(3);

    let mut network = PolicyNet::new();
    if let Some(ref wp) = weights_path {
        network.load_weights(wp).expect("load weights");
        eprintln!("Loaded weights from {}", wp);
    } else {
        // A fresh rollout must use the SAME initialisation the trainer will
        // warm-start from. Leaving this at `PolicyNet::new()` (all zeros, uniform
        // policy) while the trainer starts from Xavier means the importance
        // ratio compares two different policies on iteration 0 - PPO's central
        // assumption broken before the first update, which diverged to
        // |dlog ratio| ~ 1e12 and a NaN value loss.
        network.init_random(0xC0FFEE);
        eprintln!("No weights -- Xavier-initialised policy (seed 0xC0FFEE)");
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
    let mut net_wins_total = 0u32;
    let mut net_losses_total = 0u32;
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
            if game_idx == 0
                && (step_i % 50 == 0 || std::env::var_os("PPO_TRACE_PHASE").is_some())
            {
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
            // With a ladder opponent engaged, the network takes the seat the ladder bot
            // does NOT. Measured, not assumed: with `PPO_SEATS=1` the net was put
            // on P1 and the ladder bot on P2, so every trajectory was the bot
            // losing and 0/120 games were won by P1 - the ladder produced zero
            // positive reward while appearing to train.
            let net_seat: Option<u8> = if imitation {
                Some(0)
            } else if mirror {
                // Pure self-play: both seats are the network.
                None
            } else if record_seat == 2 {
                // Both seats recorded, so seat choice does not matter.
                None
            } else {
                Some(1)
            };
            let should_record = !imitation
                && match net_seat {
                    None => true,
                    Some(s) => me == s,
                };

            let me_is_net = match net_seat {
                None => true,
                Some(s) => me == s,
            };
            // Structural phases are not learned decisions, and letting the net
            // pick them at random is what made every game a Draw at turn 1: RPS
            // ties re-pick until `stuck > 200`, and an attacker-order coin flip
            // adds nothing. The arena intercepts exactly these phases before
            // dispatch for the same reason (`bot_arena.rs`), so the collector
            // must too - otherwise RL is spending its whole budget learning
            // rock-paper-scissors and the games never start.
            //
            // The mulligan is included for the LADDER BOT's seat only. It is a
            // multi-step interaction (select, then confirm), and routing the
            // net's own mulligan through a single `choose_opponent_action` call
            // left it re-picking the same step until `stuck > 200` - the game
            // never started, which is why every game ended as a Draw at turn 1.
            // The mulligan is a multi-step interaction (select cards, then
            // confirm). Whichever seat holds it, resolve it deterministically:
            // pick the first selection-style action, then confirm/skip. Routing
            // it through the sampled policy re-picked the same non-advancing
            // step until `stuck > 200`, so every game ended as a Draw at turn 1
            // with 0 wins - a silent total failure that looked like "the ladder
            // is hard".
            let mulligan = matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::MulliganFirstAttacker
                    | rabuka_engine::game_state::Phase::MulliganSecondAttacker
            );
            let structural = mulligan
                || matches!(
                    gs.current_phase,
                    rabuka_engine::game_state::Phase::RockPaperScissors
                        | rabuka_engine::game_state::Phase::ChooseFirstAttacker
                );

            // The live-card set is ALSO multi-step (select a life, possibly several, then
            // confirm) for whichever seat holds it. Same failure mode as the
            // mulligan: sampled one step at a time, it re-picks a
            // non-advancing selection until `stuck > 200`. When the NET holds the
            // live set, hand the decision to the ladder bot's live-set logic
            // rather than sampling it - a live set is one of the two things this
            // bot family is actually good at, and the net cannot learn it well
            // enough to avoid hanging the phase.
            let live_set = matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
                    | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker
            );

            let action = if mulligan {
                // Prefer the ladder bot's own mulligan logic when the bot holds
                // the mulligan; for the net's seat, just take the confirm/skip
                // step so the phase terminates.
                let target = if !me_is_net {
                    choose_opponent_action(&gs, &actions, me, plan, opp_kind)
                } else {
                    actions
                        .iter()
                        .find(|a| {
                            matches!(
                                a.action_type,
                                rabuka_engine::game_setup::ActionType::ConfirmMulligan
                                    | rabuka_engine::game_setup::ActionType::SkipMulligan
                            )
                        })
                        .cloned()
                        .unwrap_or_else(|| actions[0].clone())
                };
                target
            } else if structural || (live_set && should_record) {
                choose_opponent_action(&gs, &actions, me, plan, opp_kind)
            } else if should_record && actions.len() > 1 {
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
                let (_logits, value) = network.evaluate_actions(&enc_state, &action_encs);
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
            } else if opponent_present {
                choose_opponent_action(&gs, &actions, me, plan, opp_kind)
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

        // Progress is reported from the NETWORK's seat, not P1's. With a ladder opponent
        // engaged the network is P2, so the old P1-only counter read 0 wins for
        // 120 games while the network was in fact winning - which is exactly how
        // a broken metric hides a working ladder.
        let net_p2 = !imitation && !mirror && net_seat_had_opponent();
        if std::env::var_os("PPO_TRACE_END").is_some() {
            eprintln!(
                "  END p1z={} p2z={} steps={} turns={} result={:?}",
                p1z,
                p2z,
                trajectory.len(),
                gs.turn_number,
                gs.game_result
            );
        }
        let net_wins = if net_p2 { p2z >= 3 } else { p1z >= 3 };
        if net_wins {
            net_wins_total += 1;
        } else if net_p2 {
            if p1z >= 3 {
                net_losses_total += 1;
            }
        }

        if p1z >= 3 {
            p1_wins += 1;
        }

        // Discount the outcome back along the trajectory. Two corrections matter:
        //
        //  - The discount is applied WITHIN a seat's contiguous run, not across
        //    the whole trajectory. With both seats recorded, consecutive steps
        //    can belong to different players and resetting at the boundary keeps
        //    gamma counting decisions rather than interleaved half-turns.
        //  - Each step's reward is signed by ITS seat. `final_reward` is from
        //    P1's perspective, so P2's steps take -final_reward.
        let discount: f32 = std::env::var("PPO_DISCOUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|d: &f32| d.is_finite() && *d > 0.0 && *d <= 1.0)
            .unwrap_or(0.99);
        let sign_for = |seat: u8| if seat == 0 { 1.0f32 } else { -1.0f32 };
        // Walk backwards, restarting the discount whenever the seat changes.
        let mut carry = final_reward;
        let mut prev_seat = u8::MAX;
        for step in trajectory.iter_mut().rev() {
            if step.seat != prev_seat {
                carry = final_reward;
                prev_seat = step.seat;
            } else {
                carry *= discount;
            }
            step.reward = carry * sign_for(step.seat);
        }
        if let Some(last) = trajectory.last_mut() {
            last.reward = final_reward * sign_for(last.seat);
            last.done = true;
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
            // Trajectory version 3 appends the seat. Version 2 files end here,
            // so a v3 reader must tolerate both or every previously collected
            // corpus becomes unreadable.
            if TRAJECTORY_VERSION >= 3 {
                let _ = out.write_all(&[step.seat]);
            }
        }
        total_steps += trajectory.len() as u64;

        if game_idx % 25 == 0 || game_idx == num_games - 1 {
            eprintln!(
                "Game {}: {} steps (total={}, net_wins={})",
                game_idx + 1,
                trajectory.len(),
                total_steps,
                net_wins_total
            );
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "\nDone: {} games, {} steps, {:.0} steps/s, net_win_rate={:.3} ({}W/{}L), p1_wins={}",
        num_games,
        total_steps,
        total_steps as f64 / elapsed.max(0.001),
        (net_wins_total as f64 / (net_wins_total + net_losses_total).max(1) as f64),
        net_wins_total, net_losses_total,
        p1_wins
    );
    eprintln!("Trajectories written to {}", out_path);
    // Save the policy that generated the data. PPO is only correct if the
    // trainer warm-starts from exactly this network; without it the ratio on
    // iteration 0 is between two different policies and the update diverges.
    let policy_out = std::env::var("PPO_SAVE_POLICY")
        .unwrap_or_else(|_| out_path.replace(".bin", "_policy.bin"));
    network.save_weights(&policy_out).expect("save policy");
    eprintln!("Policy that generated this data saved to {}", policy_out);
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

/// Resolve the ladder opponent from `PPO_OPP_BOT`.
///
/// The ladder exists because mirror self-play is degenerate on this game: two
/// near-uniform policies draw EVERY game (measured 0 wins in 500), so reward is
/// 0 everywhere and PPO has no gradient. A weak opponent that *loses* supplies
/// the signal a mirror cannot. Climb v1 -> v2 -> ... by holding each rung near
/// 50% and only advancing when beaten.
///
/// Defaults to `v7` so existing runs keep their opponent.
/// True when the network plays a single seat against an engaged ladder bot.
/// Centralised so the seat decision in the action loop and the win/loss
/// accounting cannot disagree - they did once, and it reported 0 wins for 120
/// games of a network that was winning as P2.
fn net_seat_had_opponent() -> bool {
    if std::env::var("PPO_BEHAVIOR").is_ok_and(|v| v == "v7") {
        return false;
    }
    let opp = std::env::var("PPO_OPP_BOT").unwrap_or_else(|_| "v7".to_string());
    if opp == "mirror" || opp == "self" {
        return false;
    }
    std::env::var("PPO_SEATS")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(2)
        == 1
}

fn ladder_opponent() -> BotKind {    let name = std::env::var("PPO_OPP_BOT").unwrap_or_else(|_| "v7".to_string());
    match name.as_str() {
        "v1" => BotKind::V1,
        "v2" => BotKind::V2,
        "v3" => BotKind::V3,
        "v4" => BotKind::V4,
        "v5" => BotKind::V5,
        "v6" => BotKind::V6,
        "v7plain" => BotKind::V7Plain,
        "v7" | "v7rollout" => BotKind::V7,
        "v8" => BotKind::V8,
        // v9 is the bot the learned value function is actually being trained FOR.
        // Generating from v8 or a random mirror produces states v9 never visits,
        // which is the wrong distribution for a leaf that will only ever be
        // consulted while v9 searches.
        "v9" => BotKind::V9,
        "v10" => BotKind::V10,
        other => {
            eprintln!("PPO_OPP_BOT={other} is not a known bot; falling back to v7");
            BotKind::V7
        }
    }
}

fn choose_opponent_action(
    gs: &GameState,
    actions: &[rabuka_engine::game_setup::Action],
    me: u8,
    plan: &strategy_v3::V3Plan,
    kind: BotKind,
) -> rabuka_engine::game_setup::Action {
    let policy = strategy_v2::V2Policy::default();
    // RPS and attacker order are structural phases the arena also intercepts
    // before dispatch; here they are decided deterministically per seat so the
    // two sides do not both reach for the same gesture (a tie re-picks forever).
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
        rabuka_engine::game_state::Phase::ChooseFirstAttacker => {
            // The RPS winner takes second, matching the arena's default so a
            // ladder measurement is comparable with arena numbers.
            let won = gs.rps_winner == Some(if me == 0 { 1 } else { 2 });
            let target = if won {
                rabuka_engine::game_setup::ActionType::ChooseSecondAttacker
            } else {
                rabuka_engine::game_setup::ActionType::ChooseFirstAttacker
            };
            actions
                .iter()
                .find(|a| a.action_type == target)
                .cloned()
                .or_else(|| {
                    actions
                        .iter()
                        .find(|a| {
                            a.action_type
                                == rabuka_engine::game_setup::ActionType::ChooseSecondAttacker
                        })
                        .cloned()
                })
                .unwrap_or_else(|| choose_v7_fallback(actions))
        }
        rabuka_engine::game_state::Phase::MulliganFirstAttacker
        | rabuka_engine::game_state::Phase::MulliganSecondAttacker => {
            kind.choose_mulligan(gs, actions, &gs.card_database)
        }
        rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
        | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker => {
            kind.choose_live_set(gs, actions, &gs.card_database, &policy, plan)
        }
        _ => kind.choose_action(gs, actions, me, &policy, plan),
    }
}

fn choose_v7_action(
    gs: &GameState,
    actions: &[rabuka_engine::game_setup::Action],
    me: u8,
    plan: &strategy_v3::V3Plan,
) -> rabuka_engine::game_setup::Action {
    choose_opponent_action(gs, actions, me, plan, BotKind::V7)
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