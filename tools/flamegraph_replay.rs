use rabuka_engine::bin_common;
use rabuka_engine::bot::registry::BotKind;
use rabuka_engine::bot::strategy_v2::V2Policy;
use rabuka_engine::bot::strategy_v3::V3Plan;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup::{self, Action, ActionType};
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::replay::{observe, Completion, GameTrace, Header, Step};
use rabuka_engine::rng::{self, Lcg};
use rabuka_engine::turn::TurnEngine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const USAGE: &str = "flamegraph_replay record TRACE DECK GAMES ENGINE_SEED POLICY_SEED [POLICY]\nflamegraph_replay replay TRACE DECK\nRun from the engine directory. TRACE must not exist for recording.\nRecord/replay uses execute_action plus settle_single_player_state.\nChecks a documented state projection, not complete engine equivalence.\nProfiling builds emit folded stacks to stdout; diagnostics go to stderr.";

#[derive(Debug)]
struct Config {
    record: bool,
    trace: PathBuf,
    deck: PathBuf,
    games: usize,
    engine_seed: u32,
    policy_seed: u64,
    policy: String,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn parse_args(args: &[String]) -> io::Result<Option<Config>> {
    if args == ["--help"] || args == ["-h"] {
        return Ok(None);
    }
    let record = args.first().map(String::as_str) == Some("record");
    if !(record && (args.len() == 6 || args.len() == 7)
        || args.first().map(String::as_str) == Some("replay") && args.len() == 3)
    {
        return Err(invalid(USAGE));
    }
    let config = Config {
        record,
        trace: PathBuf::from(&args[1]),
        deck: PathBuf::from(&args[2]),
        games: if record {
            args[3].parse().map_err(|_| invalid("Invalid games"))?
        } else {
            0
        },
        engine_seed: if record {
            args[4]
                .parse()
                .map_err(|_| invalid("Invalid engine seed"))?
        } else {
            0
        },
        policy_seed: if record {
            args[5]
                .parse()
                .map_err(|_| invalid("Invalid policy seed"))?
        } else {
            0
        },
        policy: if record {
            args.get(6).cloned().unwrap_or_else(|| "random".into())
        } else {
            "random".into()
        },
    };
    if record && !BotKind::ALL.contains(&config.policy.as_str()) {
        return Err(invalid(format!("Unknown policy: {}", config.policy)));
    }
    if record && (config.games == 0 || config.games > 10_000 || config.engine_seed == 0) {
        return Err(invalid(
            "Games must be 1..=10000 and engine seed must be nonzero",
        ));
    }
    Ok(Some(config))
}


fn execute(gs: &mut GameState, action: &Action) -> Result<(), String> {
    let p = action.parameters.as_ref();
    bin_common::execute_and_settle(gs, action)
}

fn mismatch(expected: &Value, actual: &Value, path: &str) -> Option<String> {
    match (expected, actual) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                let next = format!("{path}/{key}");
                let Some(other) = b.get(key) else {
                    return Some(format!("{next}: missing field"));
                };
                if let Some(diff) = mismatch(value, other, &next) {
                    return Some(diff);
                }
            }
            b.keys()
                .find(|key| !a.contains_key(*key))
                .map(|key| format!("{path}/{key}: unexpected field"))
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: length {} != {}", a.len(), b.len()));
            }
            a.iter()
                .zip(b)
                .enumerate()
                .find_map(|(i, (a, b))| mismatch(a, b, &format!("{path}/{i}")))
        }
        _ if expected != actual => Some(format!("{path}: expected {expected}, actual {actual}")),
        _ => None,
    }
}

fn verify<T: Serialize>(
    expected: &T,
    actual: &T,
    location: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(diff) = mismatch(
        &serde_json::to_value(expected)?,
        &serde_json::to_value(actual)?,
        location,
    ) {
        return Err(invalid(diff).into());
    }
    Ok(())
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
        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => PolicyRoute::LiveSet,
        _ => PolicyRoute::Action,
    }
}

