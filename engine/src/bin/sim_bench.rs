//! Canonical full-game throughput bench (Phase-0 baseline metric).
//!
//! Usage (from `engine/`):
//!   cargo run --release --bin sim_bench -- [--games N] [--deck NAME]
//!       [--seed S] [--policy random|v1|...|v7|conductor] [--repeat R]
//!       [--jobs N] [--alloc]
//!
//! - Default: every deck in `../web_ui/decks`, `--games 2000` per deck,
//!   `--policy random`, `--repeat 1`, `--jobs 1` (single-thread baseline).
//! - Execute path is `game_setup::execute_action` + `settle_single_player_state`
//!   (the bot_arena / search / RL path), NOT `profile_target`'s raw loop.
//! - Card DB + deck templates load outside the timed region; per-game deal +
//!   full play loop are inside.
//! - Deterministic: per-game `rng::seed(engine_seed)` + local `rng::Lcg`
//!   (bot_arena `game_seeds` pattern). Engine RNG is thread-local; bot policy
//!   calls restore that thread's engine RNG. Same seeds → same games at any
//!   `--jobs`.
//! - `--jobs N` runs games of one deck on N threads (std::thread::scope).
//!   Keep `--jobs 1` when comparing to recorded baselines.
//!
//! This bin intentionally does NOT init a logger and does NOT enable the
//! `profiling` feature (both change measured behavior).
//!
//! Alloc counts: build with `--features alloc_tracker` and set
//! `RABUKA_ALLOC_TRACK=1`, pass `--alloc`. Report prints after the timed loop.

use rabuka_engine::bin_common::{classify_winner, GameOutcome};
use rabuka_engine::bot::{registry::BotKind, strategy_v2, strategy_v3};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder::Deck;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::replay::{observe, Completion, GameTrace, Header, Step};
use rabuka_engine::rng::Lcg;
use rabuka_engine::turn::TurnEngine;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

const USAGE: &str = "\
sim_bench: canonical full-game throughput bench
Usage: cargo run --release --bin sim_bench -- [options]
  --games N       games per deck per repeat (default 2000)
  --deck NAME     single deck name (file stem under ../web_ui/decks)
  --seed S        base engine seed (default 1)
  --policy P      random (default) | v1..v8 | conductor
  --repeat R      repeat the full deck sweep R times (default 1)
   --jobs N        worker threads per deck sweep (default 1 = baseline)
   --per-game      print per-game outcome, end reason, and action count
   --trace PATH    write replay-compatible JSONL for one deck and one repeat
   --alloc         print allocator delta after the timed loop
                   (requires --features alloc_tracker + RABUKA_ALLOC_TRACK)
  --help          show this help";

#[derive(Clone)]
struct Options {
    games: u32,
    deck: Option<String>,
    seed: u32,
    policy: BotKind,
    policy_raw: String,
    repeat: u32,
    jobs: u32,
    alloc: bool,
    per_game: bool,
    trace: Option<PathBuf>,
}

