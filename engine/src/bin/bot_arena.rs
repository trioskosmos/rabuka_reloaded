//! Bot arena: run N-second matchups between bot versions and random.
//!
//! Usage: cargo run --release --bin bot_arena -- [p1] [p2] [budget_secs]
//!   p1/p2: v1 | v2 | v3 | random   (default: v2 random 10)
//!
//! Moved out of tests/test_modules/strategy_bot_test.rs  Ethis is a
//! benchmark/arena, not a unit test. Run it when you want numbers.

use rabuka_engine::bot::{strategy, strategy_v2, strategy_v3, strategy_v4, strategy_v5};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
enum BotKind {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    Random,
}

fn parse_kind(s: &str) -> BotKind {
    match s {
        "v1" => BotKind::V1,
        "v2" => BotKind::V2,
        "v3" => BotKind::V3,
        "v4" => BotKind::V4,
        "v5" => BotKind::V5,
        "v6" => BotKind::V6,
        "v7" => BotKind::V7,
        _ => BotKind::Random,
    }
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() >> 33) as usize % n
        }
    }
}

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
    for (_tid, card) in db.cards.iter() {
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
            db.get_card(c).map_or(false, |x| {
                x.card_type == rabuka_engine::card::CardType::Live
            })
        })
        .count()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p1_kind = parse_kind(args.get(1).map(|s| s.as_str()).unwrap_or("v2"));
    let p2_kind = parse_kind(args.get(2).map(|s| s.as_str()).unwrap_or("random"));
    let budget: u64 = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let trace = args.iter().any(|a| a == "--trace");
    let logs = args.iter().any(|a| a == "--logs");
    let deck_name = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "5CP3Z idou".to_string());

    let kind_name = |k: BotKind| match k {
        BotKind::V1 => "v1",
        BotKind::V2 => "v2",
        BotKind::V3 => "v3",
        BotKind::V4 => "v4",
        BotKind::V5 => "v5",
        BotKind::V6 => "v6",
        BotKind::V7 => "v7",
        BotKind::Random => "random",
    };

    let mut db = fresh_database();
    let nums = load_test_deck(&db, &deck_name);
    eprintln!(
        "ARENA deck={} entries={} distinct={}",
        deck_name,
        nums.len(),
        nums.iter().collect::<std::collections::HashSet<_>>().len()
    );
    let (t1, t2) = build_templates(&mut db, &nums, &nums);

    let mut rng = Lcg(0x5EED_1234_ABCD_0001);
    let v2_policy = strategy_v2::V2Policy::default();

    let mut wins = [0u32; 2];
    let mut draws = 0u32;
    let mut games = 0u32;
    let mut total_actions = 0u64;
    let mut total_turns = 0u64;
    let t0 = std::time::Instant::now();
    // Optional hard game-count cap (ARENA_GAMES env): removes time-budget
    // truncation bias when comparing logged vs unlogged runs.
    let max_games: u32 = std::env::var("ARENA_GAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(u32::MAX);
    let mut trace_rows: Vec<String> = Vec::new();
    let mut game_start_idx = 0usize;
    if trace {
        trace_rows.push(
            "game,turn,phase,player,action_type,card_no,live_p1,live_p2,success_p1,success_p2"
                .into(),
        );
    }

    while t0.elapsed().as_secs() < budget && games < max_games {
        games += 1;
        let mut gs = deal_from_templates(&db, &t1, &t2);
        // Archetype detection runs once per game over the full own decklist.
        let plan_p1 = strategy_v3::V3Plan::detect(&gs, 0, &db);
        let plan_p2 = strategy_v3::V3Plan::detect(&gs, 1, &db);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        // Live-phase telemetry: snapshot at every phase change so transcripts
        // show WHO set WHAT and whether checks passed.
        let mut prev_phase = gs.current_phase;
        let mut timeline: Vec<String> = Vec::new();
        let mut snap_row = |tag: &str, gs: &GameState| -> String {
            let lives_in_hand = |p: &rabuka_engine::player::Player| {
                p.hand
                    .cards
                    .iter()
                    .filter(|&&c| {
                        db.get_card(c).map_or(false, |x| {
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
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
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
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // Mulligan: v2+ uses its policy; v1/random keep the hand.
            if matches!(
                gs.current_phase,
                Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
            ) {
                let a = match kind {
                    BotKind::V2 => {
                        strategy_v2::choose_mulligan_action_v2(&gs, &actions, &db)
                    }
                    BotKind::V3 => {
                        strategy_v3::choose_mulligan_action_v3(&gs, &actions, &db)
                    }
                    BotKind::V4 => strategy_v4::choose_mulligan_v4(&gs, &actions, &db),
                    BotKind::V5 | BotKind::V6 => strategy_v4::choose_mulligan_v4(&gs, &actions, &db),
                    _ => actions
                        .iter()
                        .find(|a| {
                            matches!(
                                a.action_type,
                                rabuka_engine::game_setup::ActionType::ConfirmMulligan
                                    | rabuka_engine::game_setup::ActionType::SkipMulligan
                            )
                        })
                        .unwrap_or(&actions[0])
                        .clone(),
                };
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // Live card set.
            if matches!(
                gs.current_phase,
                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
            ) {
                let a = match kind {
                    BotKind::V1 => strategy::choose_live_set_action(&gs, &actions, &db),
                    BotKind::V2 => {
                        strategy_v2::choose_live_set_action_v2(&gs, &actions, &db, &v2_policy)
                    }
                    BotKind::V3 => {
                        let plan = if active_is_p1 { &plan_p1 } else { &plan_p2 };
                        strategy_v3::choose_live_set_action_v3(
                            &gs, &actions, &db, &v2_policy, plan,
                        )
                    }
                    BotKind::V4 => strategy_v4::choose_live_set_v4(&gs, &actions, &db),
                    BotKind::V7 => rabuka_engine::bot::rollout::choose_live_set_v7(&gs, &actions, &db),
                    BotKind::V5 | BotKind::V6 => strategy_v5::choose_live_set_v5(&gs, &actions, &db),
                    BotKind::Random => actions[rng.range(actions.len())].clone(),
                };
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
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
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
            let action = match kind {
                BotKind::V1 => strategy::choose_action_heuristic(&gs, &actions, me),
                BotKind::V2 => strategy_v2::choose_action_heuristic_v2(&gs, &actions, me),
                BotKind::V3 => {
                    let plan = if active_is_p1 { &plan_p1 } else { &plan_p2 };
                    strategy_v3::choose_action_heuristic_v3(&gs, &actions, me, plan)
                }
                BotKind::V4 => strategy_v4::choose_action_v4(&gs, &actions, me),
                BotKind::V6 | BotKind::V7 => strategy_v5::choose_action_v6(&gs, &actions, me),
                BotKind::V5 => strategy_v5::choose_action_v5(&gs, &actions, me),
                BotKind::Random => actions[rng.range(actions.len())].clone(),
            };
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
            let _ = game_setup::execute_action(&mut gs, &action);
            game_setup::settle_single_player_state(&mut gs);
            total_actions += 1;
        }
        total_turns += gs.turn_number as u64;

        let z1 = gs.player1.success_live_card_zone.cards.len();
        let z2 = gs.player2.success_live_card_zone.cards.len();
        if z1 >= 3 && z2 <= 2 {
            wins[0] += 1;
        } else if z2 >= 3 && z1 <= 2 {
            wins[1] += 1;
        } else {
            draws += 1;
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
            let _ = std::fs::write(dir.join(format!("game_{games:03}.txt")), out);

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
                let _ = std::fs::write(
                    dir.join(format!("replay_game_{games:03}.txt")),
                    replay,
                );
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
    if trace {
        let path = std::path::Path::new("../test_output/bot_arena_trace.csv");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = std::fs::write(path, trace_rows.join("\n") + "\n");
        eprintln!("trace written to {}", path.display());
    }
}