fn run_game(
    gs: &mut GameState,
    db: &CardDatabase,
    game: usize,
    policy: BotKind,
    policy_seed: &mut Lcg,
    v2_policy: &V2Policy,
    plan_p1: &V3Plan,
    plan_p2: &V3Plan,
    expected: Option<&GameTrace>,
    measured: &mut Duration,
) -> Result<GameTrace, Box<dyn std::error::Error>> {
    let initial = observe(gs)?;
    if let Some(expected) = expected {
        if expected.game != game {
            return Err(invalid("Trace game order mismatch").into());
        }
        if expected.steps.is_empty() || expected.steps.len() > 2000 {
            return Err(invalid("Trace game must contain 1..=2000 steps").into());
        }
        verify(&expected.initial, &initial, &format!("game/{game}/initial"))?;
    }
    let mut trace = GameTrace {
        game,
        initial,
        steps: Vec::new(),
    };
    let mut last_turn = 0;
    let mut stuck = 0;
    for index in 0..2000 {
        let start = Instant::now();
        TurnEngine::check_victory_condition(gs);
        *measured += start.elapsed();
        if gs.game_result != GameResult::Ongoing {
            let terminal = Step {
                operation: "finished".into(),
                actions: Vec::new(),
                selected: None,
                result: None,
                state: observe(gs)?,
            };
            if let Some(expected) = expected {
                let step = expected
                    .steps
                    .get(index)
                    .ok_or_else(|| invalid(format!("game/{game}/{index}: unexpected terminal")))?;
                verify(step, &terminal, &format!("game/{game}/{index}"))?;
                if expected.steps.len() != index + 1 {
                    return Err(invalid("Trailing steps after terminal").into());
                }
            }
            trace.steps.push(terminal);
            return Ok(trace);
        }
        if gs.turn_number == last_turn {
            stuck += 1;
        } else {
            stuck = 0;
            last_turn = gs.turn_number;
        }
        if stuck > 300 {
            return Err(invalid(format!(
                "Game {game}: stuck cutoff; not a valid benchmark completion"
            ))
            .into());
        }
        let start = Instant::now();
        let advanced = game_setup::auto_advance_one(gs);
        *measured += start.elapsed();
        let mut step = Step {
            operation: "auto".into(),
            actions: Vec::new(),
            selected: None,
            result: None,
            state: Value::Null,
        };
        if !advanced {
            let start = Instant::now();
            let actions = game_setup::generate_possible_actions(gs);
            *measured += start.elapsed();
            step.actions = actions
                .iter()
                .map(serde_json::to_value)
                .collect::<Result<_, _>>()?;
            if actions.is_empty() {
                step.operation = "empty_advance".into();
                let start = Instant::now();
                TurnEngine::advance_phase(gs);
                *measured += start.elapsed();
            } else {
                step.operation = "action".into();
                let selected = if let Some(expected) = expected {
                    let expected = expected
                        .steps
                        .get(index)
                        .ok_or_else(|| invalid("Replay has more steps than trace"))?;
                    verify(
                        &expected.actions,
                        &step.actions,
                        &format!("game/{game}/{index}/actions"),
                    )?;
                    step.actions
                        .iter()
                        .position(|a| Some(a) == expected.selected.as_ref())
                        .ok_or_else(|| invalid("Recorded semantic action is not available"))?
                } else {
                    let selected_action = if policy == BotKind::Random {
                        let decision_id = decision_player(gs).id.clone();
                        let me = u8::from(decision_id != gs.player1.id);
                        let policy_is_p1 = me == 0;
                        match policy_route(gs) {
                            PolicyRoute::Mulligan | PolicyRoute::LiveSet => {
                                actions[policy_seed.range(actions.len())].clone()
                            }
                            PolicyRoute::Action => {
                                if gs.current_phase == Phase::RockPaperScissors {
                                    actions[policy_seed.range(actions.len())].clone()
                                } else if gs.current_phase == Phase::ChooseFirstAttacker {
                                    let won_rps =
                                        gs.rps_winner == Some(if policy_is_p1 { 1 } else { 2 });
                                    if won_rps {
                                        actions
                                            .iter()
                                            .find(|a| {
                                                a.action_type == ActionType::ChooseSecondAttacker
                                            })
                                            .cloned()
                                            .unwrap_or_else(|| {
                                                actions[policy_seed.range(actions.len())].clone()
                                            })
                                    } else {
                                        actions[policy_seed.range(actions.len())].clone()
                                    }
                                } else {
                                    actions[policy_seed.range(actions.len())].clone()
                                }
                            }
                        }
                    } else if gs.current_phase == Phase::RockPaperScissors {
                        actions[policy_seed.range(actions.len())].clone()
                    } else if gs.current_phase == Phase::ChooseFirstAttacker {
                        let decision_id = decision_player(gs).id.clone();
                        let policy_is_p1 = decision_id == gs.player1.id;
                        let won_rps = gs.rps_winner == Some(if policy_is_p1 { 1 } else { 2 });
                        if won_rps {
                            actions
                                .iter()
                                .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                                .cloned()
                                .unwrap_or_else(|| actions[policy_seed.range(actions.len())].clone())
                        } else {
                            actions[policy_seed.range(actions.len())].clone()
                        }
                    } else {
                        let decision_id = decision_player(gs).id.clone();
                        let me = u8::from(decision_id != gs.player1.id);
                        let plan = if me == 0 { plan_p1 } else { plan_p2 };
                        let checkpoint = rng::checkpoint();
                        let selected = match policy_route(gs) {
                            PolicyRoute::Mulligan => {
                                policy.choose_mulligan(gs, &actions, db)
                            }
                            PolicyRoute::LiveSet => {
                                policy.choose_live_set(gs, &actions, db, v2_policy, plan)
                            }
                            PolicyRoute::Action => {
                                policy.choose_action(gs, &actions, me, v2_policy, plan)
                            }
                        };
                        rng::restore(checkpoint);
                        selected
                    };
                    let selected_value = serde_json::to_value(&selected_action)?;
                    step.actions
                        .iter()
                        .position(|a| a == &selected_value)
                        .ok_or_else(|| invalid("Policy selected an unavailable action"))?
                };
                step.selected = Some(step.actions[selected].clone());
                let start = Instant::now();
                let result = execute(gs, &actions[selected]);
                *measured += start.elapsed();
                result.map_err(|error| {
                    invalid(format!("game/{game}/{index}: action failed: {error}"))
                })?;
                step.result = Some("ok".into());
            }
        }
        step.state = observe(gs)?;
        if let Some(expected) = expected {
            let previous = expected
                .steps
                .get(index)
                .ok_or_else(|| invalid("Replay has more steps than trace"))?;
            verify(previous, &step, &format!("game/{game}/{index}"))?;
        }
        trace.steps.push(step);
    }
    Err(invalid(format!(
        "Game {game}: iteration cutoff; not a valid benchmark completion"
    ))
    .into())
}