fn parse_args() -> Result<Options, String> {
    let mut opts = Options {
        games: 2000,
        deck: None,
        seed: 1,
        policy: BotKind::Random,
        policy_raw: "random".into(),
        repeat: 1,
        jobs: 1,
        alloc: false,
        per_game: false,
        trace: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--alloc" => opts.alloc = true,
            "--per-game" => opts.per_game = true,
            "--trace" => {
                let value = args
                    .next()
                    .filter(|v| !v.starts_with("--"))
                    .ok_or_else(|| "missing value for --trace".to_string())?;
                opts.trace = Some(PathBuf::from(value));
            }
            "--games" | "--deck" | "--seed" | "--policy" | "--repeat" | "--jobs" => {
                let value = args
                    .next()
                    .filter(|v| !v.starts_with("--"))
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                match arg.as_str() {
                    "--games" => opts.games = value.parse().map_err(|e| format!("--games: {e}"))?,
                    "--deck" => opts.deck = Some(value),
                    "--seed" => opts.seed = value.parse().map_err(|e| format!("--seed: {e}"))?,
                    "--repeat" => opts.repeat = value.parse().map_err(|e| format!("--repeat: {e}"))?,
                    "--jobs" => opts.jobs = value.parse().map_err(|e| format!("--jobs: {e}"))?,
                    "--policy" => {
                        if !BotKind::ALL.contains(&value.as_str()) {
                            return Err(format!(
                                "unknown policy: {value}; expected {}",
                                BotKind::ALL.join(", ")
                            ));
                        }
                        opts.policy_raw = value.clone();
                        opts.policy = BotKind::parse(&value);
                    }
                    _ => unreachable!(),
                }
            }
            other => return Err(format!("unknown option: {other}\n{USAGE}")),
        }
    }
    if opts.games == 0 || opts.seed == 0 || opts.repeat == 0 || opts.jobs == 0 {
        return Err("--games, --seed, --repeat, and --jobs must be positive".into());
    }
    if opts.trace.is_some() && (opts.deck.is_none() || opts.repeat != 1 || opts.jobs != 1) {
        return Err("--trace requires --deck, --repeat 1, and --jobs 1".into());
    }
    Ok(opts)
}

/// Same seed derivation as bot_arena (cross-check compatibility).
fn game_seeds(base: u32, game: u32) -> (u32, u64) {
    let engine = ((u64::from(base) - 1 + u64::from(game) - 1) % u64::from(u32::MAX) + 1) as u32;
    (engine, 0x5EED_1234_ABCD_0001 ^ u64::from(engine))
}

/// Restore the global engine RNG on drop (including panic paths), so bot
/// policy code cannot pollute deal/shuffle streams.
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

struct GameStats {
    actions: u64,
    end_reason: &'static str,
    outcome: GameOutcome,
    game_result: GameResult,
    trace: Option<GameTrace>,
}


