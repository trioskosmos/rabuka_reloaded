//! Scored game replay: play ONE game and record what every bot saw, what it
//! scored each legal option, and which one it took.
//!
//! Why this exists. Every strength number in this project is a win rate, and a
//! win rate says that a bot is worse, never what it did wrong. `bot_arena
//! --trace` already dumps a per-turn event stream, but it records only *which*
//! action was chosen — the scores that produced the choice are thrown away, and
//! with them the entire reason a decision looked wrong. `bot_arena --compare`
//! scores options, but only for a position you have already snapshotted, not
//! along a trajectory. This tool does both at once, for a whole game.
//!
//! What a decision record contains:
//!   - the board as the acting player sees it (own hand + stage + energy +
//!     success zone; opponent public zones only — hidden information is not
//!     leaked into the artifact),
//!   - every legal action with a human description and its card's printed
//!     stats (cost / blade / hearts / need / score),
//!   - the deciding bot's numeric score AND its component breakdown string for
//!     each option,
//!   - the index it actually chose.
//!
//! Scoring honesty. `score_actions` is a *static evaluation* and exists only
//! for the generic action route (Main phase and choice resolution). The live-set
//! route chooses by rollout, not by a static score, so those options are
//! recorded with `scored: false` and no numbers. A viewer that invented a
//! number there would be worse than one that shows nothing.
//!
//! Usage (from `engine/`):
//!   cargo run --release --bin bot_replay -- v7 v8 "5CP3Z idou" --seed 11 --out ../test_output/replay.json
//!
//! Flags:
//!   --seed N          deterministic deal+decisions (default 11)
//!   --deck NAME       decklist for seat 1 (default "5CP3Z idou")
//!   --deck2 NAME      decklist for seat 2 (default: same as --deck; a
//!                     different name makes the matchup non-mirrored, which is
//!                     the only configuration that can see seat-asymmetric bugs)
//!   --score-as NAME   attribute scores to this bot regardless of who acts
//!                     (default: the acting bot). Use this to compare v6/v7/v8
//!                     on the *same* decision.
//!   --no-scores       skip scoring entirely (fastest; decisions only)
//!   --out PATH        output JSON (default ../test_output/replay.json)
//!   --stdout          print JSON to stdout instead of writing a file
//!
//! Score semantics (recorded in the artifact so a reader cannot misread it):
//! scores are the bot's native evaluation units after any Pass override — NOT
//! win probability, and NOT comparable across bots.

use rabuka_engine::bin_common::{execute_and_settle, fresh_database, load_deck};
use rabuka_engine::bot::{registry::BotKind, strategy_v2, strategy_v3, strategy_v6, strategy_v7,
    strategy_v8};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game_setup::{self, Action, ActionType};
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::rng::Lcg;
use rabuka_engine::turn::TurnEngine;
use serde_json::{json, Value};

type ScoreFn = fn(&GameState, &[Action], u8) -> Vec<(f64, String)>;

struct Options {
    p1: BotKind,
    p2: BotKind,
    seed: u64,
    deck: String,
    deck2: Option<String>,
    score_as: Option<BotKind>,
    scores: bool,
    out: String,
    stdout: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut positional: Vec<String> = Vec::new();
        let mut seed = 11u64;
        let mut deck = "5CP3Z idou".to_string();
        let mut deck2 = None;
        let mut score_as = None;
        let mut scores = true;
        let mut out = "../test_output/replay.json".to_string();
        let mut stdout = false;
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--no-scores" => scores = false,
                "--stdout" => stdout = true,
                "--seed" | "--deck" | "--deck2" | "--score-as" | "--out" => {
                    let value = it
                        .next()
                        .filter(|v| !v.starts_with("--"))
                        .ok_or_else(|| format!("missing value for {arg}"))?
                        .clone();
                    match arg.as_str() {
                        "--seed" => seed = value.parse()?,
                        "--deck" => deck = value,
                        "--deck2" => deck2 = Some(value),
                        "--score-as" => score_as = Some(BotKind::parse(&value)),
                        _ => out = value,
                    }
                }
                other if other.starts_with("--") => {
                    return Err(format!("unknown option: {other}").into())
                }
                other => positional.push(other.to_string()),
            }
        }
        if positional.len() > 2 {
            return Err("expected at most [p1] [p2]; the deck is --deck (its name contains spaces)".into());
        }
        Ok(Self {
            p1: positional
                .first()
                .map(|s| BotKind::parse(s))
                .unwrap_or(BotKind::V7),
            p2: positional
                .get(1)
                .map(|s| BotKind::parse(s))
                .unwrap_or(BotKind::V7),
            seed,
            deck,
            deck2,
            score_as,
            scores,
            out,
            stdout,
        })
    }
}

