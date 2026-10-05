//! Does a seed actually reproduce a game?
//!
//! `engine/src/compat.rs` maps the engine-wide `HashMap`/`HashSet` to
//! `std::collections`, so every map the engine builds is seeded with its own
//! `RandomState`. Two structurally identical maps therefore enumerate the same
//! keys in *different* orders — including two maps built in the same process,
//! in the same function, microseconds apart. Anywhere an enumeration feeds a
//! decision (an offered-action list, a target-option list, a "first card that
//! matches" scan, a tie-break), the same seed stops meaning the same game.
//!
//! That is not hypothetical. `compat.rs` records that swapping the hasher
//! changed which action a scripted UI picked and roughly doubled the length of
//! a full scripted match, and that nothing caught it: the lockstep test only
//! checks the two link sides agree *with each other*, never against a fixed
//! expectation. So the only symptom of a gameplay change was "a test got
//! slower" — which is how a 3x behaviour change shipped as a perf regression.
//!
//! ## What this test does
//!
//! It plays the SAME seeded game several times over, and requires every run to
//! produce an identical action sequence and an identical board after every
//! single step. Between runs it re-clones the card database, which allocates
//! fresh `HashMap`s and so gets fresh iteration orders — that is what makes
//! this a probe for order dependence rather than a restatement of "the same
//! code produced the same output".
//!
//! ## Why this and not a golden
//!
//! A golden file records whatever the engine does today, including today's
//! arbitrary iteration order, and then asks you to re-record it when you fix
//! the order dependence — which is exactly backwards: it launders the defect
//! into an expectation. This test has no recorded expectations and cannot be
//! satisfied by re-recording anything. It asserts a property ("this seed means
//! this game") that is true or false of the engine right now.
//!
//! When it fails, the message names the first step where two runs diverged and
//! prints both boards at that step, so the offending decision is visible
//! without a debugger.

use rabuka_engine::bin_common;
use rabuka_engine::bot::{registry::BotKind, strategy_v2, strategy_v3};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::replay::observe;
use rabuka_engine::rng::Lcg;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

/// The deck under test. `bin_common::load_deck` reads
/// `web_ui/decks/<name>.txt` and hard-errors if it is absent, so this cannot
/// silently become a different matchup.
const DECK: &str = "5CP3Z idou";

/// How many times to replay the identical seeded game. Each replay starts from
/// a fresh clone of the database, hence a fresh set of iteration orders. Two
/// replays agreeing proves nothing; the point is that all of them agree, and
/// more replays means more independently-ordered maps to disagree.
const REPLAYS: usize = 4;

/// Per-game iteration cap, matching `sim_bench`. A game that reaches it has
/// not finished, which `each_game_reaches_a_terminal_result` reports on
/// separately so a stuck engine cannot masquerade as a stable one.
const ITERATION_CAP: usize = 600;

/// One game's worth of recorded history: the canonical board projection at the
/// top of every loop iteration, and the action chosen at the bottom of it.
#[derive(Debug, PartialEq, Eq)]
struct Replay {
    boards: Vec<String>,
    actions: Vec<String>,
}

/// Same seed derivation as `sim_bench` / `bot_arena`, so this probes the same
/// (engine_seed, policy_seed) pair pair the benchmarks and the PPO pipeline use.
fn game_seeds(base: u32, game: u32) -> (u32, u64) {
    let engine = ((u64::from(base) - 1 + u64::from(game) - 1) % u64::from(u32::MAX) + 1) as u32;
    (engine, 0x5EED_1234_ABCD_0001 ^ u64::from(engine))
}

fn decision_player(gs: &GameState) -> &rabuka_engine::player::Player {
    if gs.has_pending_choice() {
        if gs.can_player_act(0) {
            return &gs.player1;
        }
        if gs.can_player_act(1) {
            return &gs.player2;
        }
    }
    gs.active_player()
}

#[derive(Clone, Copy)]
enum PolicyRoute {
    Action,
    Mulligan,
    LiveSet,
}

fn policy_route(gs: &GameState) -> PolicyRoute {
    if gs.has_pending_choice() {
        return PolicyRoute::Action;
    }
    match gs.current_phase {
        Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => PolicyRoute::Mulligan,
        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => {
            PolicyRoute::LiveSet
        }
        _ => PolicyRoute::Action,
    }
}

/// Restore the global engine RNG on drop (including panic paths), so bot policy
/// code cannot perturb the deal/shuffle stream.
struct RngRestore(u32);
impl Drop for RngRestore {
    fn drop(&mut self) {
        rabuka_engine::rng::restore(self.0);
    }
}

fn policy_call<T>(call: impl FnOnce() -> T) -> T {
    let restore = RngRestore(rabuka_engine::rng::checkpoint());
    let result = call();
    drop(restore);
    result
}