fn run_game(
    db: &Arc<CardDatabase>,
    t1: &Deck,
    t2: &Deck,
    engine_seed: u32,
    arena_seed: u64,
    policy: BotKind,
    v2_policy: &strategy_v2::V2Policy,
    game: usize,
    record: bool,
) -> Result<GameStats, String> {
    rabuka_engine::rng::seed(engine_seed);
    let mut rng = Lcg(arena_seed);
    let mut gs = rabuka_engine::bin_common::deal_game(
        db,
        t1,
        t2,
        "p1",
        "P1",
        "p2",
        "P2",
    );

    // Archetype detection once per game (only read by v3+; cheap for v2).
    let (plan_p1, plan_p2) = match policy {
        BotKind::Random | BotKind::V1 | BotKind::V2 => (
            policy_call(|| strategy_v3::V3Plan {
                rush_archetype: false,
            }),
            policy_call(|| strategy_v3::V3Plan {
                rush_archetype: false,
            }),
        ),
        _ => (
            policy_call(|| strategy_v3::V3Plan::detect(&gs, 0, db)),
            policy_call(|| strategy_v3::V3Plan::detect(&gs, 1, db)),
        ),
    };

    let mut trace = if record {
        Some(GameTrace {
            game,
            initial: observe(&gs).map_err(|error| error.to_string())?,
            steps: Vec::new(),
        })
    } else {
        None
    };
    let mut actions_total = 0u64;
    let mut end_reason = "iteration_cap";
    let mut last_turn = 0u8;
    let mut stuck = 0u32;

    for _ in 0..600 {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            end_reason = "game_result";
            if let Some(trace) = trace.as_mut() {
                trace.steps.push(Step {
                    operation: "finished".into(),
                    actions: Vec::new(),
                    selected: None,
                    result: None,
                    state: observe(&gs).map_err(|error| error.to_string())?,
                });
            }
            break;
        }
        if gs.turn_number == last_turn {
            stuck += 1;
            if stuck > 200 {
                end_reason = "same_turn_iteration_cap";
                break;
            }
        } else {
            stuck = 0;
            last_turn = gs.turn_number;
        }

        if game_setup::auto_advance_one(&mut gs) {
            if let Some(trace) = trace.as_mut() {
                trace.steps.push(Step {
                    operation: "auto".into(),
                    actions: Vec::new(),
                    selected: None,
                    result: None,
                    state: observe(&gs).map_err(|error| error.to_string())?,
                });
            }
            continue;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            if let Some(trace) = trace.as_mut() {
                trace.steps.push(Step {
                    operation: "empty_advance".into(),
                    actions: Vec::new(),
                    selected: None,
                    result: None,
                    state: observe(&gs).map_err(|error| error.to_string())?,
                });
            }
            continue;
        }

        let decision_id = decision_player(&gs).id.clone();
        let me = u8::from(decision_id != gs.player1.id);
        let policy_is_p1 = me == 0;
        let chosen = if policy == BotKind::Random {
            match policy_route(&gs) {
                PolicyRoute::Mulligan => policy_call(|| {
                    BotKind::Random
                        .choose_mulligan(&gs, &actions, &gs.card_database)
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
            let plan = if me == 0 { &plan_p1 } else { &plan_p2 };
            match policy_route(&gs) {
                PolicyRoute::Mulligan => policy_call(|| {
                    policy.choose_mulligan(&gs, &actions, &gs.card_database)
                }),
                PolicyRoute::LiveSet => policy_call(|| {
                    policy.choose_live_set(&gs, &actions, &gs.card_database, v2_policy, &plan)
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
                        policy_call(|| {
                            policy.choose_action(&gs, &actions, me, v2_policy, &plan)
                        })
                    }
                }
            }
        };

        let execution_result = bin_common_execute(&mut gs, &chosen);
        if record {
            if let Some(error) = execution_result.as_ref().err() {
                return Err(format!("game {game} action failed: {error}"));
            }
            if let Some(trace) = trace.as_mut() {
                let action_values = actions
                    .iter()
                    .map(serde_json::to_value)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())?;
                let selected = serde_json::to_value(&chosen).map_err(|error| error.to_string())?;
                trace.steps.push(Step {
                    operation: "action".into(),
                    actions: action_values,
                    selected: Some(selected),
                    result: Some("ok".into()),
                    state: observe(&gs).map_err(|error| error.to_string())?,
                });
            }
        }
        actions_total += 1;
    }

    Ok(GameStats {
        actions: actions_total,
        end_reason,
        outcome: classify_winner(&gs),
        game_result: gs.game_result,
        trace,
    })
}

fn bin_common_execute(gs: &mut GameState, action: &game_setup::Action) -> Result<(), String> {
    rabuka_engine::bin_common::execute_and_settle(gs, action)
}

struct DeckRun {
    deck: String,
    games: u32,
    actions: u64,
    secs: f64,
    outcomes: BTreeMap<&'static str, u32>,
    ends: BTreeMap<&'static str, u32>,
}

fn outcome_label(o: GameOutcome) -> &'static str {
    match o {
        GameOutcome::P1Win => "P1 wins",
        GameOutcome::P2Win => "P2 wins",
        GameOutcome::Draw => "draw",
        GameOutcome::Stuck => "stuck",
    }
}

struct TraceWriter {
    writer: BufWriter<std::fs::File>,
    actions: usize,
}

impl TraceWriter {
    fn new(path: &std::path::Path, header: &Header) -> Result<Self, String> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let mut writer = BufWriter::new(file);
        write_json_line(&mut writer, header)?;
        Ok(Self { writer, actions: 0 })
    }

    fn write_game(&mut self, game: &GameTrace) -> Result<(), String> {
        self.actions += game
            .steps
            .iter()
            .filter(|step| step.operation == "action")
            .count();
        write_json_line(&mut self.writer, game)
    }

    fn finish(mut self, games: usize) -> Result<(), String> {
        let completion = Completion {
            complete: true,
            games,
            actions: self.actions,
        };
        write_json_line(&mut self.writer, &completion)?;
        self.writer
            .flush()
            .map_err(|error| error.to_string())?;
        self.writer
            .get_ref()
            .sync_all()
            .map_err(|error| error.to_string())
    }
}