/// Engine RNG is global mutable state. Policy scoring can draw from it, which
/// would silently shift the trajectory after we record it. Snapshot before,
/// restore after, so recording is observation-only.
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

fn score_fn(kind: BotKind) -> Option<ScoreFn> {
    match kind {
        BotKind::V6 => Some(strategy_v6::score_actions as ScoreFn),
        BotKind::V7 | BotKind::V7Plain | BotKind::V7Rollout => {
            Some(strategy_v7::score_actions as ScoreFn)
        }
        BotKind::V8 => Some(strategy_v8::score_actions_v8 as ScoreFn),
        _ => None,
    }
}

/// Which seat is being asked to decide right now. Mirrors bot_arena: a pending
/// choice is resolved by whoever `can_player_act`, otherwise the active player.
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

fn seat_index(gs: &GameState, player: &rabuka_engine::player::Player) -> u8 {
    u8::from(player.id != gs.player1.id)
}

/// `full` adds printed ability text and the raw action. Options use it (that is
/// the whole point — you are inspecting what the bot chose and why); board
/// cards do not, because a stage of 3 cards and a hand of 10 across 800
/// decisions is tens of megabytes of duplicated text the viewer never reads.
fn card_json(db: &CardDatabase, id: i16, full: bool) -> Value {
    match db.get_card(id) {
        Some(card) => {
            let mut out = json!({
                "id": id,
                "card_no": card.card_no,
                "name": card.name,
                "type": format!("{:?}", card.card_type),
                "cost": card.cost,
                "blade": card.blade,
                "base_hearts": card.base_heart,
                "blade_hearts": card.blade_heart,
                "need_hearts": card.need_heart,
                "score": card.score,
            });
            if full {
                out["abilities"] = json!(card
                    .abilities
                    .iter()
                    .map(|a| a.resolve().full_text.to_string())
                    .collect::<Vec<_>>());
            }
            out
        }
        None => json!({"id": id, "unresolved": true}),
    }
}

fn cards_json(db: &CardDatabase, ids: &[i16], full: bool) -> Vec<Value> {
    ids.iter().map(|&id| card_json(db, id, full)).collect()
}

/// The board exactly as the acting player may see it. Own hand is shown (they
/// are deciding on it); the opponent contributes only public zones. A replay
/// that leaked the opponent hand would teach nothing true about the bot.
fn view_json(gs: &GameState) -> Value {
    let own = decision_player(gs);
    let opponent = if own.id == gs.player1.id {
        &gs.player2
    } else {
        &gs.player1
    };
    let db = &gs.card_database;
    json!({
        "own": {
            "player": own.id,
            "is_first_attacker": own.is_first_attacker,
            "hand": cards_json(db, &own.hand.cards, false),
            "stage": cards_json(db, &own.stage.stage, false),
            "energy_active": own.energy_zone.active_count(),
            "energy_total": own.energy_zone.cards.len(),
            "live_zone": cards_json(db, &own.live_card_zone.cards, true),
            "success_count": own.success_live_card_zone.cards.len(),
            "success": cards_json(db, &own.success_live_card_zone.cards, true),
        },
        "opponent_public": {
            "stage": cards_json(db, &opponent.stage.stage, false),
            "live_zone": cards_json(db, &opponent.live_card_zone.cards, true),
            "success_count": opponent.success_live_card_zone.cards.len(),
            "success": cards_json(db, &opponent.success_live_card_zone.cards, true),
        },
    })
}

