//! Bot arena: run N-second matchups between bot versions and random.
//!
//! Usage: cargo run --release --bin bot_arena -- [p1] [p2] [budget_secs] [deck]
//!   p1/p2: any name in bot::registry::BotKind::ALL (default: v2 random 10)
//!
//! Moved out of tests/test_modules/strategy_bot_test.rs  Ethis is a
//! benchmark/arena, not a unit test. Run it when you want numbers.

use rabuka_engine::bot::{registry::BotKind, strategy_v2, strategy_v3};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

type ArenaResult<T> = Result<T, Box<dyn std::error::Error>>;

struct Options {
    p1: BotKind,
    p2: BotKind,
    budget: u64,
    games: Option<u32>,
    seed: u32,
    deck: String,
    audit: Option<PathBuf>,
    trace: bool,
    logs: bool,
}

impl Options {
    fn parse(args: &[String], env_games: Option<&str>) -> ArenaResult<Self> {
        let mut positional = Vec::new();
        let mut games = None;
        let mut seed = 1u32;
        let mut audit = None;
        let mut trace = false;
        let mut logs = false;
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--trace" => trace = true,
                "--logs" => logs = true,
                "--games" | "--seed" | "--audit" => {
                    let value = args.next().filter(|v| !v.starts_with("--"))
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    match arg.as_str() {
                        "--games" => games = Some(value.parse::<u32>()?),
                        "--seed" => seed = value.parse::<u32>()?,
                        _ => audit = Some(PathBuf::from(value)),
                    }
                }
                _ if arg.starts_with("--") => return Err(format!("unknown option: {arg}").into()),
                _ => positional.push(arg.as_str()),
            }
        }
        if positional.len() > 4 {
            return Err("expected [p1] [p2] [budget_secs] [deck]".into());
        }
        if games.is_none() {
            games = env_games.map(str::parse::<u32>).transpose()?;
        }
        if games == Some(0) || seed == 0 {
            return Err("game count and seed must be positive".into());
        }
        if audit.is_some() && games.is_none() {
            return Err("--audit requires --games N or ARENA_GAMES=N".into());
        }
        let parse_kind = |name: &str| -> ArenaResult<BotKind> {
            if !BotKind::ALL.contains(&name) {
                return Err(format!("unknown bot: {name}; expected {}", BotKind::ALL.join(", ")).into());
            }
            Ok(BotKind::parse(name))
        };
        Ok(Self {
            p1: parse_kind(positional.first().copied().unwrap_or("v2"))?,
            p2: parse_kind(positional.get(1).copied().unwrap_or("random"))?,
            budget: positional.get(2).map(|s| s.parse()).transpose()?.unwrap_or(10),
            games,
            seed,
            deck: positional.get(3).copied().unwrap_or("5CP3Z idou").to_string(),
            audit,
            trace,
            logs,
        })
    }

    fn should_start_game(&self, completed: u32, elapsed: std::time::Duration) -> bool {
        match self.games {
            Some(count) => completed < count,
            None => elapsed.as_secs() < self.budget,
        }
    }
}

fn game_seeds(base: u32, game: u32) -> (u32, u64) {
    let engine = ((u64::from(base) - 1 + u64::from(game) - 1) % u64::from(u32::MAX) + 1) as u32;
    (engine, 0x5EED_1234_ABCD_0001 ^ u64::from(engine))
}

fn audit_card(db: &CardDatabase, id: i16) -> Value {
    if id < 0 {
        return Value::Null;
    }
    match db.get_card(id) {
        Some(card) => json!({
            "card_id": id,
            "card_no": card.card_no,
            "card_type": card.card_type,
            "base_cost": card.cost,
            "base_hearts": card.base_heart,
            "base_blades": card.blade,
            "blade_hearts": card.blade_heart,
            "required_hearts": card.need_heart,
            "base_score": card.score,
        }),
        None => json!({"card_id": id, "unresolved": true}),
    }
}

fn audit_cards(db: &CardDatabase, ids: &[i16]) -> Vec<Value> {
    ids.iter().map(|&id| audit_card(db, id)).collect()
}

fn audit_view(gs: &GameState) -> Value {
    let own = gs.active_player();
    let opponent = if own.id == gs.player1.id { &gs.player2 } else { &gs.player1 };
        let db = &gs.card_database;
    json!({
        "own": {
            "player": own.id,
            "is_first_attacker": own.is_first_attacker,
            "hand": audit_cards(db, &own.hand.cards),
            "stage_left_center_right": audit_cards(db, &own.stage.stage),
            "energy": {"active": own.energy_zone.active_count(), "total": own.energy_zone.cards.len()},
            "success_count": own.success_live_card_zone.cards.len(),
            "success": audit_cards(db, &own.success_live_card_zone.cards),
        },
        "opponent_public": {
            "stage_left_center_right": audit_cards(db, &opponent.stage.stage),
            "success_count": opponent.success_live_card_zone.cards.len(),
            "success": audit_cards(db, &opponent.success_live_card_zone.cards),
        },
    })
}