fn write_json_line<T: Serialize>(writer: &mut impl Write, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    writer.write_all(&bytes).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())
}

fn load_templates(db: &mut Arc<CardDatabase>, deck_name: &str) -> Result<(Deck, Deck), String> {
    let deck_path =
        std::path::Path::new("../web_ui/decks").join(format!("{deck_name}.txt"));
    if !deck_path.exists() {
        return Err(format!("deck file not found: {}", deck_path.display()));
    }
    let deck = deck_parser::DeckParser::parse_deck_file(&deck_path)
        .map_err(|e| format!("parse {deck_name}: {e}"))?;
    let nums = deck_parser::DeckParser::deck_list_to_card_numbers(&deck);
    game_setup::build_two_decks(db, &nums, &nums)
        .map_err(|e| format!("build {deck_name}: {e}"))
}

fn list_decks(filter: &Option<String>) -> Result<Vec<String>, String> {
    let dir = std::path::Path::new("../web_ui/decks");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("txt") {
                p.file_stem().and_then(|s| s.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();
    names.sort();
    if let Some(want) = filter {
        if !names.iter().any(|n| n == want) {
            return Err(format!(
                "deck {want:?} not found; available: {}",
                names.join(", ")
            ));
        }
        names.retain(|n| n == want);
    }
    if names.is_empty() {
        return Err("no decks found".into());
    }
    Ok(names)
}

fn run_sweep(
    opts: &Options,
    db: &mut Arc<CardDatabase>,
    deck_names: &[String],
    games: u32,
    repeat_tag: usize,
    trace_writer: &mut Option<TraceWriter>,
) -> Result<Vec<DeckRun>, String> {
    let v2_policy = strategy_v2::V2Policy::default();
    let mut runs = Vec::with_capacity(deck_names.len());
    for name in deck_names {
        let (t1, t2) = load_templates(db, name)?;
        let mut total_actions = 0u64;
        let mut outcomes: BTreeMap<&'static str, u32> = BTreeMap::new();
        let mut ends: BTreeMap<&'static str, u32> = BTreeMap::new();
        // Game index continues across decks so seeds stay distinct.
        // Repeat tag is intentionally NOT part of the seed: every repeat runs
        // the identical game sequence so cross-repeat spread measures timing
        // noise only (not seed→game-length variance, which can exceed 70%).
        let seed_offset = (runs.len() as u32).wrapping_mul(100_000);
        let base_seed = opts.seed.wrapping_add(seed_offset);
        let jobs = opts.jobs.min(games.max(1));
        let t0 = Instant::now();
        if jobs <= 1 {
            for g in 0..games {
                let (engine_seed, arena_seed) = game_seeds(base_seed, g.wrapping_add(1));
                let stats = run_game(
                    db,
                    &t1,
                    &t2,
                    engine_seed,
                    arena_seed,
                    opts.policy,
                    &v2_policy,
                    g as usize,
                    trace_writer.is_some(),
                )?;
                if let Some(writer) = trace_writer.as_mut() {
                    let trace = stats
                        .trace
                        .as_ref()
                        .ok_or_else(|| "trace was not recorded".to_string())?;
                    writer.write_game(trace)?;
                }
                total_actions += stats.actions;
                if opts.per_game {
                    println!(
                        "GAME deck={name} index={g} outcome={} end={} actions={}",
                        outcome_label(stats.outcome),
                        stats.end_reason,
                        stats.actions
                    );
                }
                *outcomes.entry(outcome_label(stats.outcome)).or_insert(0) += 1;
                *ends.entry(stats.end_reason).or_insert(0) += 1;
                let _ = stats.game_result;
            }
        } else {
            // Parallel: one game index → one seed pair, independent of worker.
            // Engine RNG is thread-local; each run_game seeds that worker.
            let chunks: Vec<(u32, u32)> = {
                let per = games / jobs;
                let rem = games % jobs;
                let mut out = Vec::with_capacity(jobs as usize);
                let mut start = 0u32;
                for j in 0..jobs {
                    let len = per + u32::from(j < rem);
                    out.push((start, start + len));
                    start += len;
                }
                out
            };
            let parts = std::thread::scope(|scope| {
                let handles: Vec<_> = chunks
                    .iter()
                    .map(|&(start, end)| {
                        let db = &*db;
                        let t1 = &t1;
                        let t2 = &t2;
                        let v2_policy = &v2_policy;
                        scope.spawn(move || {
                            let mut actions = 0u64;
                            let mut local_outcomes: BTreeMap<&'static str, u32> =
                                BTreeMap::new();
                            let mut local_ends: BTreeMap<&'static str, u32> = BTreeMap::new();
                            for g in start..end {
                                let (engine_seed, arena_seed) =
                                    game_seeds(base_seed, g.wrapping_add(1));
                                let stats = run_game(
                                    db,
                                    t1,
                                    t2,
                                    engine_seed,
                                    arena_seed,
                                    opts.policy,
                                    v2_policy,
                                    g as usize,
                                    false,
                                )
                                .expect("sim_bench worker failed");
                                 actions += stats.actions;
                                 if opts.per_game {
                                     println!(
                                         "GAME deck={name} index={g} outcome={} end={} actions={}",
                                         outcome_label(stats.outcome),
                                         stats.end_reason,
                                         stats.actions
                                     );
                                 }
                                 *local_outcomes.entry(outcome_label(stats.outcome)).or_insert(0) +=
                                     1;
                                *local_ends.entry(stats.end_reason).or_insert(0) += 1;
                                let _ = stats.game_result;
                            }
                            (actions, local_outcomes, local_ends)
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|h| h.join().expect("sim_bench worker panicked"))
                    .collect::<Vec<_>>()
            });
            for (actions, local_outcomes, local_ends) in parts {
                total_actions += actions;
                for (k, v) in local_outcomes {
                    *outcomes.entry(k).or_insert(0) += v;
                }
                for (k, v) in local_ends {
                    *ends.entry(k).or_insert(0) += v;
                }
            }
        }
        let secs = t0.elapsed().as_secs_f64();
        runs.push(DeckRun {
            deck: name.clone(),
            games,
            actions: total_actions,
            secs,
            outcomes,
            ends,
        });
        let gps = games as f64 / secs.max(f64::EPSILON);
        let aps = total_actions as f64 / secs.max(f64::EPSILON);
        println!(
            "DECK\t{}\trepeat={}\tgames={}\tactions={}\tsecs={:.3}\tgps={:.2}\taps={:.1}",
            name, repeat_tag, games, total_actions, secs, gps, aps
        );
    }
    Ok(runs)
}

fn main() {
    fn assert_send<T: Send>() {}
    assert_send::<GameState>();
    if cfg!(debug_assertions) {
        eprintln!("SIM_BENCH=DEBUG-BUILD: numbers are NOT comparable to --release baselines!");
    }
    if let Err(e) = real_main() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let opts = parse_args()?;
    // Headless training: strip UI-only Action strings and log materialization.
    // Decision fields stay. Default for web/UI is full display + logs.
    // RABUKA_ACTION_DISPLAY=1 / RABUKA_LOGS=1 restore pieces for same-binary A/B.
    let display = std::env::var("RABUKA_ACTION_DISPLAY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let logs = std::env::var("RABUKA_LOGS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    game_setup::set_action_display(display);
    game_setup::set_logging_enabled(logs);

    // ── Setup (outside the timed region) ────────────────────────────────
    let setup_t0 = Instant::now();
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path)
        .map_err(|e| format!("load cards: {e:?}"))?;
    let mut db = Arc::new(CardDatabase::load_or_create(cards));
    let deck_names = list_decks(&opts.deck)?;
    let setup_secs = setup_t0.elapsed().as_secs_f64();
    let mut trace_writer = if let Some(path) = opts.trace.as_ref() {
        let deck_path = std::path::Path::new("../web_ui/decks")
            .join(format!("{}.txt", deck_names[0]));
        let deck_text = std::fs::read_to_string(&deck_path)
            .map_err(|error| format!("{}: {error}", deck_path.display()))?;
        let build_identity = std::env::var("RABUKA_REPLAY_BUILD_IDENTITY")
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_else(|| json!({}));
        let header = Header {
            version: 3,
            projection: "players-queue-snapshots-movements-observations-v2".into(),
            feature_schema: 1,
            execution: "execute-and-settle-v1".into(),
            profiling: cfg!(feature = "profiling"),
            producer: "sim_bench".into(),
            deck_text,
            games: opts.games as usize,
            engine_seed: opts.seed,
            policy_seed: 0x5EED_1234_ABCD_0001,
            policy: opts.policy_raw.clone(),
            build_identity,
        };
        Some(TraceWriter::new(path, &header)?)
    } else {
        None
    };

    println!(
        "SIM_BENCH setup_secs={:.3} decks={} games={} repeat={} jobs={} seed={} policy={} display={} logs={} debug_assertions={}",
        setup_secs,
        deck_names.len(),
        opts.games,
        opts.repeat,
        opts.jobs,
        opts.seed,
        opts.policy_raw,
        display,
        logs,
        cfg!(debug_assertions),
    );
    if cfg!(debug_assertions) {
        println!("SIM_BENCH=DEBUG-BUILD");
    }

    #[cfg(feature = "alloc_tracker")]
    let alloc_guard = if opts.alloc {
        if std::env::var("RABUKA_ALLOC_TRACK").is_err() {
            eprintln!("--alloc: set RABUKA_ALLOC_TRACK=1 so the guard reports");
        }
        rabuka_engine::alloc_counter::start()
    } else {
        None
    };
    #[cfg(not(feature = "alloc_tracker"))]
    if opts.alloc {
        eprintln!("--alloc ignored: rebuild with --features alloc_tracker");
    }

    // ── Warmup (outside the timed region) ───────────────────────────────
    // Two cold-start sources inflated repeat-0 gps (~40–90% low):
    //   1. Lazy OnceLock ability decodes (must touch every deck's cards).
    //   2. CPU frequency / turbo ramp — needs sustained load, not a few games.
    // So: cycle all decks until ~WARMUP_SECS of wall time (default 10s),
    // always completing at least one full pass over deck_names first.
    if opts.trace.is_none() {
        let warmup_secs: f64 = std::env::var("SIM_BENCH_WARMUP_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10.0);
        let v2w = strategy_v2::V2Policy::default();
        let w_t0 = Instant::now();
        let mut warmup_games = 0u32;
        let mut passes = 0u32;
        loop {
            for (di, dn) in deck_names.iter().enumerate() {
                let (t1w, t2w) = load_templates(&mut db, dn)?;
                // A handful of games per deck per pass; passes repeat until time is up.
                for w in 0..4 {
                    let (es, ps) = game_seeds(
                        opts.seed ^ 0xA5A5_A5A5,
                        (passes as u32) * 10_000 + (di as u32) * 10 + w + 1,
                    );
                    let _ = run_game(&db, &t1w, &t2w, es, ps, opts.policy, &v2w, 0, false)
                        .expect("sim_bench warmup failed");
                    warmup_games += 1;
                }
            }
            passes += 1;
            // Always finish pass 1 (ability coverage); then honor wall-time target.
            if passes >= 1 && w_t0.elapsed().as_secs_f64() >= warmup_secs {
                break;
            }
            // Hard cap so a pathological seed can't spin forever.
            if passes >= 64 {
                break;
            }
        }
        println!(
            "SIM_BENCH warmup_games={} passes={} secs={:.2} decks={}",
            warmup_games,
            passes,
            w_t0.elapsed().as_secs_f64(),
            deck_names.len()
        );
    }

    // ── Timed sweeps ────────────────────────────────────────────────────
    let mut all_runs: Vec<DeckRun> = Vec::new();
    for rep in 0..opts.repeat {
        let runs = run_sweep(
            &opts,
            &mut db,
            &deck_names,
            opts.games,
            rep as usize,
            &mut trace_writer,
        )?;
        all_runs.extend(runs);
    }

    if let Some(writer) = trace_writer.take() {
        writer.finish(opts.games as usize)?;
    }

    // Drop the alloc guard BEFORE printing the aggregate so its report
    // (stderr) is not interleaved mid-table on stdout.
    #[cfg(feature = "alloc_tracker")]
    drop(alloc_guard);

    // ── Aggregate ───────────────────────────────────────────────────────
    let total_games: u64 = all_runs.iter().map(|r| u64::from(r.games)).sum();
    let total_actions: u64 = all_runs.iter().map(|r| r.actions).sum();
    let total_secs: f64 = all_runs.iter().map(|r| r.secs).sum();
    let mean_gps = if total_secs > 0.0 {
        total_games as f64 / total_secs
    } else {
        0.0
    };
    let mean_aps = if total_secs > 0.0 {
        total_actions as f64 / total_secs
    } else {
        0.0
    };

    // Per-deck mean gps across repeats (for variance + min/max).
    let mut per_deck: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for r in &all_runs {
        let gps = f64::from(r.games) / r.secs.max(f64::EPSILON);
        per_deck.entry(r.deck.as_str()).or_default().push(gps);
    }
    let mut deck_gps: Vec<(&str, f64, f64, f64)> = Vec::new(); // name, mean, min, max
    for (name, samples) in &per_deck {
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let min = samples.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        deck_gps.push((name, mean, min, max));
    }
    deck_gps.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== per-deck gps (mean over repeats, sorted) ===");
    for (name, mean, min, max) in &deck_gps {
        let spread = if *mean > 0.0 {
            (max - min) / mean * 100.0
        } else {
            0.0
        };
        println!("{name}\tmean={mean:.2}\tmin={min:.2}\tmax={max:.2}\tspread={spread:.1}%");
    }

    // Outcome histogram across all runs.
    let mut outcomes: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut ends: BTreeMap<&'static str, u32> = BTreeMap::new();
    for r in &all_runs {
        for (k, v) in &r.outcomes {
            *outcomes.entry(k).or_insert(0) += v;
        }
        for (k, v) in &r.ends {
            *ends.entry(k).or_insert(0) += v;
        }
    }
    println!("\n=== outcomes ===");
    for (label, count) in &outcomes {
        println!(
            "  {label:20} {count:>8} ({:>5.1}%)",
            *count as f64 / total_games.max(1) as f64 * 100.0
        );
    }
    println!("=== end reasons ===");
    for (label, count) in &ends {
        println!(
            "  {label:24} {count:>8} ({:>5.1}%)",
            *count as f64 / total_games.max(1) as f64 * 100.0
        );
    }

    // --features profiling: the engine's `timer` instrumentation only reports
    // if something asks for it. profile_target does; sim_bench (the canonical
    // production-path bench) did not, so the one harness that runs the real
    // execute_and_settle path could never produce a profile.
    if cfg!(feature = "profiling") {
        if opts.jobs > 1 {
            eprintln!(
                "SIM_BENCH: jobs={} -- timer rows are aggregated across threads and the \
                 global TIMERS mutex is on the hot path, so these timings are NOT \
                 comparable to a jobs=1 run. Use --jobs 1 for profiling.",
                opts.jobs
            );
        }
        rabuka_engine::timer::print_results();
    }

    println!(
        "\nSIM_BENCH_SUMMARY policy={} seed={} games={} decks={} repeat={} jobs={} total_secs={:.3} mean_gps={:.3} mean_aps={:.1} total_actions={}",
        opts.policy_raw,
        opts.seed,
        opts.games,
        deck_names.len(),
        opts.repeat,
        opts.jobs,
        total_secs,
        mean_gps,
        mean_aps,
        total_actions,
    );
    Ok(())
}
