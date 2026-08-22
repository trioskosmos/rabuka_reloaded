//! Turn-replay analyzer: find the plays the bots missed.
//!
//! Protocol:
//!   1. Play ONE game (default v5 vs v2, mirrored deck, fixed seed).
//!   2. Snapshot GameState at every turn entry.
//!   3. When a turn (>= FROM_TURN) ends with NO success placement for
//!      either side, re-drive that whole turn N times with uniform-random
//!      decisions for both players, resuming normal bots after the turn.
//!   4. Report whether ANY random line placed a success, and dump the
//!      action sequence of placing lines so we can see the winning play
//!      versus what the bots actually chose.
//!
//! Deck order is fixed at setup and never reshuffled by this tool, so a
//! random replay differs ONLY in decisions - no cheating, pure undo/redo.

use rabuka_engine::bot::{strategy_v2, strategy_v4, strategy_v5};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::rng::Lcg;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

fn fresh_database() -> Arc<CardDatabase> {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).expect("load cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

fn load_test_deck(db: &Arc<CardDatabase>, name: &str) -> Vec<String> {
    let deck_path = std::path::Path::new("../web_ui/decks").join(format!("{name}.txt"));
    if deck_path.exists() {
        let deck =
            rabuka_engine::deck_parser::DeckParser::parse_deck_file(&deck_path).expect("parse deck");
        return rabuka_engine::deck_parser::DeckParser::deck_list_to_card_numbers(&deck);
    }
    let mut nums: Vec<String> = Vec::new();
    for (_tid, card) in db.cards.iter() {
        if !matches!(card.card_type, rabuka_engine::card::CardType::Energy) && nums.len() < 60 {
            nums.push(card.card_no.to_string());
        }
    }
    nums
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

/// One engine step: auto-advance first, otherwise make one decision.
/// `random_mode`: uniform-random choice for everything.
fn step(
    gs: &mut GameState,
    db: &Arc<CardDatabase>,
    rng: &mut Lcg,
    v2_policy: &strategy_v2::V2Policy,
    random_mode: bool,
) -> bool {
    TurnEngine::check_victory_condition(gs);
    if gs.game_result != GameResult::Ongoing {
        return false;
    }

    if game_setup::auto_advance_one(gs) {
        return true;
    }

    let actions = game_setup::generate_possible_actions(gs);
    if actions.is_empty() {
        TurnEngine::advance_phase(gs);
        return true;
    }

    let active_is_p1 = gs.active_player().id == "p1";
    let me = if active_is_p1 { 0u8 } else { 1u8 };

    // RPS: always random (no information to decide with).
    if gs.current_phase == Phase::RockPaperScissors {
        let a = actions[rng.range(actions.len())].clone();
        let _ = game_setup::execute_action(gs, &a);
        game_setup::settle_single_player_state(gs);
        return true;
    }

    // First-attacker choice: take second attacker if we won RPS, else random.
    if gs.current_phase == Phase::ChooseFirstAttacker {
        let won_rps = gs.rps_winner == Some(if active_is_p1 { 1 } else { 2 });
        let idx = if won_rps && !random_mode {
            actions
                .iter()
                .position(|a| {
                    a.action_type == rabuka_engine::game_setup::ActionType::ChooseSecondAttacker
                })
                .unwrap_or(0)
        } else {
            rng.range(actions.len())
        };
        let _ = game_setup::execute_action(gs, &actions[idx]);
        game_setup::settle_single_player_state(gs);
        return true;
    }

    if matches!(
        gs.current_phase,
        Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
    ) {
        let a = if random_mode {
            actions[rng.range(actions.len())].clone()
        } else {
            strategy_v4::choose_mulligan_v4(gs, &actions, db)
        };
        let _ = game_setup::execute_action(gs, &a);
        game_setup::settle_single_player_state(gs);
        return true;
    }

    if random_mode {
        let a = actions[rng.range(actions.len())].clone();
        let _ = game_setup::execute_action(gs, &a);
        game_setup::settle_single_player_state(gs);
        return true;
    }

    let is_live_set = matches!(
        gs.current_phase,
        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
    );
    let a = if is_live_set {
        if active_is_p1 {
            strategy_v5::choose_live_set_v5(gs, &actions, db)
        } else {
            strategy_v2::choose_live_set_action_v2(gs, &actions, db, v2_policy)
        }
    } else if active_is_p1 {
        strategy_v5::choose_action_v5(gs, &actions, me)
    } else {
        strategy_v2::choose_action_heuristic_v2(gs, &actions, me)
    };
    let _ = game_setup::execute_action(gs, &a);
    game_setup::settle_single_player_state(gs);
    true
}

/// Like `step` in forced random mode, but logs every executed decision.
fn random_step_logged(
    gs: &mut GameState,
    db: &Arc<CardDatabase>,
    rng: &mut Lcg,
    seq: &mut Vec<String>,
) -> bool {
    TurnEngine::check_victory_condition(gs);
    if gs.game_result != GameResult::Ongoing {
        return false;
    }
    if game_setup::auto_advance_one(gs) {
        return true;
    }
    let actions = game_setup::generate_possible_actions(gs);
    if actions.is_empty() {
        TurnEngine::advance_phase(gs);
        return true;
    }
    let idx = rng.range(actions.len());
    let a = actions[idx].clone();
    let who = if gs.active_player().id == "p1" { "P1" } else { "P2" };
    let card = a
        .parameters
        .as_ref()
        .and_then(|p| p.card_id)
        .and_then(|cid| db.get_card(cid))
        .map(|c| c.card_no.to_string())
        .unwrap_or_default();
    seq.push(format!(
        "t{} {} {:?} {} {}",
        gs.turn_number, who, gs.current_phase, a.action_type, card
    ));
    let _ = game_setup::execute_action(gs, &a);
    game_setup::settle_single_player_state(gs);
    true
}

fn succ(gs: &GameState) -> (usize, usize) {
    (
        gs.player1.success_live_card_zone.cards.len(),
        gs.player2.success_live_card_zone.cards.len(),
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let from_turn: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let replays_per_turn: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40);

    let db = fresh_database();
    let nums = load_test_deck(&db, "5CP3Z idou");
    eprintln!("REPLAY deck entries={}", nums.len());
    let mut db_mut = std::sync::Arc::clone(&db);
    let (t1, t2) =
        rabuka_engine::game_setup::build_two_decks(&mut db_mut, &nums, &nums).expect("build decks");

    let mut rng = Lcg(0x5EED_1234_ABCD_0001);
    let v2_policy = strategy_v2::V2Policy::default();

    // ---- Play the baseline game once, snapshotting at each turn entry ----
    let mut gs = deal_from_templates(&db, &t1, &t2);
    let mut snapshots: Vec<GameState> = vec![gs.clone()];
    let mut cur_turn = gs.turn_number;
    let mut steps_in_turn = 0u32;

    for _ in 0..20000 {
        if gs.turn_number != cur_turn {
            cur_turn = gs.turn_number;
            snapshots.push(gs.clone());
            steps_in_turn = 0;
        }
        steps_in_turn += 1;
        if steps_in_turn > 300 {
            eprintln!(
                "STALL: no turn progress at t{} phase={:?}",
                gs.turn_number, gs.current_phase
            );
            break; // oscillation guard, same as bot_arena's stuck counter
        }
        if !step(&mut gs, &db, &mut rng, &v2_policy, false) {
            break;
        }
    }
    println!(
        "BASELINE GAME OVER: t{} succ {:?} result={:?}",
        gs.turn_number,
        succ(&gs),
        gs.game_result
    );

    // ---- Analyze each stalled turn >= from_turn with random replays ----
    let mut report = String::new();
    let mut total_stalled = 0u32;
    let mut turns_with_placing_line = 0u32;

    for si in 0..snapshots.len() {
        let snap = &snapshots[si];
        let snap_turn = snap.turn_number;
        let s_before = succ(snap);
        if snap_turn < from_turn {
            continue;
        }

        // Stalled = next snapshot exists, is exactly one turn later, and
        // nobody's success count changed during that turn.
        let stalled = match snapshots.get(si + 1) {
            Some(next) => {
                next.turn_number == snap_turn + 1 && succ(next) == s_before
            }
            None => false,
        };
        if !stalled {
            continue;
        }
        total_stalled += 1;

        let mut placed_lines: Vec<(String, (usize, usize))> = Vec::new();
        let mut place_count = 0u32;
        for rep in 0..replays_per_turn {
            let mut rgs = snap.clone();
            let seed = rng.next();
            let mut rrng = Lcg(seed);
            let mut seq: Vec<String> = Vec::new();
            let mut guard = 0usize;
            while guard < 400 {
                guard += 1;
                if rgs.turn_number > snap_turn || rgs.game_result != GameResult::Ongoing {
                    break;
                }
                if succ(&rgs) != s_before {
                    break; // placement happened mid-turn; stop here
                }
                if !random_step_logged(&mut rgs, &db, &mut rrng, &mut seq) {
                    break;
                }
            }
            let after = succ(&rgs);
            if after != s_before {
                place_count += 1;
                if placed_lines.len() < 5 {
                    placed_lines.push((format!("rep{rep}: {}", seq.join(" | ")), after));
                }
            }
        }
        if place_count > 0 {
            turns_with_placing_line += 1;
        }
        report.push_str(&format!(
            "\n=== TURN {} STALLED (succ {:?} unchanged) | {}/{} random replays PLACE ===\n",
            snap_turn, s_before, place_count, replays_per_turn
        ));
        for (line, after) in &placed_lines {
            report.push_str(&format!("  PLACING LINE -> succ {:?}\n    {}\n", after, line));
        }
    }

    println!("{}", report);
    println!(
        "SUMMARY: stalled turns >= t{}: {}, of which {} had a placing random line",
        from_turn, total_stalled, turns_with_placing_line
    );
    let _ = std::fs::create_dir_all("../test_output");
    let _ = std::fs::write("../test_output/turn_replay_report.txt", &report);
}
