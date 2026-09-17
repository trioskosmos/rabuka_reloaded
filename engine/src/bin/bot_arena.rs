//! Bot arena: run N-second matchups between bot versions and random.
//!
//! Usage: cargo run --release --bin bot_arena -- [p1] [p2] [budget_secs] [deck]
//!   p1/p2: any name in bot::registry::BotKind::ALL (default: v2 random 10)
//!
//! Moved out of tests/test_modules/strategy_bot_test.rs  Ethis is a
//! benchmark/arena, not a unit test. Run it when you want numbers.

use rabuka_engine::bot::{registry::BotKind, strategy_v2, strategy_v3, strategy_v6, strategy_v7};
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
    snapshots: Option<PathBuf>,
    compare: Option<PathBuf>,
    trace: bool,
    logs: bool,
}

impl Options {
    fn parse(args: &[String], env_games: Option<&str>) -> ArenaResult<Self> {
        let mut positional = Vec::new();
        let mut games = None;
        let mut seed = 1u32;
        let mut audit = None;
        let mut snapshots = None;
        let mut compare = None;
        let mut trace = false;
        let mut logs = false;
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--trace" => trace = true,
                "--logs" => logs = true,
                "--games" | "--seed" | "--audit" | "--snapshots" | "--compare" => {
                    let value = args.next().filter(|v| !v.starts_with("--"))
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    match arg.as_str() {
                        "--games" => games = Some(value.parse::<u32>()?),
                        "--seed" => seed = value.parse::<u32>()?,
                        "--snapshots" => snapshots = Some(PathBuf::from(value)),
                        "--compare" => compare = Some(PathBuf::from(value)),
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
        if (audit.is_some() || snapshots.is_some()) && games.is_none() {
            return Err("--audit and --snapshots require --games N or ARENA_GAMES=N".into());
        }
        if compare.is_some() && (audit.is_some() || snapshots.is_some() || !positional.is_empty() || trace || logs) {
            return Err("--compare PATH is a standalone exact-state diagnostic mode".into());
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
            snapshots,
            compare,
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

type ScoreFn = fn(&GameState, &[game_setup::Action], u8) -> Vec<(f64, String)>;

/// Exact-state equivalence oracle: identical legal-action offers and identical
/// v6/v7 numeric scoring behavior. Deliberately NOT a byte comparison —
/// independently rebuilt HashMaps serialize equal content in different
/// iteration orders while remaining logically identical.
fn behaviorally_equal(a: &GameState, b: &GameState) -> ArenaResult<bool> {
    let actions_a = game_setup::generate_possible_actions(a);
    let actions_b = game_setup::generate_possible_actions(b);
    if serde_json::to_value(&actions_a)? != serde_json::to_value(&actions_b)? {
        return Ok(false);
    }
    let me = if a.active_player().id == a.player1.id { 0u8 } else { 1u8 };
    for score in [strategy_v6::score_actions as ScoreFn, strategy_v7::score_actions as ScoreFn] {
        let (x, y) = (score(a, &actions_a, me), score(b, &actions_b, me));
        if x.len() != y.len()
            || x.iter().zip(&y).any(|((s1, c1), (s2, c2))| s1.to_bits() != s2.to_bits() || c1 != c2) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn corpus_fold(seed: u32) -> &'static str {
    if seed % 5 == 0 { "holdout" } else { "train" }
}

fn snapshot_eligible(gs: &GameState) -> bool {
    gs.current_phase == Phase::Main && gs.game_result == GameResult::Ongoing
        && gs.get_pending_choice().is_none()
        && gs.ability_queue.is_idle() && gs.ability_queue.is_empty()
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedCard {
    id: i16,
    card_no: String,
    identity: Value,
}

/// Opt-in full-hidden-state capture for offline exact-state debugging.
/// CONTAINS BOTH PLAYERS' PRIVATE INFORMATION (hands, deck order). Never a
/// fair counterfactual evaluation format; per-action scores are hindsight
/// diagnostics only.
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedPosition {
    format: String,
    schema: u32,
    metadata: Value,
    /// Global engine xorshift32 state captured pre-decision; restored before
    /// each bot's scoring/selection so scoring side effects (sim draws) don't
    /// shift subsequent engine RNG.
    engine_rng: u32,
    /// Arena LCG state (instance-owned, pub field) — resumed after replay.
    arena_rng: u64,
    /// Physical card copies with exact card_no identity; supports duplicate
    /// deck copies (per-copy IDs from create_copy).
    cards: Vec<SavedCard>,
    card_no_to_id: std::collections::BTreeMap<String, i16>,
    normalized_no_to_id: std::collections::BTreeMap<String, i16>,
    next_id: i16,
    /// Full GameState (serde fields; card_database skipped, reattached here).
    state: Vec<u8>,
    /// serde-skipped engine-internal fields (loop history, logs, scratch).
    internal_state: Vec<u8>,
    /// Offered actions at capture time; restore rejects any drift.
    offers: Value,
}

fn card_identity(card: &rabuka_engine::card::Card) -> ArenaResult<Value> {
    Ok(json!({
        "card": serde_json::to_value(card)?,
        "abilities": card.abilities.iter().map(|a| serde_json::to_value(a.resolve())).collect::<Result<Vec<_>, _>>()?,
    }))
}

impl SavedPosition {
    fn capture(gs: &GameState, arena_rng: &Lcg, metadata: Value) -> ArenaResult<Self> {
        if !snapshot_eligible(gs) {
            return Err("snapshot requires idle Main with no pending choices".into());
        }
        let db = &gs.card_database;
        let mut cards = db.cards.iter().map(|(&id, card)| Ok(SavedCard {
            id, card_no: card.card_no.to_string(), identity: card_identity(card)?,
        })).collect::<ArenaResult<Vec<_>>>()?;
        cards.sort_by_key(|c| c.id);
        Ok(Self {
            format: "rabuka-arena-exact-state".into(), schema: 1, metadata,
            engine_rng: rabuka_engine::rng::checkpoint(), arena_rng: arena_rng.0,
            cards,
            card_no_to_id: db.card_no_to_id.iter().map(|(k, &v)| (k.clone(), v)).collect(),
            normalized_no_to_id: db.normalized_no_to_id.iter().map(|(k, &v)| (k.clone(), v)).collect(),
            next_id: db.next_id,
            state: rmp_serde::to_vec_named(gs)?,
            internal_state: rmp_serde::to_vec_named(&(
                &gs.game_state_history, &gs.structured_log, &gs.debug_trace,
                &gs.scratch_exp_blade, &gs.scratch_exp_score, &gs.scratch_exp_heart, &gs.scratch_entry_positions,
            ))?,
            offers: serde_json::to_value(game_setup::generate_possible_actions(gs))?,
        })
    }

    /// Rebuild the physical card database (unique per-copy IDs) and the full
    /// GameState, then verify card identity against the current card database
    /// and offers/v6/v7 scoring behavior against the captured record.
    fn restore(&self, templates: &CardDatabase) -> ArenaResult<GameState> {
        if self.format != "rabuka-arena-exact-state" || self.schema != 1 {
            return Err("unsupported snapshot format/schema".into());
        }
        let mut db = CardDatabase::new();
        for saved in &self.cards {
            let card = templates.card_no_to_id.get(&saved.card_no)
                .and_then(|&id| templates.get_card(id))
                .ok_or_else(|| format!("snapshot card unavailable: {}", saved.card_no))?;
            if card_identity(card)? != saved.identity {
                return Err(format!("snapshot card/ability identity mismatch: {}", saved.card_no).into());
            }
            if saved.id < 0 || saved.id >= self.next_id || db.cards.insert(saved.id, card.clone()).is_some() {
                return Err("invalid or duplicate physical card ID".into());
            }
        }
        for (name, &id) in &self.card_no_to_id {
            if db.get_card(id).is_none_or(|c| c.card_no.as_ref() != name) {
                return Err("invalid exact card lookup in snapshot".into());
            }
        }
        for &id in self.normalized_no_to_id.values() {
            if db.get_card(id).is_none() { return Err("invalid normalized card lookup in snapshot".into()); }
        }
        db.card_no_to_id = self.card_no_to_id.iter().map(|(k, &v)| (k.clone(), v)).collect();
        db.normalized_no_to_id = self.normalized_no_to_id.iter().map(|(k, &v)| (k.clone(), v)).collect();
        db.next_id = self.next_id;
        let mut gs: GameState = rmp_serde::from_slice(&self.state)?;
        gs.card_database = Arc::new(db);
        (gs.game_state_history, gs.structured_log, gs.debug_trace,
            gs.scratch_exp_blade, gs.scratch_exp_score, gs.scratch_exp_heart, gs.scratch_entry_positions)
            = rmp_serde::from_slice(&self.internal_state)?;
        if !snapshot_eligible(&gs) {
            return Err("snapshot state is not an idle Main position".into());
        }
        let offers = serde_json::to_value(game_setup::generate_possible_actions(&gs))?;
        if offers != self.offers {
            return Err("restored offers drift from captured offers".into());
        }
        Ok(gs)
    }

    /// Atomic-ish write: create-new temp file, fsync, rename; never overwrites.
    fn write(&self, path: &std::path::Path) -> ArenaResult<()> {
        if path.exists() { return Err(format!("refusing to overwrite snapshot {}", path.display()).into()); }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let result = (|| -> ArenaResult<()> {
            let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
            file.write_all(&rmp_serde::to_vec_named(self)?)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() { let _ = std::fs::remove_file(&temporary); }
        result
    }
}

/// Restores the global engine RNG state on drop, including panic paths.
struct RngRestore(u32);
impl Drop for RngRestore {
    fn drop(&mut self) { rabuka_engine::rng::restore(self.0); }
}

/// Replay one saved position offline: same RNG state restored before each
/// bot's scoring and selection; emits all available actions with card_no,
/// per-bot numeric scores (null if nonfinite), component breakdowns, chosen
/// indices. EXACT-STATE HINDSIGHT DIAGNOSTIC ONLY — not fair counterfactual
/// outcome evaluation; no win claims.
fn compare_position(saved: &SavedPosition, templates: &CardDatabase) -> ArenaResult<Value> {
    let _restore_rng = RngRestore(rabuka_engine::rng::checkpoint());
    let gs = saved.restore(templates)?;
    let actions = game_setup::generate_possible_actions(&gs);
    if actions.is_empty() { return Err("snapshot has no actions".into()); }
    let me = if gs.active_player().id == gs.player1.id { 0 } else { 1 };
    let mut bots = serde_json::Map::new();
    for (name, score, choose) in [
        ("v6", strategy_v6::score_actions as ScoreFn, strategy_v6::choose_action_v6 as fn(&GameState, &[game_setup::Action], u8) -> game_setup::Action),
        ("v7", strategy_v7::score_actions as ScoreFn, strategy_v7::choose_action_v7 as fn(&GameState, &[game_setup::Action], u8) -> game_setup::Action),
    ] {
        rabuka_engine::rng::restore(saved.engine_rng);
        let scores = score(&gs, &actions, me);
        if scores.len() != actions.len() { return Err("score/action length mismatch".into()); }
        rabuka_engine::rng::restore(saved.engine_rng);
        let chosen = choose(&gs, &actions, me);
        let chosen_value = serde_json::to_value(&chosen)?;
        let chosen_index = actions.iter().position(|a| serde_json::to_value(a).ok().as_ref() == Some(&chosen_value))
            .ok_or("bot chose an action not offered")?;
        bots.insert(name.into(), json!({
            "chosen_index": chosen_index,
            "scores": scores.iter().map(|(score, components)| json!({
                "score": if score.is_finite() { Some(*score) } else { None },
                "components": components,
            })).collect::<Vec<_>>(),
        }));
    }
    let available = actions.iter().enumerate().map(|(index, a)| Ok(json!({
        "index": index,
        "card_no": a.parameters.as_ref().and_then(|p| p.card_id)
            .and_then(|id| gs.card_database.get_card(id)).map(|c| c.card_no.as_ref()),
        "action": audit_action(&gs, a)?,
    }))).collect::<ArenaResult<Vec<_>>>()?;
    Ok(json!({
        "format": "rabuka-arena-comparison", "schema": 1, "metadata": saved.metadata,
        "evaluation": "exact-state debugging only; hidden-state one-ply scores are not fair counterfactual outcome evaluation; no win claims",
        "snapshot_visibility": "full hidden state is stored offline only; view is policy player's hand and public opponent board",
        "score_semantics": "final native policy score after Pass override, not win probability; nonfinite values are null",
        "view": audit_view(&gs), "available_actions": available, "bots": bots,
        "engine_rng": saved.engine_rng, "arena_rng": saved.arena_rng.to_string(),
        "policy_environment": std::env::vars().filter(|(key, _)| key.starts_with("V6_") || key.starts_with("V7_")).collect::<std::collections::BTreeMap<_, _>>(),
    }))
}

fn compare_path(path: &std::path::Path) -> ArenaResult<()> {
    let mut files = if path.is_dir() {
        std::fs::read_dir(path)?.map(|entry| entry.map(|e| e.path())).collect::<Result<Vec<_>, _>>()?
            .into_iter().filter(|p| p.extension().is_some_and(|e| e == "rmp")).collect::<Vec<_>>()
    } else { vec![path.to_path_buf()] };
    files.sort();
    if files.is_empty() { return Err("no .rmp snapshots found".into()); }
    let templates = fresh_database();
    let mut stdout = std::io::stdout().lock();
    for file in files {
        let saved: SavedPosition = rmp_serde::from_slice(&std::fs::read(&file)?)?;
        write_jsonl(&mut stdout, &compare_position(&saved, &templates)?)?;
    }
    Ok(())
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
    if let Some(path) = &options.compare { return compare_path(path); }
    if let Some(path) = &options.snapshots {
        std::fs::create_dir_all(path)?;
        eprintln!("SNAPSHOTS: full hidden state, offline exact-state debugging only; seed % 5 == 0 is held out; cap 20/game");
    }
    let p1_kind = options.p1;
    let p2_kind = options.p2;
    let trace = options.trace;
    let logs = options.logs;
    let deck_name = &options.deck;
    let mut audit = options.audit.as_ref().map(std::fs::File::create).transpose()?;
    if logs {
        std::fs::create_dir_all("../test_output/arena_logs")?;
    }
    if (audit.is_some() || options.snapshots.is_some()) && !std::path::Path::new("../web_ui/decks").join(format!("{deck_name}.txt")).is_file() {
        return Err(format!("audit/snapshots require an existing deck file: {deck_name}.txt").into());
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
        let mut captured_turns = std::collections::HashSet::new();
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
            // Opt-in capture: first eligible idle-Main decision per player
            // turn, hard-capped at 20 per game. RNG states captured as-is;
            // full hidden state is written offline only.
            if let Some(directory) = &options.snapshots {
                if snapshot_eligible(&gs) && captured_turns.len() < 20
                    && captured_turns.insert((gs.turn_number, me)) {
                    let metadata = json!({
                        "game": games, "game_seed": engine_seed, "base_seed": options.seed,
                        "decision": decisions.decision + 1, "turn": gs.turn_number,
                        "phase": gs.current_phase, "policy_player": gs.active_player().id,
                        "fold": corpus_fold(engine_seed),
                        "split_rule": "game_seed modulo 5 == 0: holdout; otherwise train",
                        "sampling": "first eligible Main decision per player turn; at most 20 per game",
                        "deck": deck_name, "bots": [p1_kind.name(), p2_kind.name()],
                        "policy_environment": std::env::vars().filter(|(key, _)| key.starts_with("V6_") || key.starts_with("V7_")).collect::<std::collections::BTreeMap<_, _>>(),
                    });
                    let saved = SavedPosition::capture(&gs, &rng, metadata)?;
                    let restored = saved.restore(&db)?;
                    if !behaviorally_equal(&gs, &restored)? {
                        return Err("snapshot serialization is not lossless for this state".into());
                    }
                    saved.write(&directory.join(format!(
                        "{}-seed-{engine_seed:010}-decision-{:04}.rmp",
                        corpus_fold(engine_seed), decisions.decision + 1,
                    )))?;
                }
            }

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
    fn snapshot_options_are_bounded_and_split_is_grouped_by_seed() {
        assert!(Options::parse(&args(&["--snapshots", "positions"]), None).is_err());
        let options = Options::parse(&args(&["--games", "2", "--snapshots", "positions"]), None).unwrap();
        assert_eq!(options.snapshots, Some(PathBuf::from("positions")));
        assert!(options.audit.is_none());
        let compare = Options::parse(&args(&["--compare", "positions"]), None).unwrap();
        assert_eq!(compare.compare, Some(PathBuf::from("positions")));
        assert!(Options::parse(&args(&["--compare", "p", "--snapshots", "q", "--games", "1"]), None).is_err());
        assert!(Options::parse(&args(&["--compare", "p", "--trace"]), None).is_err());
        assert!(Options::parse(&args(&["--compare", "p", "v6", "v7"]), None).is_err());
        for seed in 1..100 {
            let fold = corpus_fold(seed);
            assert_eq!(fold == "holdout", seed % 5 == 0);
        }
    }

    #[test]
    fn saved_real_deck_roundtrip_preserves_offers_choices_rng_and_original() {
        let _restore_rng = RngRestore(rabuka_engine::rng::checkpoint());
        rabuka_engine::rng::seed(17);
        let mut db = fresh_database();
        let nums = load_test_deck(&db, "5CP3Z idou");
        let (t1, t2) = build_templates(&mut db, &nums, &nums);
        let mut gs = deal_from_templates(&db, &t1, &t2);
        let mut setup_rng = Lcg(17321);
        for _ in 0..100 {
            if snapshot_eligible(&gs) { break; }
            if game_setup::auto_advance_one(&mut gs) { continue; }
            let actions = game_setup::generate_possible_actions(&gs);
            let action = actions.iter().find(|a| a.action_type == game_setup::ActionType::ConfirmMulligan).unwrap_or(&actions[setup_rng.range(actions.len())]);
            game_setup::execute_action(&mut gs, action).unwrap();
            game_setup::settle_single_player_state(&mut gs);
        }
        assert!(snapshot_eligible(&gs));
        // Non-serde engine internals must survive the roundtrip.
        gs.game_state_history.push(1234567);
        gs.debug_trace.push("snapshot test".into());
        let db_before = gs.card_database.cards.len();
        let arena_rng = Lcg(9876);
        let rng_before = rabuka_engine::rng::checkpoint();
        let saved = SavedPosition::capture(&gs, &arena_rng, json!({"game_seed": 17, "fold": corpus_fold(17)})).unwrap();
        // Capture must not disturb either RNG.
        assert_eq!(rng_before, rabuka_engine::rng::checkpoint());
        assert_eq!(arena_rng.0, saved.arena_rng);
        // Restore replays the exact global engine RNG stream.
        let expected_rng: Vec<_> = (0..8).map(|_| rabuka_engine::rng::rand_range(1_000_000)).collect();
        rabuka_engine::rng::restore(saved.engine_rng);
        assert_eq!(expected_rng, (0..8).map(|_| rabuka_engine::rng::rand_range(1_000_000)).collect::<Vec<_>>());
        // Arena LCG state roundtrips.
        assert_eq!(Lcg(saved.arena_rng).next_u64(), Lcg(arena_rng.0).next_u64());
        // File write: atomic create, refuses overwrite, reload identical.
        let directory = std::env::temp_dir().join(format!("rabuka-snapshot-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("position.rmp");
        saved.write(&path).unwrap();
        assert!(saved.write(&path).is_err());
        let mut loaded: SavedPosition = rmp_serde::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        // Physical duplicate-deck IDs: real decks need far more physical IDs
        // than template card_nos.
        let template_count = fresh_database().cards.len();
        assert!(saved.cards.len() > template_count + 100, "real decks require unique physical duplicate-card IDs");
        let restored = loaded.restore(&fresh_database()).unwrap();
        // Offers and v6/v7 numeric behavior identical after restore.
        assert!(behaviorally_equal(&gs, &restored).unwrap());
        assert_eq!(restored.card_database.cards.len(), db_before);
        for (&id, card) in &gs.card_database.cards {
            assert_eq!(restored.card_database.get_card(id).unwrap().card_no, card.card_no);
        }
        assert_eq!(restored.game_state_history, gs.game_state_history);
        assert_eq!(restored.debug_trace, gs.debug_trace);
        // compare is RNG-clean, deterministic across surrounding RNG use,
        // scores every offered action, and chooses an offered index.
        let compare_before = rabuka_engine::rng::checkpoint();
        let first = compare_position(&loaded, &db).unwrap();
        assert_eq!(compare_before, rabuka_engine::rng::checkpoint());
        rabuka_engine::rng::seed(998877);
        let second = compare_position(&loaded, &db).unwrap();
        assert_eq!(first, second);
        assert_eq!(rabuka_engine::rng::checkpoint(), 998877);
        for name in ["v6", "v7"] {
            assert_eq!(first["bots"][name]["scores"].as_array().unwrap().len(), saved.offers.as_array().unwrap().len());
            assert!(first["bots"][name]["chosen_index"].as_u64().is_some());
        }
        // Original card DB unmutated by capture/compare paths; capture-time
        // RNG neutrality was asserted right after SavedPosition::capture, and
        // compare-time neutrality (relative to each call's entry) above — the
        // global state here is deliberately 998877 from the determinism probe.
        assert_eq!(gs.card_database.cards.len(), db_before);
        // Rejection paths: bad schema, unknown card, ineligible phase.
        loaded.schema = 999;
        assert!(loaded.restore(&db).is_err());
        loaded.schema = 1;
        loaded.cards[0].card_no = "unsupported-card".into();
        assert!(loaded.restore(&db).is_err());
        gs.current_phase = Phase::RockPaperScissors;
        assert!(SavedPosition::capture(&gs, &arena_rng, json!({})).is_err());
        std::fs::remove_dir_all(&directory).unwrap();
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