fn audit_action(gs: &GameState, action: &game_setup::Action) -> ArenaResult<Value> {
    use game_setup::ActionType;
    let mut value = serde_json::to_value(action)?;
    let references_card = matches!(action.action_type,
        ActionType::SelectMulligan | ActionType::SelectLiveCard | ActionType::PlayMemberToStage
        | ActionType::UseAbility | ActionType::SetLiveCard | ActionType::ChoiceSelect);
    if references_card {
        if let Some(id) = action.parameters.as_ref().and_then(|p| p.card_id) {
            let own = gs.active_player();
            let opponent = if own.id == gs.player1.id { &gs.player2 } else { &gs.player1 };
            let visible = id >= 0 && (own.hand.cards.contains(&id)
                || own.stage.stage.contains(&id)
                || own.success_live_card_zone.cards.contains(&id)
                || own.waitroom.cards.contains(&id)
                || opponent.stage.stage.contains(&id)
                || opponent.success_live_card_zone.cards.contains(&id));
            if visible {
                value["resolved_card"] = audit_card(&gs.card_database, id);
            } else {
                value["identity_redacted"] = json!(true);
                value["description"] = Value::Null;
                value["description_ja"] = Value::Null;
                for field in ["card_id", "card_name", "card_no", "source_ability", "base_cost", "final_cost", "available_areas", "double_baton_pairs"] {
                    value["parameters"][field] = Value::Null;
                }
            }
        }
    }
    Ok(value)
}

#[derive(Default)]
struct DecisionAudit {
    game: u32,
    decision: u32,
    errors: u32,
    boundary: Option<(u8, Phase, String)>,
    boundary_id: u32,
    boundary_step: u32,
    seen: HashMap<String, u32>,
}