/// Play one seeded game and record every step.
///
/// This mirrors `sim_bench`'s loop — same iteration cap, same stuck-turn guard,
/// same phase routing, same `execute_and_settle` execute path — because that
/// loop is the one the benchmarks measure and the one the PPO pipeline collects
/// from. If this loop and that one disagree, the discrepancy is worth knowing,
/// but a divergence *between replays* is the bug this file exists to find.
fn play(base_db: &CardDatabase, card_numbers: &[String], engine_seed: u32, arena_seed: u64, policy: BotKind) -> Replay {
    // A fresh clone means fresh `HashMap`s, which means a fresh `RandomState`
    // and a fresh iteration order for every map the game goes on to build.
    // Card ids survive the clone, so the replays stay directly comparable.
    let mut db = Arc::new(base_db.clone());
    let (t1, t2) = game_setup::build_two_decks(&mut db, card_numbers, card_numbers)
        .expect("build deck from the real card database");

    rabuka_engine::rng::seed(engine_seed);
    let mut rng = Lcg(arena_seed);
    let mut gs = bin_common::deal_game(&db, &t1, &t2, "p1", "P1", "p2", "P2");

    // Archetype detection once per game; the cheap policies ignore it, exactly
    // as in `sim_bench`, which keeps the seeded stream identical.
    let v2_policy = strategy_v2::V2Policy::default();
    let plan = strategy_v3::V3Plan {
        rush_archetype: false,
    };

    let mut replay = Replay {
        boards: Vec::new(),
        actions: Vec::new(),
    };
    let mut last_turn = 0u8;
    let mut stuck = 0u32;

    for _ in 0..ITERATION_CAP {
        replay.boards.push(
            observe(&gs)
                .expect("board projection serializes")
                .to_string(),
        );

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
            replay.actions.push("<auto>".to_string());
            continue;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            replay.actions.push("<advance_phase>".to_string());
            continue;
        }

        let decision_id = decision_player(&gs).id.clone();
        let me = u8::from(decision_id != gs.player1.id);
        let policy_is_p1 = me == 0;
        let chosen = if policy == BotKind::Random {
            match policy_route(&gs) {
                PolicyRoute::Mulligan => policy_call(|| {
                    BotKind::Random.choose_mulligan(&gs, &actions, &gs.card_database)
                }),
                // RPS: no information — pure Lcg (matches bot_arena).
                _ if gs.current_phase == Phase::RockPaperScissors => {
                    actions[rng.range(actions.len())].clone()
                }
                // S6: when this side won RPS, take second attacker.
                _ if gs.current_phase == Phase::ChooseFirstAttacker => {
                    let won_rps = gs.rps_winner == Some(if policy_is_p1 { 1 } else { 2 });
                    if won_rps {
                        actions
                            .iter()
                            .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                            .cloned()
                            .unwrap_or_else(|| actions[rng.range(actions.len())].clone())
                    } else {
                        actions[rng.range(actions.len())].clone()
                    }
                }
                _ => actions[rng.range(actions.len())].clone(),
            }
        } else {
            match policy_route(&gs) {
                PolicyRoute::Mulligan => policy_call(|| {
                    policy.choose_mulligan(&gs, &actions, &gs.card_database)
                }),
                PolicyRoute::LiveSet => policy_call(|| {
                    policy.choose_live_set(&gs, &actions, &gs.card_database, &v2_policy, &plan)
                }),
                PolicyRoute::Action => {
                    if gs.current_phase == Phase::RockPaperScissors {
                        actions[rng.range(actions.len())].clone()
                    } else if gs.current_phase == Phase::ChooseFirstAttacker {
                        let won_rps = gs.rps_winner == Some(if policy_is_p1 { 1 } else { 2 });
                        if won_rps {
                            actions
                                .iter()
                                .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                                .cloned()
                                .unwrap_or_else(|| actions[rng.range(actions.len())].clone())
                        } else {
                            actions[rng.range(actions.len())].clone()
                        }
                    } else {
                        policy_call(|| policy.choose_action(&gs, &actions, me, &v2_policy, &plan))
                    }
                }
            }
        };

        replay
            .actions
            .push(serde_json::to_string(&chosen).expect("action serializes"));
        bin_common::execute_and_settle(&mut gs, &chosen).expect("the chosen action executes");
    }

    replay
}

/// Both policies' replays, computed once and shared by all three tests.
///
/// Replaying a full game is the expensive part of this file (a whole game is
/// ~600 decisions against the real card database), and all three assertions
/// want the same matrix: two assert agreement, one asserts the games are
/// actually finished. Recomputing it per test tripled the cost for nothing.
static REPLAYS_ONCE: std::sync::OnceLock<Vec<(BotKind, Vec<Replay>)>> = std::sync::OnceLock::new();