struct TraceOutput {
    writer: Option<BufWriter<File>>,
    staging: PathBuf,
    destination: PathBuf,
}

impl TraceOutput {
    fn new(destination: &std::path::Path) -> io::Result<Self> {
        if destination.exists() {
            return Err(invalid("Trace destination already exists"));
        }
        let mut name = destination.as_os_str().to_os_string();
        name.push(".partial");
        let staging = PathBuf::from(name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)?;
        Ok(Self {
            writer: Some(BufWriter::new(file)),
            staging,
            destination: destination.to_path_buf(),
        })
    }

    fn publish(mut self) -> io::Result<()> {
        let mut writer = self
            .writer
            .take()
            .ok_or_else(|| invalid("Missing trace writer"))?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);
        fs::hard_link(&self.staging, &self.destination)?;
        fs::remove_file(&self.staging)?;
        Ok(())
    }
}

impl Drop for TraceOutput {
    fn drop(&mut self) {
        drop(self.writer.take());
        let _ = fs::remove_file(&self.staging);
    }
}

const MAX_RECORD_BYTES: u64 = 64 * 1024 * 1024;

fn read_record<T: serde::de::DeserializeOwned>(
    reader: &mut impl BufRead,
    limit: u64,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut line = String::new();
    let size = reader.take(limit + 1).read_line(&mut line)? as u64;
    if size == 0 {
        return Err(invalid("Truncated trace").into());
    }
    if size > limit {
        return Err(invalid("Trace record exceeds byte limit").into());
    }
    if !line.ends_with('\n') {
        return Err(invalid("Truncated trace record: missing newline").into());
    }
    Ok(serde_json::from_str(&line)?)
}