/// One-line human description of an option, plus the card it references.
fn describe_action(gs: &GameState, action: &Action) -> Value {
    let db = &gs.card_database;
    let card_id = action.parameters.as_ref().and_then(|p| p.card_id);
    let card = card_id.and_then(|id| db.get_card(id));
    let mut label = action.description.clone();
    if let Some(card) = card {
        let name: &str = &card.name;
        if !label.contains(name) {
            label = format!("{label} — {name}");
        }
    }
    if let Some(selected) = action.selected {
        label = format!("{label} [selected={selected}]");
    }
    json!({
        "label": label,
        "action_type": format!("{:?}", action.action_type),
        "card": card_id.map(|id| card_json(db, id, true)).unwrap_or(Value::Null),
    })
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Route {
    Action,
    Mulligan,
    LiveSet,
}

fn route(gs: &GameState) -> Route {
    if gs.has_pending_choice() {
        return Route::Action;
    }
    match gs.current_phase {
        Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => Route::Mulligan,
        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => Route::LiveSet,
        _ => Route::Action,
    }
}

fn choose(
    gs: &GameState,
    actions: &[Action],
    kind: BotKind,
    v2: &strategy_v2::V2Policy,
    plan: &strategy_v3::V3Plan,
    rng: &mut Lcg,
) -> Action {
    // These three phases are turn-STRUCTURE, not tactical decisions, and
    // routing them through a Main-phase policy is what stalls the game: the
    // policies score options by board value, and every structural option ties,
    // so the argmax is arbitrary and the turn order (and with it energy
    // accrual) never resolves. Matched to bot_arena's handling.
    match gs.current_phase {
        // RPS carries no information to decide from; a seeded random draw is
        // enough to resolve it, and it is what the arena does.
        Phase::RockPaperScissors if !gs.has_pending_choice() => {
            return actions[rng.range(actions.len())].clone();
        }
        // Rule 8.4.13 / arena S6: the RPS loser is the first attacker.
        Phase::ChooseFirstAttacker if !gs.has_pending_choice() => {
            let policy_is_p1 = decision_player(gs).id == gs.player1.id;
            let won_rps = gs.rps_winner == Some(if policy_is_p1 { 1 } else { 2 });
            return if won_rps {
                actions
                    .iter()
                    .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                    .cloned()
                    .unwrap_or_else(|| actions[0].clone())
            } else {
                actions[rng.range(actions.len())].clone()
            };
        }
        _ => {}
    }
    match route(gs) {
        Route::Mulligan => policy_call(|| kind.choose_mulligan(gs, actions, &gs.card_database)),
        Route::LiveSet if kind == BotKind::Random => actions[rng.range(actions.len())].clone(),
        Route::LiveSet => {
            policy_call(|| kind.choose_live_set(gs, actions, &gs.card_database, v2, plan))
        }
        Route::Action if kind == BotKind::Random => actions[rng.range(actions.len())].clone(),
        Route::Action => {
            let me = seat_index(gs, decision_player(gs));
            policy_call(|| kind.choose_action(gs, actions, me, v2, plan))
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // NOTE: deliberately NOT set_training_mode(true). That helper turns OFF
    // action display and rule logging, which are exactly the two things this
    // artifact exists to capture — it left every option label blank and the
    // live_check list empty. RL collectors want it on; a replay wants the
    // opposite.
    let options = Options::parse(&std::env::args().skip(1).collect::<Vec<_>>())?;

    // `build_two_decks` must receive an Arc it SOLELY owns. It registers
    // per-copy card IDs through `Arc::make_mut`; handing it `Arc::clone(&db)`
    // makes `make_mut` deep-clone the shared database and throw the registered
    // copies away, so every deck ends up full of dangling card IDs. The symptom
    // is not a crash — it is a game that LOOKS fine and never plays: energy
    // zone empty for 255 turns, one legal option ("Pass") per Main phase,
    // zero placements, draw. Do not reintroduce the clone.
    let mut db = fresh_database();
    let deck1_numbers = load_deck(&options.deck);
    let deck2_name = options.deck2.clone().unwrap_or_else(|| options.deck.clone());
    let deck2_numbers = load_deck(&deck2_name);

    let (t1, t2) = game_setup::build_two_decks(&mut db, &deck1_numbers, &deck2_numbers)?;
    // ONE call, both lists. `build_two_decks(db, seat1_numbers, seat2_numbers)`
    // takes the two decklists as separate arguments and registers the energy
    // decks as part of the same pass; calling it once per seat re-registered
    // the energy cards.

    let mut gs = rabuka_engine::bin_common::deal_default_game(&db, &t1, &t2);
    rabuka_engine::rng::seed(options.seed as u32);
    let mut rng = Lcg::new(options.seed ^ 0x9E37_79B9_7F4A_7C15);
    let v2 = strategy_v2::V2Policy::default();
    let kinds = [options.p1, options.p2];

    let mut decisions: Vec<Value> = Vec::new();
    let mut timeline: Vec<Value> = Vec::new();
    let mut live_checks: Vec<Value> = Vec::new();
    let mut seen_log = 0usize;
    let mut stuck = 0u32;
    let mut last_key = String::new();
    let plans = [
        policy_call(|| strategy_v3::V3Plan::detect(&gs, 0, &gs.card_database)),
        policy_call(|| strategy_v3::V3Plan::detect(&gs, 1, &gs.card_database)),
    ];
    let mut winner = "draw";

    // Generous: a turn is several decisions, and the project's health metric is
    // a median game of 5-8 turns, but a long replay is still a legal artifact to
    // produce. The per-phase stuck guard below is what actually stops a loop.
    for _step in 0..6000 {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let p1 = gs.player1.success_live_card_zone.cards.len();
            let p2 = gs.player2.success_live_card_zone.cards.len();
            winner = match rabuka_engine::bin_common::classify_winner(&gs) {
                rabuka_engine::bin_common::GameOutcome::P1Win => "p1",
                rabuka_engine::bin_common::GameOutcome::P2Win => "p2",
                _ => "draw",
            };
            eprintln!("game over: p1={p1} p2={p2} -> {winner}");
            break;
        }

        let key = format!("{}:{:?}", gs.active_player().id, gs.current_phase);
        if key == last_key {
            stuck += 1;
            if stuck > 40 {
                winner = "stuck";
                eprintln!("stuck in {key}");
                break;
            }
        } else {
            stuck = 0;
            last_key = key.clone();
        }

        if game_setup::auto_advance_one(&mut gs) {
            continue;
        }
        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            continue;
        }

        // Drain any live-check verdicts the engine logged since the last step.
        for entry in &gs.structured_log[seen_log.min(gs.structured_log.len())..] {
            if entry.category == "live_result" {
                live_checks.push(json!({
                    "turn": entry.turn,
                    "text": entry.text,
                }));
            }
        }
        seen_log = gs.structured_log.len();

        let me = seat_index(&gs, decision_player(&gs));
        let kind = kinds[me as usize];
        // Plans are per-GAME, computed right after the deal, exactly as the
        // arena does. A plan is a snapshot of the opening intent; recomputing
        // it every decision is a different policy, not a fresher one.
        let plan = &plans[me as usize];

        let current_route = route(&gs);
        let scorer = options
            .score_as
            .or(if options.scores && current_route == Route::Action {
                Some(kind)
            } else {
                None
            })
            .and_then(score_fn);

        // Score BEFORE choosing, and restore engine RNG around both, so neither
        // perturbs the trajectory and the numbers describe the real decision.
        let scores: Option<Vec<(f64, String)>> = scorer.map(|score| {
            policy_call(|| {
                let out = score(&gs, &actions, me);
                if out.len() == actions.len() {
                    out
                } else {
                    // A scorer that returns a different arity than it was
                    // offered is a bug; record the mismatch rather than
                    // silently zipping unrelated numbers onto options.
                    eprintln!("warning: scorer arity mismatch");
                    Vec::new()
                }
            })
        });

        let chosen = choose(&gs, &actions, kind, &v2, plan, &mut rng);
        let chosen_value = serde_json::to_value(&chosen).ok();
        let chosen_index = chosen_value
            .as_ref()
            .and_then(|cv| {
                actions
                    .iter()
                    .position(|a| serde_json::to_value(a).ok().as_ref() == Some(cv))
            })
            .unwrap_or(0);

        let options_json: Vec<Value> = actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let mut entry = describe_action(&gs, action);
                match scores.as_ref().and_then(|s| s.get(i)) {
                    Some((score, components)) => {
                        entry["score"] = if score.is_finite() {
                            json!(( *score * 1000.0).round() / 1000.0)
                        } else {
                            Value::Null
                        };
                        entry["components"] = json!(components);
                    }
                    None => {
                        entry["score"] = Value::Null;
                        entry["components"] = Value::Null;
                    }
                }
                entry["index"] = json!(i);
                entry["chosen"] = json!(i == chosen_index);
                entry
            })
            .collect();

        let scored_here = scores.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        decisions.push(json!({
            "turn": gs.turn_number,
            "phase": format!("{:?}", gs.current_phase),
            "seat": me,
            "bot": kind.name(),
            "scored_by": scorer.map(|_| options.score_as.map(|k| k.name()).unwrap_or(kind.name())),
            "scored": scored_here,
            "score_note": if scored_here {
                "native evaluation units; NOT win probability"
            } else if current_route == Route::LiveSet {
                "live-set picks by rollout, not a static score; no numbers exist here"
            } else {
                "no static scorer for this bot"
            },
            "option_count": actions.len(),
            "view": view_json(&gs),
            "options": options_json,
            "chosen_index": chosen_index,
        }));

        timeline.push(json!({
            "turn": gs.turn_number,
            "phase": format!("{:?}", gs.current_phase),
            "succ_p1": gs.player1.success_live_card_zone.cards.len(),
            "succ_p2": gs.player2.success_live_card_zone.cards.len(),
        }));

        execute_and_settle(&mut gs, &chosen).map_err(|e| format!("execute failed: {e}"))?;
    }

    let p1_success = gs.player1.success_live_card_zone.cards.len();
    let p2_success = gs.player2.success_live_card_zone.cards.len();
    let placements_p1 = live_checks
        .iter()
        .filter(|c| c["text"].as_str().unwrap_or("").contains("succ="))
        .count();

    let artifact = json!({
        "format": "rabuka-scored-replay",
        "schema": 1,
        "metadata": {
            "p1": options.p1.name(),
            "p2": options.p2.name(),
            "deck": options.deck,
            "deck2": deck2_name,
            "mirrored": options.deck == deck2_name,
            "seed": options.seed,
        },
        "score_semantics": "each bot's native evaluation units after its own Pass override; NOT win probability; NOT comparable between bots",
        "visibility": "own hand is shown to the acting player; opponent hand is never included",
        "result": {
            "winner": winner,
            "success_p1": p1_success,
            "success_p2": p2_success,
            "turns": gs.turn_number,
            "decisions": decisions.len(),
            "live_check_rows": placements_p1,
        },
        "live_checks": live_checks,
        "timeline": timeline,
        "decisions": decisions,
    });

    let json = serde_json::to_string_pretty(&artifact)?;
    if options.stdout {
        println!("{json}");
    } else {
        let path = std::path::Path::new(&options.out);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &json)?;
        eprintln!(
            "wrote {} ({} decisions, {} turns, result {})",
            path.display(),
            decisions.len(),
            gs.turn_number,
            winner
        );
    }
    Ok(())
}

// Keep the unused import honest: ActionType is referenced in docs above and is
// part of the public action vocabulary this tool serializes.
#[allow(dead_code)]
fn _action_type_is_used() -> ActionType {
    ActionType::Pass
}