fn matrix() -> &'static [(BotKind, Vec<Replay>)] {
    REPLAYS_ONCE.get_or_init(|| {
        [BotKind::Random, BotKind::V1]
            .into_iter()
            .map(|policy| (policy, replays(policy)))
            .collect()
    })
}

fn replays(policy: BotKind) -> Vec<Replay> {
    let base = bin_common::fresh_database();
    let card_numbers = bin_common::load_deck(DECK);
    let (engine_seed, arena_seed) = game_seeds(1, 1);
    (0..REPLAYS)
        .map(|_| play(&base, &card_numbers, engine_seed, arena_seed, policy))
        .collect()
}

/// The assertion proper: replaying one seed must yield one game.
///
/// Both the chosen actions and the board after every step are compared, and the
/// failure reports the first step where replays disagree plus both boards, so
/// the divergence is actionable rather than a bare "arrays differ".
fn assert_replays_identical(policy: BotKind, runs: &[Replay]) {
    let first = &runs[0];
    for (index, run) in runs.iter().enumerate().skip(1) {
        let board_step = first
            .boards
            .iter()
            .zip(&run.boards)
            .position(|(a, b)| a != b);
        let action_step = first
            .actions
            .iter()
            .zip(&run.actions)
            .position(|(a, b)| a != b);
        let step = match (board_step, action_step) {
            (None, None) => None,
            (b, a) => Some(a.into_iter().chain(b).min().unwrap()),
        };
        assert!(
            step.is_none(),
            "policy `{}` is NOT reproducible from a seed: replay 0 and replay {index} \
             diverge at step {step:?} (replay 0 played {} steps, replay {index} played {}).\n\
             \n\
             The engine reached that step from identical inputs, so a decision between them \
             consumed a `HashMap`/`HashSet` iteration order rather than the game state. That \
             is what makes a seed stop naming a game: replays diverge, bug reports stop \
             reproducing, bot training runs are not comparable, and two link peers only agree \
             because they happen to share a process.\n\
             \n\
             --- step {step:?} action, replay 0 ---\n{}\n\
             --- step {step:?} action, replay {index} ---\n{}\n\
             --- step {step:?} board, replay 0 ---\n{}\n\
             --- step {step:?} board, replay {index} ---\n{}\n",
            policy.name(),
            first.actions.len(),
            run.actions.len(),
            first.actions.get(step.unwrap_or(usize::MAX)).map(String::as_str).unwrap_or("<none>"),
            run.actions.get(step.unwrap_or(usize::MAX)).map(String::as_str).unwrap_or("<none>"),
            first.boards.get(step.unwrap_or(usize::MAX)).map(String::as_str).unwrap_or("<none>"),
            run.boards.get(step.unwrap_or(usize::MAX)).map(String::as_str).unwrap_or("<none>"),
        );
    }
}

/// The random policy is the one every benchmark and the PPO collector drive, so
/// it is the one that has to be reproducible for any of their numbers to mean
/// anything.
#[test]
fn random_policy_is_reproducible_from_a_seed() {
    let (policy, runs) = matrix()
        .iter()
        .find(|(p, _)| *p == BotKind::Random)
        .expect("the random policy is in the matrix");
    assert_replays_identical(*policy, runs);
}

/// The scripted policy is the one `compat.rs` names as the victim: swapping the
/// hasher changed which action it picked. A scripted policy picks by scoring
/// the offered actions, so it is the most exposed to an unstable ordering of
/// that offer — and it is what a human plays against on every console.
#[test]
fn scripted_policy_is_reproducible_from_a_seed() {
    let (policy, runs) = matrix()
        .iter()
        .find(|(p, _)| *p == BotKind::V1)
        .expect("the scripted policy is in the matrix");
    assert_replays_identical(*policy, runs);
}

/// A stable-but-useless result is not a pass. Every replayed game has to
/// actually finish, otherwise "all replays agree" could mean "all replays
/// stalled in the same place", which would make the two tests above agree with
/// a completely broken engine.
#[test]
fn each_replayed_game_reaches_a_terminal_result() {
    for (policy, runs) in matrix() {
        for run in runs {
            assert!(
                run.boards.len() < ITERATION_CAP,
                "policy `{}` hit the {ITERATION_CAP}-iteration cap instead of finishing; \
                 a game that never ends cannot demonstrate reproducibility",
                policy.name()
            );
            assert!(
                run.actions.len() > 20,
                "policy `{}` played only {} actions; too few to exercise the engine",
                policy.name(),
                run.actions.len()
            );
        }
    }
}