//! Experimental strategy-bot tests.
//!
//! 1. The fairness property: `evaluate_state` must not change when the
//!    opponent's *hidden* cards change (only public info may influence it).
//! 2. The heuristic policy beats uniform-random play over a batch of games.

use rabuka_engine::bot::{strategy, strategy_v2, strategy_v3};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

/// Fresh standalone database (refcount 1). Required for full-game setup:
/// `build_two_decks` -> `add_default_energy_cards_from_database` uses
/// `Arc::make_mut`, which on a *shared* Arc would register the new energy
/// card copies in a temporary deep-cloned database and drop it, leaving the
/// decks with dangling energy IDs.
fn fresh_database() -> Arc<CardDatabase> {
    let cards_json = include_str!("../../../cards/cards.json");
    let cards =
        rabuka_engine::card_loader::CardLoader::load_cards_from_strs(cards_json)
            .expect("Failed to load embedded cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

fn load_test_deck(db: &Arc<CardDatabase>) -> Vec<String> {
    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    if deck_path.exists() {
        let deck = DeckParser::parse_deck_file(deck_path).expect("parse fade deck");
        return DeckParser::deck_list_to_card_numbers(&deck);
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

fn deal_game(db: &mut Arc<CardDatabase>, nums1: &[String], nums2: &[String]) -> GameState {
    let (t1, t2) = game_setup::build_two_decks(db, nums1, nums2).expect("build decks");
    deal_from_templates(db, &t1, &t2)
}

/// Deal a fresh game from pre-built template decks (cheap: clones card-ID
/// queues, not Card structs). Mirrors `bin_common::deal_game`.
fn deal_from_templates(
    db: &Arc<CardDatabase>,
    t1: &rabuka_engine::deck_builder::Deck,
    t2: &rabuka_engine::deck_builder::Deck,
) -> GameState {
    let mut d1 = t1.clone();
    d1.shuffle_main_deck();
    d1.shuffle_energy_deck();
    let mut d2 = t2.clone();
    d2.shuffle_main_deck();
    d2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    let mut p2 = Player::new("p2".into(), "P2".into(), false);
    p1.set_main_deck(d1.main_deck);
    p1.set_energy_deck(d1.energy_deck);
    p2.set_main_deck(d2.main_deck);
    p2.set_energy_deck(d2.energy_deck);

    let mut gs = GameState::new(p1, p2, Arc::clone(db));
    game_setup::setup_game(&mut gs);
    gs
}

/// Build deck templates once (expensive: 144 per-copy Card clones) for
/// time-bounded benchmarks.
fn build_templates(db: &mut Arc<CardDatabase>, nums: &[String]) -> (rabuka_engine::deck_builder::Deck, rabuka_engine::deck_builder::Deck) {
    game_setup::build_two_decks(db, nums, nums).expect("build decks")
}

/// Tiny deterministic PRNG so the test does not depend on external crates.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// Fairness: evaluation must be invariant to changes in the opponent's hand
/// contents / deck order (hidden information).
#[test]
fn evaluate_state_ignores_opponent_hidden_cards() {
    let mut db = fresh_database();
    let nums = load_test_deck(&db);
    let mut gs = deal_game(&mut db, &nums, &nums);

    // Advance past setup into a normal phase.
    gs.current_phase = rabuka_engine::game_state::Phase::Main;

    let base = strategy::evaluate_state(&gs, 0, &strategy::StrategyWeights::fair());

    // Swap the opponent's entire hand and deck order around; value must not move.
    let orig_hand = gs.player2.hand.cards.clone();
    let mut reversed_hand = orig_hand.clone();
    reversed_hand.reverse();
    gs.player2.hand.cards = reversed_hand;
    gs.player2.main_deck.cards.reverse();

    let after = strategy::evaluate_state(&gs, 0, &strategy::StrategyWeights::fair());
    assert_eq!(
        base, after,
        "evaluate_state changed when only opponent hidden info changed"
    );
}

/// The heuristic policy must beat uniform-random play over a batch of games.
/// Every game is traced to `test_output/strategy_bot/game_NNN.csv`
/// (columns: game,turn,phase,player,action_type,card_no,description,resulting_success_p1,resulting_success_p2)
/// so move distributions can be analyzed in a spreadsheet.
#[test]
fn strategy_bot_beats_random() {
    let mut db = fresh_database();
    let nums = load_test_deck(&db);

    // P1 = strategy bot (heuristic), P2 = uniform random.
    // Runs for a fixed time budget instead of a fixed game count.
    const BUDGET_SECS: u64 = 10;
    let mut p1_wins = 0u32;
    let mut p2_wins = 0u32;
    let mut draws = 0u32;
    let mut rng = Lcg(0x5EED_1234_ABCD_0001);

    let out_dir = std::path::Path::new("../test_output/strategy_bot");
    let _ = std::fs::create_dir_all(out_dir);
    let mut tally_p1: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut tally_p2: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let (t1, t2) = build_templates(&mut db, &nums);
    let t0 = std::time::Instant::now();
    let mut total_actions = 0u64;
    let mut total_turns = 0u64;
    let mut total_iters = 0u64;
    let mut t_gen = 0.0f64;
    let mut t_exec = 0.0f64;
    let mut t_settle = 0.0f64;
    let mut bot_evals = 0u64;

    let mut game_idx = 0usize;
    while t0.elapsed().as_secs() < BUDGET_SECS {
        game_idx += 1;
        let mut gs = deal_from_templates(&db, &t1, &t2);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        let mut rows: Vec<String> = Vec::new();
        rows.push(
            "game,turn,phase,player,action_type,card_no,description,success_p1,success_p2".into(),
        );

        for _ in 0..600 {
            total_iters += 1;
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
                continue;
            }

            let tg = std::time::Instant::now();
            let actions = game_setup::generate_possible_actions(&gs);
            t_gen += tg.elapsed().as_secs_f64();
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            use rabuka_engine::game_setup::ActionType;
            let active_is_p1 = gs.active_player().id == "p1";

            // RPS: sandbox mode routes the same prompt list to P1 then P2
            // (turn/actions.rs:119). Both sides play randomly here.
            if gs.current_phase == rabuka_engine::game_state::Phase::RockPaperScissors {
                let a = &actions[rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // S6: when P1 wins RPS, take second attacker (see docs/BOT_STRATEGY.md).
            if gs.current_phase == rabuka_engine::game_state::Phase::ChooseFirstAttacker {
                let a = if gs.rps_winner == Some(1) {
                    actions
                        .iter()
                        .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                        .unwrap_or(&actions[0])
                } else {
                    &actions[rng.range(actions.len())]
                };
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // Mulligan: conclude immediately (keep hand)  Eper-card Select
            // toggles can loop forever otherwise (see match_runner::ai_turn).
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::MulliganFirstAttacker
                    | rabuka_engine::game_state::Phase::MulliganSecondAttacker
            ) {
                if let Some(a) = actions.iter().find(|a| {
                    matches!(a.action_type, ActionType::ConfirmMulligan | ActionType::SkipMulligan)
                }) {
                    let _ = game_setup::execute_action(&mut gs, a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
            }

            // Live Card Set: strategy policy (select best lives, then confirm).
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
                    | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker
            ) {
                let a = strategy::choose_live_set_action(&gs, &actions, &db);
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            let action = if active_is_p1 {
                bot_evals += actions.len() as u64;
                strategy::choose_action_heuristic(&gs, &actions, 0)
            } else {
                actions[rng.range(actions.len())].clone()
            };

            let card_no = action
                .parameters
                .as_ref()
                .and_then(|p| p.card_id)
                .and_then(|cid| db.get_card(cid))
                .map(|c| c.card_no.to_string())
                .unwrap_or_default();
            let tally = if active_is_p1 {
                &mut tally_p1
            } else {
                &mut tally_p2
            };
            *tally.entry(format!("{:?}", action.action_type)).or_insert(0) += 1;

            let te = std::time::Instant::now();
            let _ = game_setup::execute_action(&mut gs, &action);
            t_exec += te.elapsed().as_secs_f64();
            let ts = std::time::Instant::now();
            game_setup::settle_single_player_state(&mut gs);
            t_settle += ts.elapsed().as_secs_f64();
            total_actions += 1;

            rows.push(format!(
                "{},{},{:?},{},{},{},{},{},{}",
                game_idx,
                gs.turn_number,
                gs.current_phase,
                if active_is_p1 { "strategy" } else { "random" },
                action.action_type,
                csv_escape(&card_no),
                csv_escape(&action.description),
                gs.player1.success_live_card_zone.cards.len(),
                gs.player2.success_live_card_zone.cards.len(),
            ));
        }
        total_turns += gs.turn_number as u64;

        let p1z = gs.player1.success_live_card_zone.cards.len();
        let p2z = gs.player2.success_live_card_zone.cards.len();
        if p1z >= 3 && p2z <= 2 {
            p1_wins += 1;
        } else if p2z >= 3 && p1z <= 2 {
            p2_wins += 1;
        } else {
            draws += 1;
        }

        rows.push(format!(
            ",,,,,,RESULT p1={p1z} p2={p2z},{p1z},{p2z}"
        ));
        let path = out_dir.join(format!("game_{game_idx:03}.csv"));
        let _ = std::fs::write(&path, rows.join("\n") + "\n");
    }

    // Per-player, per-action-type tallies for spreadsheet analysis.
    fn write_tally(path: &std::path::Path, m: &std::collections::HashMap<String, u64>) {
        let mut lines: Vec<String> = vec!["action_type,count".into()];
        let mut v: Vec<_> = m.iter().collect();
        v.sort();
        for (k, c) in v {
            lines.push(format!("{k},{c}"));
        }
        let _ = std::fs::write(path, lines.join("\n") + "\n");
    }
    write_tally(&out_dir.join("strategy_action_tally.csv"), &tally_p1);
    write_tally(&out_dir.join("random_action_tally.csv"), &tally_p2);
    // Combined (backwards compat).
    {
        let mut combined = tally_p1.clone();
        for (k, v) in tally_p2 {
            *combined.entry(k).or_insert(0) += v;
        }
        write_tally(&out_dir.join("action_tally.csv"), &combined);
    }
    // One-line title file so the spreadsheet folder is self-documenting.
    let _ = std::fs::write(
        out_dir.join("README.txt"),
        format!(
            "P1 = strategy bot (engine/src/bot/strategy.rs)\nP2 = uniform random\nDeck: web_ui/decks/fade deck.txt (mirror)\nBudget: {BUDGET_SECS}s ({game_idx} games)\nCSV columns: game,turn,phase,player(action owner),action_type,card_no,description,success_p1,success_p2 (player values are strategy vs random, not p1/p2 ids)\n"
        ),
    );
    eprintln!("move traces written to {} (P1=strategy, P2=random)", out_dir.display());

    let dt = t0.elapsed().as_secs_f64();
    eprintln!(
        "strategy (P1) vs random (P2) over {} games in {:.1}s: {:.1} gps | P1 {} - P2 {} - draws {}",
        game_idx,
        dt,
        game_idx as f64 / dt.max(0.001),
        p1_wins, p2_wins, draws
    );
    eprintln!(
        "  per game: {:.1} turns, {:.1} player actions, {:.1} loop iters | {:.0} actions/s, {:.0} iters/s | bot evaluated {:.0} candidates/s ({:.1}/decision)",
        total_turns as f64 / game_idx as f64,
        total_actions as f64 / game_idx as f64,
        total_iters as f64 / game_idx as f64,
        total_actions as f64 / dt,
        total_iters as f64 / dt,
        bot_evals as f64 / dt,
        bot_evals as f64 / total_actions.max(1) as f64
    );
    eprintln!(
        "  time split: gen {:.1}% | exec {:.1}% | settle {:.1}% | other {:.1}%",
        100.0 * t_gen / dt,
        100.0 * t_exec / dt,
        100.0 * t_settle / dt,
        100.0 * (dt - t_gen - t_exec - t_settle).max(0.0) / dt
    );
    assert!(
        p1_wins > p2_wins,
        "strategy bot ({p1_wins} wins) did not beat random ({p2_wins} wins) over {game_idx} games"
    );
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// V2 (yell-math live-set policy + mulligan policy) vs V1 (greedy live-set,
/// no mulligan). Both sides use the same heuristic main-phase eval — the
/// difference is purely the phase policies. Target: v2 dominates v1 at a
/// rate comparable to v1's edge over random.
#[test]
fn strategy_v2_beats_v1() {
    let mut db = fresh_database();
    let nums = load_test_deck(&db);
    let (t1, t2) = build_templates(&mut db, &nums);

    const BUDGET_SECS: u64 = 10;
    let mut rng = Lcg(0x2B0C_5EED_DEAD_0002);
    let t0 = std::time::Instant::now();
    let mut p1_wins = 0u32;
    let mut p2_wins = 0u32;
    let mut draws = 0u32;
    let mut total_actions = 0u64;
    let mut total_turns = 0u64;
    let policy = rabuka_engine::bot::V2Policy::default();

    let mut game_idx = 0usize;
    while t0.elapsed().as_secs() < BUDGET_SECS {
        game_idx += 1;
        let mut gs = deal_from_templates(&db, &t1, &t2);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;

        for _ in 0..600 {
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
                continue;
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            use rabuka_engine::game_setup::ActionType;
            let active_is_p1 = gs.active_player().id == "p1";

            // RPS: sandbox routes prompts to P1 then P2; both random.
            if gs.current_phase == rabuka_engine::game_state::Phase::RockPaperScissors {
                let a = &actions[rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // S6: winner of RPS prefers second attacker when it is P1's pick.
            if gs.current_phase == rabuka_engine::game_state::Phase::ChooseFirstAttacker {
                let a = if gs.rps_winner == Some(1) {
                    actions
                        .iter()
                        .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                        .unwrap_or(&actions[0])
                } else {
                    &actions[rng.range(actions.len())]
                };
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // Mulligan: P1 uses the v2 curve policy, P2 keeps its hand.
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::MulliganFirstAttacker
                    | rabuka_engine::game_state::Phase::MulliganSecondAttacker
            ) {
                let a = if active_is_p1 {
                    strategy_v2::choose_mulligan_action_v2(&gs, &actions, &db)
                } else if let Some(a) = actions.iter().find(|a| {
                    matches!(a.action_type, ActionType::ConfirmMulligan | ActionType::SkipMulligan)
                }) {
                    a.clone()
                } else {
                    actions[rng.range(actions.len())].clone()
                };
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // Live Card Set: P1 = v2 yell math, P2 = v1 greedy.
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
                    | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker
            ) {
                let a = if active_is_p1 {
                    strategy_v2::choose_live_set_action_v2(
                        &gs,
                        &actions,
                        &db,
                        &policy,
                    )
                } else {
                    strategy::choose_live_set_action(&gs, &actions, &db)
                };
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            // Main phase: P1 uses the v2 evaluation (pressure + blade value).
            let action = if active_is_p1 {
                strategy_v2::choose_action_heuristic_v2(&gs, &actions, 0)
            } else {
                strategy::choose_action_heuristic(&gs, &actions, 1)
            };
            let _ = game_setup::execute_action(&mut gs, &action);
            game_setup::settle_single_player_state(&mut gs);
            total_actions += 1;
        }
        total_turns += gs.turn_number as u64;
        let p1z = gs.player1.success_live_card_zone.cards.len();
        let p2z = gs.player2.success_live_card_zone.cards.len();
        if p1z >= 3 && p2z <= 2 {
            p1_wins += 1;
        } else if p2z >= 3 && p1z <= 2 {
            p2_wins += 1;
        } else {
            draws += 1;
        }
        drop(gs);
    }

    let dt = t0.elapsed().as_secs_f64();
    eprintln!(
        "v2 (P1) vs v1 (P2) over {} games in {:.1}s: {:.1} gps | P1 {} - P2 {} - draws {}",
        game_idx,
        dt,
        game_idx as f64 / dt.max(0.001),
        p1_wins, p2_wins, draws
    );
    eprintln!(
        "  per game: {:.1} turns, {:.1} player actions",
        total_turns as f64 / game_idx as f64,
        total_actions as f64 / game_idx as f64
    );
    assert!(
        p1_wins > p2_wins,
        "v2 ({p1_wins} wins) did not beat v1 ({p2_wins} wins) over {game_idx} games"
    );
}

/// V3 (planning: rush window, curve milestones, acquisition deltas) vs V2.
/// Runs two mirror matchups: the fade deck (pure ramp — no cheap lives) and
/// the hasunosora cup deck (cheap lives → rush plan activates). V3 must beat
/// V2 in BOTH, proving the planning layer helps in both modes.
#[test]
fn strategy_v3_beats_v2() {
    let mut results = Vec::new();
    for deck_file in ["fade deck.txt", "hasunosora_cup.txt"] {
        let (p1_wins, p2_wins, draws, games) = run_v3_vs_v2(deck_file);
        eprintln!(
            "v3 vs v2 [{deck_file}]: {} games | P1(v3) {} - P2(v2) {} - draws {}",
            games, p1_wins, p2_wins, draws
        );
        results.push((deck_file.to_string(), p1_wins, p2_wins));
    }
    for (deck, p1w, p2w) in &results {
        assert!(
            p1w > p2w,
            "v3 ({p1w}) did not beat v2 ({p2w}) on {deck}"
        );
    }
}

fn run_v3_vs_v2(deck_file: &str) -> (u32, u32, u32, usize) {
    let mut db = fresh_database();
    let deck_path = std::path::Path::new("../web_ui/decks")
        .join(deck_file);
    let deck = DeckParser::parse_deck_file(&deck_path)
        .unwrap_or_else(|e| panic!("parse {deck_file}: {e}"));
    let nums = DeckParser::deck_list_to_card_numbers(&deck);
    let (t1, t2) = build_templates(&mut db, &nums);

    const BUDGET_SECS: u64 = 8;
    let mut rng = Lcg(0x3CEF_0001_BEEF_0003);
    let t0 = std::time::Instant::now();
    let mut p1_wins = 0u32;
    let mut p2_wins = 0u32;
    let mut draws = 0u32;
    let mut stalls = 0u32;
    let policy = rabuka_engine::bot::V2Policy::default();

    let mut game_idx = 0usize;
    while t0.elapsed().as_secs() < BUDGET_SECS {
        game_idx += 1;
        let mut gs = deal_from_templates(&db, &t1, &t2);
        // Both sides get their own plan (detected from their own perspective).
        let plan_p1 = rabuka_engine::bot::V3Plan::detect(&gs, 0, &db);
        let plan_p2 = rabuka_engine::bot::V3Plan::detect(&gs, 1, &db);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        let mut stalled = false;
        let mut last_actions: Vec<String> = Vec::new();

        for _ in 0..600 {
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
                continue;
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            use rabuka_engine::game_setup::ActionType;
            let active_is_p1 = gs.active_player().id == "p1";
            let (plan_me, my_index) = if active_is_p1 {
                (&plan_p1, 0usize)
            } else {
                (&plan_p2, 1usize)
            };

            if gs.current_phase == rabuka_engine::game_state::Phase::RockPaperScissors {
                let a = &actions[rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            if gs.turn_number == last_turn {
                stuck += 1;
                if stuck > 200 {
                    stalled = true;
                    break;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
            }

            if gs.current_phase == rabuka_engine::game_state::Phase::ChooseFirstAttacker {
                let a = if gs.rps_winner == Some(1) && active_is_p1 {
                    actions
                        .iter()
                        .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                        .unwrap_or(&actions[0])
                } else if gs.rps_winner == Some(2) && !active_is_p1 {
                    actions
                        .iter()
                        .find(|a| a.action_type == ActionType::ChooseSecondAttacker)
                        .unwrap_or(&actions[0])
                } else {
                    &actions[rng.range(actions.len())]
                };
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::MulliganFirstAttacker
                    | rabuka_engine::game_state::Phase::MulliganSecondAttacker
            ) {
                // Both sides use the v2 mulligan (shared capability).
                let a = strategy_v2::choose_mulligan_action_v2(&gs, &actions, &db);
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
                    | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker
            ) {
                let a = if active_is_p1 {
                    strategy_v3::choose_live_set_action_v3(&gs, &actions, &db, &policy, plan_me)
                } else {
                    strategy_v2::choose_live_set_action_v2(&gs, &actions, &db, &policy)
                };
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            let action = if active_is_p1 {
                strategy_v3::choose_action_heuristic_v3(&gs, &actions, my_index as u8, plan_me)
            } else {
                strategy_v2::choose_action_heuristic_v2(&gs, &actions, my_index as u8)
            };
            last_actions.push(format!(
                "t{} {:?} {}",
                gs.turn_number,
                gs.current_phase,
                action.description
            ));
            if last_actions.len() > 6 {
                last_actions.remove(0);
            }
            let exec_result = game_setup::execute_action(&mut gs, &action);
            let exec_note = match &exec_result {
                Ok(()) => "ok".to_string(),
                Err(e) => format!("ERR: {e}"),
            };
            if gs.turn_number == last_turn && stuck > 180 {
                eprintln!(
                    "[loopdiag] t={} stuck={} hand={} wr_lives={} energy={}/{} act='{}' -> {}",
                    gs.turn_number,
                    stuck,
                    gs.active_player().hand.cards.len(),
                    gs.active_player()
                        .waitroom
                        .cards
                        .iter()
                        .filter(|&&cid| {
                            gs.card_database
                                .get_card(cid)
                                .map_or(false, |c| matches!(c.card_type, rabuka_engine::card::CardType::Live))
                        })
                        .count(),
                    gs.active_player().energy_zone.active_count(),
                    gs.active_player().energy_zone.cards.len(),
                    action.description.chars().take(40).collect::<String>(),
                    exec_note
                );
            }
            game_setup::settle_single_player_state(&mut gs);
        }

        let p1z = gs.player1.success_live_card_zone.cards.len();
        let p2z = gs.player2.success_live_card_zone.cards.len();
        if stalled {
            stalls += 1;
            if stalls <= 3 {
                eprintln!(
                    "[stall #{stalls}] turn={} phase={:?} p1z={p1z} p2z={p2z} pending={} p1(hand={},deck={},energy={}/{}) p2(hand={},deck={},energy={}/{}) recent: {:?}",
                    gs.turn_number,
                    gs.current_phase,
                    gs.has_pending_choice(),
                    gs.player1.hand.cards.len(),
                    gs.player1.main_deck.cards.len(),
                    gs.player1.energy_zone.active_count(),
                    gs.player1.energy_zone.cards.len(),
                    gs.player2.hand.cards.len(),
                    gs.player2.main_deck.cards.len(),
                    gs.player2.energy_zone.active_count(),
                    gs.player2.energy_zone.cards.len(),
                    last_actions
                );
            }
        } else if p1z >= 3 && p2z <= 2 {
            p1_wins += 1;
        } else if p2z >= 3 && p1z <= 2 {
            p2_wins += 1;
        } else {
            draws += 1;
        }
        drop(gs);
    }

    let dt = t0.elapsed().as_secs_f64();
    eprintln!(
        "  outcomes: v3 {} - v2 {} - rule-draws {} - STALLS {}",
        p1_wins, p2_wins, draws, stalls
    );
    eprintln!(
        "  ({game_idx} games in {dt:.1}s, {:.1} gps)",
        game_idx as f64 / dt.max(0.001)
    );
    (p1_wins, p2_wins, draws, game_idx)
}

#[test]
fn bench_random_vs_random() {
    let mut db = fresh_database();
    let nums = load_test_deck(&db);
    let (t1, t2) = build_templates(&mut db, &nums);
    const BUDGET_SECS: u64 = 10;
    let mut rng = Lcg(0xC0FFEE_1234_5678);
    let t0 = std::time::Instant::now();
    let mut games = 0usize;
    let mut p1_wins = 0u32;
    let mut total_actions = 0u64;
    let mut total_turns = 0u64;
    let mut total_iters = 0u64;
    let mut t_gen = 0.0f64;
    let mut t_exec = 0.0f64;
    let mut t_settle = 0.0f64;
    let mut t_victory = 0.0f64;
    let mut t_deal = 0.0f64;
    while t0.elapsed().as_secs() < BUDGET_SECS {
        games += 1;
        let td = std::time::Instant::now();
        let mut gs = deal_from_templates(&db, &t1, &t2);
        t_deal += td.elapsed().as_secs_f64();
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        for _ in 0..600 {
            total_iters += 1;
            let tv = std::time::Instant::now();
            TurnEngine::check_victory_condition(&mut gs);
            t_victory += tv.elapsed().as_secs_f64();
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
                continue;
            }
            let tg = std::time::Instant::now();
            let actions = game_setup::generate_possible_actions(&gs);
            t_gen += tg.elapsed().as_secs_f64();
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }
            use rabuka_engine::game_setup::ActionType;
            if gs.current_phase == rabuka_engine::game_state::Phase::RockPaperScissors
                || gs.current_phase == rabuka_engine::game_state::Phase::ChooseFirstAttacker
            {
                let a = &actions[rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::MulliganFirstAttacker
                    | rabuka_engine::game_state::Phase::MulliganSecondAttacker
            ) {
                if let Some(a) = actions
                    .iter()
                    .find(|a| matches!(a.action_type, ActionType::ConfirmMulligan | ActionType::SkipMulligan))
                {
                    let _ = game_setup::execute_action(&mut gs, a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
            }
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
                    | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker
            ) {
                // Random live selection among all available options.
                let a = &actions[rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }
            let a = actions[rng.range(actions.len())].clone();
            let te = std::time::Instant::now();
            let _ = game_setup::execute_action(&mut gs, &a);
            t_exec += te.elapsed().as_secs_f64();
            let ts = std::time::Instant::now();
            game_setup::settle_single_player_state(&mut gs);
            t_settle += ts.elapsed().as_secs_f64();
            total_actions += 1;
        }
        total_turns += gs.turn_number as u64;
        let p1z = gs.player1.success_live_card_zone.cards.len();
        let p2z = gs.player2.success_live_card_zone.cards.len();
        if p1z >= 3 && p2z <= 2 {
            p1_wins += 1;
        }
        // Drop the finished game BEFORE dealing the next one: GameState holds
        // an Arc to the database, and build_two_decks' Arc::make_mut would
        // otherwise deep-clone the entire database on every deal.
        drop(gs);
    }
    let dt = t0.elapsed().as_secs_f64();
    eprintln!(
        "random vs random over {} games in {:.1}s: {:.1} gps, P1 wins {}",
        games,
        dt,
        games as f64 / dt.max(0.001),
        p1_wins
    );
    eprintln!(
        "  per game: {:.1} turns, {:.1} player actions, {:.1} loop iters | {:.0} actions/s, {:.0} iters/s",
        total_turns as f64 / games as f64,
        total_actions as f64 / games as f64,
        total_iters as f64 / games as f64,
        total_actions as f64 / dt,
        total_iters as f64 / dt
    );
    eprintln!(
        "  time split: gen {:.1}% | exec {:.1}% | settle {:.1}% | victory {:.1}% | deal {:.1}% | other {:.1}%",
        100.0 * t_gen / dt,
        100.0 * t_exec / dt,
        100.0 * t_settle / dt,
        100.0 * t_victory / dt,
        100.0 * t_deal / dt,
        100.0 * (dt - t_gen - t_exec - t_settle - t_victory - t_deal).max(0.0) / dt
    );
}