fn write_jsonl(writer: &mut impl Write, value: &Value) -> ArenaResult<()> {
    writer.write_all(&serde_json::to_vec(value)?)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn emit(audit: &mut Option<std::fs::File>, value: &Value) -> ArenaResult<()> {
    if let Some(writer) = audit {
        write_jsonl(writer, value)?;
    }
    Ok(())
}

impl DecisionAudit {
    fn record(&mut self, gs: &GameState, actions: &[game_setup::Action], chosen: &game_setup::Action) -> ArenaResult<Value> {
        let boundary = (gs.turn_number, gs.current_phase, gs.active_player().id.clone());
        if self.boundary.as_ref() != Some(&boundary) {
            self.boundary = Some(boundary);
            self.boundary_id += 1;
            self.boundary_step = 0;
        }
        self.boundary_step += 1;
        let available = actions.iter().map(|a| audit_action(gs, a)).collect::<ArenaResult<Vec<_>>>()?;
        let chosen_value = audit_action(gs, chosen)?;
        let snapshot = audit_view(gs);
        let selected = json!(gs.live_card_selected_indices);
        let signature = serde_json::to_string(&json!([
            gs.turn_number, gs.current_phase, gs.active_player().id, snapshot, available, chosen_value, selected, gs.mulligan_selected_indices
        ]))?;
        let repeated_from = self.seen.insert(signature, self.decision);
        let selection_operation = match chosen.action_type {
            game_setup::ActionType::SelectLiveCard | game_setup::ActionType::SelectMulligan => {
                Some(if chosen.selected == Some(true) { "deselect" } else { "select" })
            }
            game_setup::ActionType::ConfirmLiveCardSet | game_setup::ActionType::ConfirmMulligan => Some("confirm"),
            _ => None,
        };
        Ok(json!({
            "event": "decision", "game": self.game, "decision": self.decision,
            "turn": gs.turn_number, "phase": gs.current_phase,
            "policy_player": gs.active_player().id,
            "pending_choice_player": gs.get_pending_choice_player_id(),
            "pending_choice": gs.get_pending_choice().is_some(),
            "boundary_id": self.boundary_id, "boundary_step": self.boundary_step,
            "repeated_visible_decision_from": repeated_from,
            "live_selected_hand_indices_before": selected,
            "mulligan_selected_hand_indices_before": gs.mulligan_selected_indices,
            "selection_operation": selection_operation,
            "view": snapshot, "chosen": chosen_value, "available_actions": available,
        }))
    }

    fn execute(&mut self, audit: &mut Option<std::fs::File>, gs: &mut GameState,
        actions: &[game_setup::Action], chosen: &game_setup::Action) -> ArenaResult<()> {
        self.decision += 1;
        if audit.is_some() {
            emit(audit, &self.record(gs, actions, chosen)?)?;
        }
        let result = game_setup::execute_action(gs, chosen);
        if let Err(error) = &result {
            self.errors += 1;
            eprintln!("ARENA game={} decision={} action={} failed: {}", self.game, self.decision, chosen.action_type, error);
        }
        emit(audit, &json!({
            "event": "action_result", "game": self.game, "decision": self.decision,
            "execution_ok": result.is_ok(),
        }))?;
        game_setup::settle_single_player_state(gs);
        Ok(())
    }
}

/// Bot identity lives in [`BotKind`] (`bot/registry.rs`): adding a version
/// means a new variant + dispatch lines there, never renames here.
use rabuka_engine::rng::Lcg;

fn fresh_database() -> Arc<CardDatabase> {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).expect("load cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

fn load_test_deck(db: &Arc<CardDatabase>, name: &str) -> Vec<String> {
    let deck_path =
        std::path::Path::new("../web_ui/decks").join(format!("{name}.txt"));
    if deck_path.exists() {
        let deck = deck_parser::DeckParser::parse_deck_file(&deck_path).expect("parse deck");
        return deck_parser::DeckParser::deck_list_to_card_numbers(&deck);
    }
    // Fallback: synthesize a legal-ish deck of distinct member/live cards.
    let mut nums: Vec<String> = Vec::new();
    for card in db.cards.values() {
        if !matches!(card.card_type, rabuka_engine::card::CardType::Energy) && nums.len() < 60 {
            nums.push(card.card_no.to_string());
        }
    }
    nums
}

fn build_templates(
    db: &mut Arc<CardDatabase>,
    n1: &[String],
    n2: &[String],
) -> (rabuka_engine::deck_builder::Deck, rabuka_engine::deck_builder::Deck) {
    game_setup::build_two_decks(db, n1, n2).expect("build decks")
}

fn deal_from_templates(
    db: &Arc<CardDatabase>,
    t1: &rabuka_engine::deck_builder::Deck,
    t2: &rabuka_engine::deck_builder::Deck,
) -> GameState {
    let mut d1 = t1.clone();
    let mut d2 = t2.clone();
    d1.shuffle_main_deck();
    d1.shuffle_energy_deck();
    d2.shuffle_main_deck();
    d2.shuffle_energy_deck();
    let mut p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
    let mut p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
    p1.set_main_deck(d1.main_deck);
    p1.set_energy_deck(d1.energy_deck);
    p2.set_main_deck(d2.main_deck);
    p2.set_energy_deck(d2.energy_deck);
    let mut gs = GameState::new(p1, p2, Arc::clone(db));
    game_setup::setup_game(&mut gs);
    gs
}

fn my_hand_lives(gs: &GameState, is_p1: bool, db: &Arc<CardDatabase>) -> usize {
    let p = if is_p1 { &gs.player1 } else { &gs.player2 };
    p.hand
        .cards
        .iter()
        .filter(|&&c| {
            db.get_card(c).is_some_and(|x| {
                x.card_type == rabuka_engine::card::CardType::Live
            })
        })
        .count()
}

fn main() -> ArenaResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env_games = std::env::var("ARENA_GAMES").ok();
    let options = Options::parse(&args, env_games.as_deref())?;
    let p1_kind = options.p1;
    let p2_kind = options.p2;
    let trace = options.trace;
    let logs = options.logs;
    let deck_name = &options.deck;
    let mut audit = options.audit.as_ref().map(std::fs::File::create).transpose()?;
    if logs {
        std::fs::create_dir_all("../test_output/arena_logs")?;
    }
    if audit.is_some() && !std::path::Path::new("../web_ui/decks").join(format!("{deck_name}.txt")).is_file() {
        return Err(format!("audit requires an existing deck file: {deck_name}.txt").into());
    }
    emit(&mut audit, &json!({
        "event": "run_start", "schema_version": 1,
        "bots": [p1_kind.name(), p2_kind.name()], "deck": deck_name,
        "games": options.games, "base_seed": options.seed,
        "iteration_cap_per_game": 600, "same_turn_iteration_cap": 200,
        "card_stats": "printed/base, not effective modifiers",
        "identity_note": "card_no is authoritative; card_id can differ across builds for same-number sibling prints (R+/P/P+/SEC)",
        "visibility": "each row is private to policy_player; opponent snapshot contains stage and success only; non-visible action card identities are redacted",
        "rng_limitations": "engine global RNG and arena LCG reseeded before each deal; simulations may consume global RNG; no checkpoint replay determinism guarantee",
    }))?;

    let kind_name = |k: BotKind| k.name();

    let mut db = fresh_database();
    let nums = load_test_deck(&db, deck_name);
    eprintln!(
        "ARENA deck={} entries={} distinct={}",
        deck_name,
        nums.len(),
        nums.iter().collect::<std::collections::HashSet<_>>().len()
    );
    let (t1, t2) = build_templates(&mut db, &nums, &nums);

    let v2_policy = strategy_v2::V2Policy::default();

    let mut wins = [0u32; 2];
    let mut draws = 0u32;
    // Draw-type breakdown: a "draw" (z1<3 xor z2<3 is false) is either a real
    // mutual 3-3 (both >=3) or a STALL (neither reached 3 — a no-contest
    // artifact from the stuck counter / turn cap). A game that ends while
    // `game_result == Ongoing` was a stuck/timeout break, not a real result.
    let mut real_draws = 0u32;
    let mut stall_draws = 0u32;
    let mut stuck_ends = 0u32;
    let mut final_hist: std::collections::BTreeMap<(u8, u8), u32> = std::collections::BTreeMap::new();
    let mut games = 0u32;
    let mut total_actions = 0u64;
    let mut total_turns = 0u64;
    // Do-nothing telemetry: how often a Main phase ends with no member played
    // (ConfirmMainPhase chosen while a deploy was available) and how often a
    // Live Card Set folds to an empty zone.
    let mut _main_decisions = 0u64;
    let mut _main_confirms = 0u64;
    let mut live_decisions = 0u64;
    let mut live_folds = 0u64;
    let mut main_phase_count = 0u64;
    let mut empty_main_count = 0u64;
    let t0 = std::time::Instant::now();
    let mut trace_rows: Vec<String> = Vec::new();
    let mut game_start_idx = 0usize;
    if trace {
        trace_rows.push(
            "game,turn,phase,player,action_type,card_no,live_p1,live_p2,success_p1,success_p2"
                .into(),
        );
    }

    while options.should_start_game(games, t0.elapsed()) {
        games += 1;
        let (engine_seed, arena_seed) = game_seeds(options.seed, games);
        rabuka_engine::rng::seed(engine_seed);
        let mut rng = Lcg(arena_seed);
        let mut decisions = DecisionAudit { game: games, ..Default::default() };
        emit(&mut audit, &json!({
            "event": "game_start", "game": games,
            "engine_seed": engine_seed, "arena_seed": arena_seed.to_string(),
        }))?;
        let mut gs = deal_from_templates(&db, &t1, &t2);
        let mut end_reason = "iteration_cap";
        // Archetype detection runs once per game over the full own decklist.
        let plan_p1 = strategy_v3::V3Plan::detect(&gs, 0, &db);
        let plan_p2 = strategy_v3::V3Plan::detect(&gs, 1, &db);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        // Live-phase telemetry: snapshot at every phase change so transcripts
        // show WHO set WHAT and whether checks passed.
        let mut prev_phase = gs.current_phase;
        let mut timeline: Vec<String> = Vec::new();
        // Per-main-phase deploy tracking (do-nothing detection); counters
        // themselves live in the outer scope and accumulate across games.
        let mut cur_is_main = false;
        let mut cur_main_plays = 0u64;
        let snap_row = |tag: &str, gs: &GameState| -> String {
            let lives_in_hand = |p: &rabuka_engine::player::Player| {
                p.hand
                    .cards
                    .iter()
                    .filter(|&&c| {
                        db.get_card(c).is_some_and(|x| {
                            x.card_type == rabuka_engine::card::CardType::Live
                        })
                    })
                    .count()
            };
            format!(
                "{},{},{},active={},live_p1={},live_p2={},succ_p1={},succ_p2={},hand_p1={} ({} lives),hand_p2={} ({} lives),en_p1={},en_p2={},cost_p1={},cost_p2={}",
                games,
                gs.turn_number,
                tag,
                if gs.active_player().id == "p1" { "P1" } else { "P2" },
                gs.player1.live_card_zone.cards.len(),
                gs.player2.live_card_zone.cards.len(),
                gs.player1.success_live_card_zone.cards.len(),
                gs.player2.success_live_card_zone.cards.len(),
                gs.player1.hand.cards.len(),
                lives_in_hand(&gs.player1),
                gs.player2.hand.cards.len(),
                lives_in_hand(&gs.player2),
                gs.player1.energy_zone.active_count(),
                gs.player2.energy_zone.active_count(),
                gs.player1
                    .stage
                    .stage
                    .iter()
                    .filter(|&&c| c >= 0)
                    .map(|&c| db.get_card(c).and_then(|x| x.cost).unwrap_or(0) as u32)
                    .sum::<u32>(),
                gs.player2
                    .stage
                    .stage
                    .iter()
                    .filter(|&&c| c >= 0)
                    .map(|&c| db.get_card(c).and_then(|x| x.cost).unwrap_or(0) as u32)
                    .sum::<u32>(),
            )
        };

        for _ in 0..600 {
            if logs && gs.current_phase != prev_phase {
                trace_rows.push(snap_row(&format!("ENTER:{:?}", gs.current_phase), &gs));
                prev_phase = gs.current_phase;
            }
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                end_reason = "game_result";
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
                if logs {
                    timeline.push(format!(
                        "TIMELINE t{} succ {}-{} hand {}({}L)/{}({}L) en {}/{}",
                        gs.turn_number,
                        gs.player1.success_live_card_zone.cards.len(),
                        gs.player2.success_live_card_zone.cards.len(),
                        gs.player1.hand.cards.len(),
                        my_hand_lives(&gs, true, &db),
                        gs.player2.hand.cards.len(),
                        my_hand_lives(&gs, false, &db),
                        gs.player1.energy_zone.active_count(),
                        gs.player2.energy_zone.active_count(),
                    ));
                }
                if games == 1 && std::env::var("DUMP_STATE").is_ok() {
                    for (lbl, p) in [("P1", &gs.player1), ("P2", &gs.player2)] {
                        eprintln!(
                            "T{} {} hand={} deck={} wr={} livezone={} succ={} en={}",
                            gs.turn_number,
                            lbl,
                            p.hand.cards.len(),
                            p.main_deck.cards.len(),
                            p.waitroom.cards.len(),
                            p.live_card_zone.cards.len(),
                            p.success_live_card_zone.cards.len(),
                            p.energy_zone.active_count(),
                        );
                    }
                    eprintln!("phase={:?}", gs.current_phase);
                }
            }

            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            let active_is_p1 = gs.active_player().id == "p1";
            let kind = if active_is_p1 { p1_kind } else { p2_kind };
            let me = if active_is_p1 { 0u8 } else { 1u8 };

            // RPS: random for both (no information to decide with).
            if gs.current_phase == Phase::RockPaperScissors {
                let a = &actions[rng.range(actions.len())];
                decisions.execute(&mut audit, &mut gs, &actions, a)?;
                continue;
            }

            // S6: when this side won RPS, take second attacker.
            if gs.current_phase == Phase::ChooseFirstAttacker {
                let won_rps = gs.rps_winner == Some(if active_is_p1 { 1 } else { 2 });
                let a = if won_rps {
                    actions
                        .iter()
                        .find(|a| {
                            a.action_type
                                == rabuka_engine::game_setup::ActionType::ChooseSecondAttacker
                        })
                        .unwrap_or(&actions[0])
                } else {
                    &actions[rng.range(actions.len())]
                };
                decisions.execute(&mut audit, &mut gs, &actions, a)?;
                continue;
            }

            // Mulligan: dispatched through the registry (v1/random keep the hand).
            if matches!(
                gs.current_phase,
                Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
            ) {
                let a = kind.choose_mulligan(&gs, &actions, &db);
                decisions.execute(&mut audit, &mut gs, &actions, &a)?;
                continue;
            }

            // Live card set: dispatched through the registry (random rolls here
            // because the choice is a raw toggle index, not a policy).
            if matches!(
                gs.current_phase,
                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
            ) {
                let plan = if active_is_p1 { &plan_p1 } else { &plan_p2 };
                let a = if kind == BotKind::Random {
                    actions[rng.range(actions.len())].clone()
                } else {
                    kind.choose_live_set(&gs, &actions, &db, &v2_policy, plan)
                };
                if a.action_type == rabuka_engine::game_setup::ActionType::ConfirmLiveCardSet {
                    live_decisions += 1;
                    if gs.live_card_selected_indices.is_empty() {
                        live_folds += 1;
                    }
                }
                if trace {
                    let card_no = a
                        .parameters
                        .as_ref()
                        .and_then(|p| p.card_id)
                        .and_then(|cid| db.get_card(cid))
                        .map(|c| c.card_no.to_string())
                        .unwrap_or_default();
                    let sel: Vec<String> = gs
                        .live_card_selected_indices
                        .iter()
                        .map(|i| i.to_string())
                        .collect();
                    trace_rows.push(format!(
                        "{},{},{:?},{},CHOICE:{},{},sel=[{}],hand_lives={},live_p1={},live_p2={},succ_p1={},succ_p2={}",
                        games,
                        gs.turn_number,
                        gs.current_phase,
                        if active_is_p1 { "P1" } else { "P2" },
                        a.action_type,
                        card_no,
                        sel.join("+"),
                        my_hand_lives(&gs, active_is_p1, &db),
                        gs.player1.live_card_zone.cards.len(),
                        gs.player2.live_card_zone.cards.len(),
                        gs.player1.success_live_card_zone.cards.len(),
                        gs.player2.success_live_card_zone.cards.len(),
                    ));
                }
                decisions.execute(&mut audit, &mut gs, &actions, &a)?;
                continue;
            }

            // Main phase.
            if trace && active_is_p1 {
                for (ai, aa) in actions.iter().enumerate() {
                    let cn = aa
                        .parameters
                        .as_ref()
                        .and_then(|p| p.card_id)
                        .and_then(|cid| db.get_card(cid))
                        .map(|c| c.card_no.to_string())
                        .unwrap_or_default();
                    trace_rows.push(format!(
                        "{},{},{:?},P1,OPT{},{},{},,,,,",
                        games,
                        gs.turn_number,
                        gs.current_phase,
                        ai,
                        aa.action_type,
                        cn
                    ));
                }
            }
            // Main phase (and everything else policy-driven): registry dispatch.
            let plan = if active_is_p1 { &plan_p1 } else { &plan_p2 };
            let action = if kind == BotKind::Random {
                actions[rng.range(actions.len())].clone()
            } else {
                kind.choose_action(&gs, &actions, me, &v2_policy, plan)
            };
            _main_decisions += 1;
            if gs.current_phase == Phase::Main {
                if !cur_is_main {
                    cur_is_main = true;
                    cur_main_plays = 0;
                }
                let is_pass = action.action_type == rabuka_engine::game_setup::ActionType::Pass;
                let is_deploy = action.action_type
                    == rabuka_engine::game_setup::ActionType::PlayMemberToStage
                    || (action.action_type == rabuka_engine::game_setup::ActionType::UseAbility
                        && action
                            .parameters
                            .as_ref()
                            .and_then(|p| p.use_baton_touch)
                            == Some(true));
                if is_deploy {
                    cur_main_plays += 1;
                }
                if is_pass {
                    main_phase_count += 1;
                    if cur_main_plays == 0 {
                        empty_main_count += 1;
                    }
                    cur_is_main = false;
                }
            } else {
                cur_is_main = false;
            }
            if action.action_type == rabuka_engine::game_setup::ActionType::Pass {
                _main_confirms += 1;
            }
            if trace {
                let card_no = action
                    .parameters
                    .as_ref()
                    .and_then(|p| p.card_id)
                    .and_then(|cid| db.get_card(cid))
                    .map(|c| c.card_no.to_string())
                    .unwrap_or_default();
                trace_rows.push(format!(
                    "{},{},{:?},{},{},{},{},{},{},{}",
                    games,
                    gs.turn_number,
                    gs.current_phase,
                    if active_is_p1 { "P1" } else { "P2" },
                    action.action_type,
                    card_no,
                    gs.player1.live_card_zone.cards.len(),
                    gs.player2.live_card_zone.cards.len(),
                    gs.player1.success_live_card_zone.cards.len(),
                    gs.player2.success_live_card_zone.cards.len(),
                ));
            }
            decisions.execute(&mut audit, &mut gs, &actions, &action)?;
            total_actions += 1;
        }
        total_turns += gs.turn_number as u64;

        let z1 = gs.player1.success_live_card_zone.cards.len();
        let z2 = gs.player2.success_live_card_zone.cards.len();
        if gs.game_result != GameResult::Ongoing {
            end_reason = "game_result";
        }
        emit(&mut audit, &json!({
            "event": "game_end", "game": games, "turn": gs.turn_number,
            "end_reason": end_reason, "game_result": gs.game_result,
            "success_counts": [z1, z2], "decisions": decisions.decision,
            "execution_errors": decisions.errors,
        }))?;
        final_hist
            .entry((z1 as u8, z2 as u8))
            .and_modify(|c| *c += 1)
            .or_insert(1);
        if z1 >= 3 && z2 <= 2 {
            wins[0] += 1;
        } else if z2 >= 3 && z1 <= 2 {
            wins[1] += 1;
        } else {
            draws += 1;
            if z1 >= 3 && z2 >= 3 {
                real_draws += 1;
            } else {
                stall_draws += 1;
            }
        }
        if matches!(gs.game_result, GameResult::Ongoing) {
            stuck_ends += 1;
        }

        if logs {
            let dir = std::path::Path::new("../test_output/arena_logs");
            let result = if z1 >= 3 && z2 <= 2 {
                "P1 WINS"
            } else if z2 >= 3 && z1 <= 2 {
                "P2 WINS"
            } else {
                "DRAW"
            };
            let mut out = format!(
                "game {} | {}({}) vs {}({}) | final success {z1}-{z2} | {}\n=== RULE LOG ===\n",
                games,
                kind_name(p1_kind),
                wins[0],
                kind_name(p2_kind),
                wins[1],
                result
            );
            for line in &gs.rule_log {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("=== STRUCTURED (turn|player|category|text) ===\n");
            for e in &gs.structured_log {
                out.push_str(&format!(
                    "t{}|{}|{}|{}\n",
                    e.turn, e.player_label, e.category, e.text
                ));
            }
            std::fs::write(dir.join(format!("game_{games:03}.txt")), out)?;

            // Self-contained replay: header + turn timeline + decisions.
            if trace {
                let mut replay = format!(
                    "REPLAY game {games} | {result} | final {z1}-{z2}\n\
                     LIVE rows (structured log) = engine's own check verdicts\n\
                     TIMELINE rows = per-turn board summary\n\
                     ENTER rows = board at phase entry\n\
                     other rows = chosen action\n=== TIMELINE ===\n"
                );
                for t in &timeline {
                    replay.push_str(t);
                    replay.push('\n');
                }
                replay.push_str("=== LIVE CHECKS (engine verdicts) ===\n");
                for e in &gs.structured_log {
                    if e.category == "live_result" {
                        replay.push_str(&format!(
                            "t{}|{}\n",
                            e.turn, e.text
                        ));
                    }
                }
                replay.push_str("=== EVENTS ===\n");
                for r in &trace_rows[game_start_idx.min(trace_rows.len())..] {
                    replay.push_str(r);
                    replay.push('\n');
                }
                std::fs::write(
                    dir.join(format!("replay_game_{games:03}.txt")),
                    replay,
                )?;
            }
        }
        game_start_idx = trace_rows.len();
    }

    let secs = t0.elapsed().as_secs_f64();
    println!(
        "{} vs {}  E{} games in {:.1}s ({:.1} gps)\nP1({}) {} - P2({}) {} - draws {}\ntotal actions {} | avg turns/game {:.1}",
        kind_name(p1_kind),
        kind_name(p2_kind),
        games,
        secs,
        games as f64 / secs,
        kind_name(p1_kind),
        wins[0],
        kind_name(p2_kind),
        wins[1],
        draws,
        total_actions,
        total_turns as f64 / games.max(1) as f64,
    );
    let mut hist_lines: Vec<String> = final_hist
        .iter()
        .map(|((a, b), c)| format!("    final success {a}-{b}: {c} game(s)"))
        .collect();
    hist_lines.sort();
    println!(
        "DRAW TYPES: real 3-3 draws={} | stall/no-contest draws={} | stuck/timeout ends={}",
        real_draws, stall_draws, stuck_ends,
    );
    println!("FINAL SCORE HISTOGRAM (success_p1-success_p2 : count):");
    for l in hist_lines {
        println!("{l}");
    }
    let empty_main_rate = if main_phase_count > 0 {
        empty_main_count as f64 / main_phase_count as f64
    } else {
        0.0
    };
    let live_fold_rate = if live_decisions > 0 {
        live_folds as f64 / live_decisions as f64
    } else {
        0.0
    };
    println!(
        "DO-NOTHING telemetry: empty Main phases {}/{} = {:.1}% | live-set fold {}/{} = {:.1}%",
        empty_main_count, main_phase_count, empty_main_rate * 100.0,
        live_folds, live_decisions, live_fold_rate * 100.0,
    );
    if trace {
        let path = std::path::Path::new("../test_output/bot_arena_trace.csv");
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, trace_rows.join("\n") + "\n")?;
        eprintln!("trace written to {}", path.display());
    }
    emit(&mut audit, &json!({"event": "run_end", "games": games}))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn fixed_games_ignore_wall_time_and_parse_flags_without_a_deck() {
        let options = Options::parse(&args(&["v7", "random", "0", "--games", "2", "--audit", "audit.jsonl"]), None).unwrap();
        assert_eq!(options.deck, "5CP3Z idou");
        assert!(options.should_start_game(1, std::time::Duration::from_secs(9999)));
        assert!(!options.should_start_game(2, std::time::Duration::ZERO));
        let env = Options::parse(&[], Some("3")).unwrap();
        assert!(env.should_start_game(2, std::time::Duration::from_secs(9999)));
        let timed = Options::parse(&[], None).unwrap();
        assert!(!timed.should_start_game(0, std::time::Duration::from_secs(10)));
    }

    #[test]
    fn invalid_or_unbounded_audit_options_fail() {
        for values in [vec!["--audit"], vec!["--audit", "out"], vec!["--games", "0"], vec!["--seed", "0"], vec!["--games", "bad"], vec!["unknown"], vec!["--unknown"]] {
            assert!(Options::parse(&args(&values), None).is_err());
        }
        assert!(Options::parse(&[], Some("bad")).is_err());
        assert_eq!(Options::parse(&args(&["--games", "2"]), Some("bad")).unwrap().games, Some(2));
    }

    #[test]
    fn seeds_are_explicit_nonzero_and_wrap() {
        assert_eq!(game_seeds(1, 1).0, 1);
        assert_eq!(game_seeds(1, 2).0, 2);
        assert_eq!(game_seeds(u32::MAX, 2).0, 1);
        assert_ne!(game_seeds(1, 1).1, game_seeds(1, 2).1);
    }

    #[test]
    fn audit_redacts_hidden_action_identity_and_keeps_complete_visible_actions() {
        let db = fresh_database();
        let mut own = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
        let mut opponent = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
        let mut ids: Vec<i16> = db.cards.keys().copied().collect();
        ids.sort();
        own.hand.cards.push(ids[0]);
        opponent.stage.stage[0] = ids[1];
        opponent.hand.cards.push(ids[2]);
        opponent.main_deck.cards.push(ids[3]);
        opponent.live_card_zone.cards.push(ids[4]);
        let mut gs = GameState::new(own, opponent, db);
        gs.current_phase = Phase::LiveCardSetFirstAttacker;
        let actions = game_setup::generate_possible_actions(&gs);
        let chosen = actions.iter().find(|a| a.action_type == game_setup::ActionType::SelectLiveCard).unwrap();
        let mut audit = DecisionAudit { game: 1, decision: 1, ..Default::default() };
        let row = audit.record(&gs, &actions, chosen).unwrap();
        assert_eq!(row["available_actions"].as_array().unwrap().len(), actions.len());
        assert_eq!(row["chosen"]["parameters"], serde_json::to_value(chosen).unwrap()["parameters"]);
        assert_eq!(row["view"]["own"]["hand"][0]["card_no"], gs.card_database.get_card(ids[0]).unwrap().card_no.as_ref());
        assert_eq!(row["view"]["opponent_public"].as_object().unwrap().len(), 3);
        for id in &ids[2..5] {
            assert!(!row.to_string().contains(&format!("\"card_id\":{id},")));
        }
        audit.decision = 2;
        let repeated = audit.record(&gs, &actions, chosen).unwrap();
        assert_eq!(repeated["boundary_step"], 2);
        assert_eq!(repeated["repeated_visible_decision_from"], 1);
        let mut hidden = chosen.clone();
        hidden.action_type = game_setup::ActionType::ChoiceSelect;
        hidden.parameters.as_mut().unwrap().card_id = Some(ids[2]);
        hidden.description = "hidden identity".into();
        let redacted = audit_action(&gs, &hidden).unwrap();
        assert_eq!(redacted["identity_redacted"], true);
        assert!(redacted["parameters"]["card_id"].is_null());
        assert!(redacted["description"].is_null());
    }

    #[test]
    fn confusable_deck_line_resolves_consistently_and_audit_exposes_card_no() {
        let db = fresh_database();
        let deck = load_test_deck(&db, "5CP3Z idou");
        let requested: Vec<&String> = deck
            .iter()
            .filter(|n| n.to_uppercase().contains("BP4-011"))
            .collect();
        assert_eq!(requested.len(), 2, "deck must contain the bp4-011 pair");
        let resolved: Vec<String> = requested
            .iter()
            .map(|n| {
                db.get_card_id(n.as_str())
                    .and_then(|id| db.get_card(id))
                    .map(|c| c.card_no.to_string())
                    .unwrap_or_default()
            })
            .collect();
        assert!(
            resolved.iter().all(|r| r.starts_with("PL!SP-bp4-011-")),
            "both copies must resolve to a bp4-011 print, got {resolved:?}"
        );
        assert_eq!(
            resolved[0], resolved[1],
            "identical deck lines must resolve to the same print within a build"
        );
    }

    #[test]
    fn jsonl_errors_propagate_including_flush() {
        struct Fail(bool);
        impl Write for Fail {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if self.0 { Ok(buf.len()) } else { Err(std::io::Error::other("write failure")) }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("flush failure"))
            }
        }
        assert!(write_jsonl(&mut Fail(false), &json!({})).is_err());
        assert!(write_jsonl(&mut Fail(true), &json!({})).is_err());
        let mut output = Vec::new();
        write_jsonl(&mut output, &json!({"event": "test"})).unwrap();
        assert_eq!(output.last(), Some(&b'\n'));
        assert_eq!(serde_json::from_slice::<Value>(&output).unwrap()["event"], "test");
    }
}