fn read_line<T: serde::de::DeserializeOwned>(
    reader: &mut impl BufRead,
) -> Result<T, Box<dyn std::error::Error>> {
    read_record(reader, MAX_RECORD_BYTES)
}

fn write_record<T: Serialize>(
    writer: &mut impl Write,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() as u64 >= MAX_RECORD_BYTES {
        return Err(invalid("Trace record exceeds byte limit").into());
    }
    writer.write_all(&bytes)?;
    writeln!(writer)?;
    Ok(())
}

fn validate_identity(expected: &Value, actual: &Value) -> io::Result<()> {
    for key in [
        "format",
        "tool",
        "assets_sha256",
        "cargo_lock_sha256",
        "features",
    ] {
        let left = expected
            .get(key)
            .ok_or_else(|| invalid(format!("header/build_identity/{key}: missing field")))?;
        let right = actual
            .get(key)
            .ok_or_else(|| invalid(format!("compiled build_identity/{key}: missing field")))?;
        if left != right {
            return Err(invalid(format!(
                "header/build_identity/{key}: incompatible build"
            )));
        }
    }
    Ok(())
}

fn validate_header(header: &Header, deck: &str) -> io::Result<()> {
    if header.version != 3 {
        return Err(invalid(
            "header/version: expected 3; re-record legacy traces",
        ));
    }
    if header.projection != "players-queue-snapshots-movements-observations-v2" {
        return Err(invalid("header/projection: unsupported projection"));
    }
    if header.feature_schema != 1 {
        return Err(invalid("header/feature_schema: unsupported feature schema"));
    }
    if header.execution != "execute-and-settle-v1" {
        return Err(invalid("header/execution: unsupported execution profile"));
    }
    if !["flamegraph_replay", "sim_bench"].contains(&header.producer.as_str()) {
        return Err(invalid("header/producer: unsupported producer"));
    }
    if !BotKind::ALL.contains(&header.policy.as_str()) {
        return Err(invalid("header/policy: unsupported policy"));
    }
    if header.profiling != cfg!(feature = "profiling") {
        return Err(invalid("header/profiling: build mode differs"));
    }
    if header.deck_text != deck {
        return Err(invalid("header/deck_text: deck contents differ"));
    }
    if header.games == 0 || header.games > 10_000 {
        return Err(invalid("header/games: expected 1..=10000"));
    }
    if header.engine_seed == 0
        || header
            .engine_seed
            .checked_add((header.games - 1) as u32)
            .is_none()
    {
        return Err(invalid("header/engine_seed: zero or per-game overflow"));
    }
    if header
        .policy_seed
        .checked_add((header.games - 1) as u64)
        .is_none()
    {
        return Err(invalid("header/policy_seed: per-game overflow"));
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) = parse_args(&std::env::args().skip(1).collect::<Vec<_>>())? else {
        println!("{USAGE}");
        return Ok(());
    };
    let mut reader = if config.record {
        None
    } else {
        Some(BufReader::new(File::open(&config.trace)?))
    };
    let mut deck_text = String::new();
    File::open(&config.deck)?
        .take(1024 * 1024 + 1)
        .read_to_string(&mut deck_text)?;
    if deck_text.len() > 1024 * 1024 {
        return Err(invalid("Deck exceeds 1 MiB limit").into());
    }
    let build_identity: Value =
        serde_json::from_str(option_env!("RABUKA_REPLAY_BUILD_IDENTITY").ok_or_else(|| {
            invalid("Missing build identity; rebuild with tools/flamegraph_replay.py")
        })?)?;
    let header = if let Some(reader) = reader.as_mut() {
        read_line::<Header>(reader)?
    } else {
        Header {
            version: 3,
            projection: "players-queue-snapshots-movements-observations-v2".into(),
            feature_schema: 1,
            execution: "execute-and-settle-v1".into(),
            profiling: cfg!(feature = "profiling"),
            producer: "flamegraph_replay".into(),
            deck_text: deck_text.clone(),
            games: config.games,
            engine_seed: config.engine_seed,
            policy_seed: config.policy_seed,
            policy: config.policy.clone(),
            build_identity: build_identity.clone(),
        }
    };
    validate_header(&header, &deck_text)?;
    if header.producer == "flamegraph_replay" {
        validate_identity(&header.build_identity, &build_identity)?;
    }
    let cards = CardLoader::load_cards_from_file(std::path::Path::new("../cards/cards.json"))?;
    let mut db = Arc::new(CardDatabase::load_or_create(cards));
    let deck = DeckParser::parse_deck_file(&config.deck)?;
    if fs::read_to_string(&config.deck)? != deck_text {
        return Err(invalid("Deck changed during loading").into());
    }
    let numbers = DeckParser::deck_list_to_card_numbers(&deck);
    let (p1, p2) = game_setup::build_two_decks(&mut db, &numbers, &numbers)?;
    let mut output = if config.record {
        Some(TraceOutput::new(&config.trace)?)
    } else {
        None
    };
    if let Some(writer) = output.as_mut().and_then(|o| o.writer.as_mut()) {
        write_record(writer, &header)?;
    }
    rabuka_engine::timer::reset();
    let mut measured = Duration::ZERO;
    let mut actions = 0;
    for game in 0..header.games {
        let expected = if let Some(reader) = reader.as_mut() {
            Some(read_line::<GameTrace>(reader)?)
        } else {
            None
        };
        let seed = header.engine_seed + game as u32;
        rng::seed(seed);
        let mut policy = Lcg::new(0x5EED_1234_ABCD_0001 ^ u64::from(seed));
        let mut gs =
            bin_common::deal_game(&db, &p1, &p2, "player1", "Player 1", "player2", "Player 2");
        let v2_policy = V2Policy::default();
        let plan_p1 = V3Plan::detect(&gs, 0, &db);
        let plan_p2 = V3Plan::detect(&gs, 1, &db);
        let policy_kind = BotKind::parse(&header.policy);
        let trace = run_game(
            &mut gs,
            &db,
            game,
            policy_kind,
            &mut policy,
            &v2_policy,
            &plan_p1,
            &plan_p2,
            expected.as_ref(),
            &mut measured,
        )?;
        actions += trace
            .steps
            .iter()
            .filter(|s| s.operation == "action")
            .count();
        if let Some(writer) = output.as_mut().and_then(|o| o.writer.as_mut()) {
            write_record(writer, &trace)?;
            writer.flush()?;
        }
        eprintln!(
            "game={} result={:?} steps={}",
            game,
            gs.game_result,
            trace.steps.len()
        );
    }
    let completion = Completion {
        complete: true,
        games: header.games,
        actions,
    };
    if let Some(writer) = output.as_mut().and_then(|o| o.writer.as_mut()) {
        write_record(writer, &completion)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    if let Some(reader) = reader.as_mut() {
        verify(&read_line::<Completion>(reader)?, &completion, "completion")?;
        if !reader.fill_buf()?.is_empty() {
            return Err(invalid("Trailing trace records").into());
        }
    }
    if let Some(output) = output {
        output.publish()?;
    }
    eprintln!("Verified {} games, {} actions; engine call time {:.3} ms (excludes setup, trace I/O and comparison; includes per-call clock overhead; not clean throughput)", header.games, actions, measured.as_secs_f64() * 1000.0);
    eprintln!("Projection excludes modifiers, internal resolver state, skipped serde fields and logs; this is not complete engine equivalence.");
    if cfg!(feature = "profiling") {
        rabuka_engine::timer::print_folded();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Header {
        Header {
            version: 3,
            projection: "players-queue-snapshots-movements-observations-v2".into(),
            feature_schema: 1,
            execution: "execute-and-settle-v1".into(),
            profiling: cfg!(feature = "profiling"),
            producer: "flamegraph_replay".into(),
            deck_text: "deck".into(),
            games: 2,
            engine_seed: 42,
            policy_seed: 7,
            policy: "random".into(),
            build_identity: json!({}),
        }
    }

    #[test]
    fn headers_report_precise_mismatch_and_seed_overflow() {
        let base = header();
        assert!(validate_header(&base, "deck").is_ok());
        assert!(validate_header(&base, "changed")
            .unwrap_err()
            .to_string()
            .contains("deck_text"));
        let mut h = base.clone();
        h.version = 1;
        assert!(validate_header(&h, "deck")
            .unwrap_err()
            .to_string()
            .contains("version"));
        h = base.clone();
        h.projection = "wrong".into();
        assert!(validate_header(&h, "deck")
            .unwrap_err()
            .to_string()
            .contains("projection"));
        h = base.clone();
        h.profiling = !h.profiling;
        assert!(validate_header(&h, "deck")
            .unwrap_err()
            .to_string()
            .contains("profiling"));
        h = base.clone();
        h.games = 0;
        assert!(validate_header(&h, "deck")
            .unwrap_err()
            .to_string()
            .contains("games"));
        h = base.clone();
        h.engine_seed = u32::MAX;
        assert!(validate_header(&h, "deck")
            .unwrap_err()
            .to_string()
            .contains("engine_seed"));
        h = base;
        h.policy_seed = u64::MAX;
        assert!(validate_header(&h, "deck")
            .unwrap_err()
            .to_string()
            .contains("policy_seed"));
    }

    #[test]
    fn bounded_records_preserve_utf8_and_reject_partial_lines() {
        let text = "\"日本語\"\n";
        assert_eq!(
            read_record::<String>(&mut io::Cursor::new(text), text.len() as u64).unwrap(),
            "日本語"
        );
        assert!(read_record::<String>(&mut io::Cursor::new(text), text.len() as u64 - 1).is_err());
        assert!(read_record::<String>(&mut io::Cursor::new("\"abc\""), 100).is_err());
    }

    #[test]
    fn trace_publication_never_replaces_and_cleans_failed_recordings() {
        let dir = std::env::temp_dir().join(format!(
            "replay-output-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("trace.jsonl");
        let mut failed = TraceOutput::new(&path).unwrap();
        failed
            .writer
            .as_mut()
            .unwrap()
            .write_all(b"partial")
            .unwrap();
        drop(failed);
        assert!(!path.exists());
        let mut successful = TraceOutput::new(&path).unwrap();
        successful
            .writer
            .as_mut()
            .unwrap()
            .write_all(b"complete\n")
            .unwrap();
        successful.publish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"complete\n");
        assert!(TraceOutput::new(&path).is_err());
        let racing = dir.join("race.jsonl");
        let output = TraceOutput::new(&racing).unwrap();
        fs::write(&racing, b"other writer").unwrap();
        assert!(output.publish().is_err());
        assert_eq!(fs::read(&racing).unwrap(), b"other writer");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn build_identity_checks_compatibility_but_allows_source_comparison() {
        let base = json!({"format": 1, "tool": "flamegraph_replay", "assets_sha256": "assets", "cargo_lock_sha256": "lock", "features": {"default": [], "requested": []}, "engine_src_sha256": "before"});
        let mut candidate = base.clone();
        candidate["engine_src_sha256"] = json!("after");
        assert!(validate_identity(&base, &candidate).is_ok());
        candidate["assets_sha256"] = json!("different");
        assert!(validate_identity(&base, &candidate)
            .unwrap_err()
            .to_string()
            .contains("assets_sha256"));
        assert!(validate_identity(&json!({}), &base).is_err());
    }

    #[test]
    fn rejects_invalid_cli() {
        for args in [
            vec![],
            vec!["record", "a", "b", "0", "1", "2"],
            vec!["record", "a", "b", "1", "0", "2"],
            vec!["replay", "a"],
        ] {
            assert!(parse_args(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
        assert!(parse_args(&["--help".into()]).unwrap().is_none());
    }

    #[test]
    fn preserves_arrays_but_ignores_object_order() {
        assert!(mismatch(
            &json!({"a": 1, "b": [2, 3]}),
            &json!({"b": [2, 3], "a": 1}),
            "state"
        )
        .is_none());
        assert_eq!(
            mismatch(&json!([1, 2]), &json!([2, 1]), "state"),
            Some("state/0: expected 1, actual 2".into())
        );
        assert!(mismatch(&json!({"a": 1}), &json!({}), "state")
            .unwrap()
            .contains("missing field"));
    }

    #[test]
    fn rejects_truncated_and_malformed_records() {
        assert!(read_line::<Header>(&mut io::Cursor::new("")).is_err());
        assert!(read_line::<Header>(&mut io::Cursor::new("not json\n")).is_err());
    }
}
