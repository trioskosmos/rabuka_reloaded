//! Hunt the UseAbility infinite loop: replay v3-vs-v2 games until some side
//! picks an identical action many times in a row within one turn (the
//! "turn-1 draw" pathology seen in bot_arena traces), then dump:
//!   1. the full board state at loop start,
//!   2. the offered action list,
//!   3. clone-eval vs real-execution outcome for the looping action,
//!   4. whether the action is still offered afterwards.
//!
//! Usage: cargo run --config 'profile.dev.opt-level=3' --bin hunt_loop

use rabuka_engine::bot::{strategy_v2, strategy_v3};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

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

fn load_deck(name: &str) -> Vec<String> {
    let deck_path = std::path::Path::new("../web_ui/decks").join(format!("{name}.txt"));
    if deck_path.exists() {
        let deck = deck_parser::DeckParser::parse_deck_file(&deck_path).expect("parse deck");
        return deck_parser::DeckParser::deck_list_to_card_numbers(&deck);
    }
    panic!("deck not found: {name}");
}

fn deal(
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

fn sig(a: &rabuka_engine::game_setup::Action) -> String {
    format!(
        "{:?}|{:?}|{:?}",
        a.action_type,
        a.parameters.as_ref().and_then(|p| p.card_id),
        a.parameters.as_ref().and_then(|p| p.card_indices.clone())
    )
}

fn dump_state(out: &mut String, label: &str, gs: &GameState) {
    out.push_str(&format!("--- {label} ---\n"));
    for (side, p) in [("P1", &gs.player1), ("P2", &gs.player2)] {
        out.push_str(&format!(
            "{} succ={} hand={} energy_active={} stage={:?} live={} waitroom={}\n",
            side,
            p.success_live_card_zone.cards.len(),
            p.hand.cards.len(),
            p.energy_zone.active_count(),
            p.stage.stage,
            p.live_card_zone.cards.len(),
            p.waitroom.cards.len(),
        ));
        out.push_str(&format!("   hand_cards={:?}\n", p.hand.cards));
    }
    out.push_str(&format!(
        "phase={:?} active={}\n",
        gs.current_phase,
        gs.active_player().id
    ));
}

fn main() {
    let db = fresh_database();
    let nums = load_deck("fade deck");
    let mut db_mut = Arc::clone(&db);
    let (t1, t2) =
        game_setup::build_two_decks(&mut db_mut, &nums, &nums).expect("build decks");
    drop(db_mut);

    let mut rng = Lcg(0x5EED_1234_ABCD_0001);
    let v2_policy = strategy_v2::V2Policy::default();
    let mut max_repeat_seen = 0u32;
    let mut max_repeat_info = String::new();
    let mut end_turns: Vec<u8> = Vec::new();
    let mut end_results: Vec<String> = Vec::new();

    'games: for game in 1..=3000u32 {
        let mut gs = deal(&db, &t1, &t2);
        let plan_p1 = strategy_v3::V3Plan::detect(&gs, 0, &db);
        let plan_p2 = strategy_v3::V3Plan::detect(&gs, 1, &db);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        let mut last_sig = String::new();
        let mut repeat = 0u32;

        for _ in 0..600 {
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                break;
            }
            if gs.turn_number == last_turn {
                stuck += 1;
                if stuck > 60 && repeat >= 4 {
                    // Capture happens after the inner loop via `repeat` check.
                    break;
                }
                if stuck > 200 {
                    break;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
                repeat = 0;
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
            let kind_is_v3 = active_is_p1;
            let me = if active_is_p1 { 0u8 } else { 1u8 };

            let action = match gs.current_phase {
                Phase::RockPaperScissors => {
                    let a = actions[rng.range(actions.len())].clone();
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                Phase::ChooseFirstAttacker => {
                    let won = gs.rps_winner == Some(if active_is_p1 { 1 } else { 2 });
                    let a = if won {
                        actions
                            .iter()
                            .find(|a| {
                                a.action_type
                                    == rabuka_engine::game_setup::ActionType::ChooseSecondAttacker
                            })
                            .unwrap_or(&actions[0])
                            .clone()
                    } else {
                        actions[rng.range(actions.len())].clone()
                    };
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => {
                    let a = match kind_is_v3 {
                        false => strategy_v2::choose_mulligan_action_v2(&gs, &actions, &db),
                        true => strategy_v3::choose_mulligan_action_v3(&gs, &actions, &db),
                    };
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => {
                    let plan = if active_is_p1 { &plan_p1 } else { &plan_p2 };
                    let a = match kind_is_v3 {
                        false => strategy_v2::choose_live_set_action_v2(
                            &gs, &actions, &db, &v2_policy,
                        ),
                        true => strategy_v3::choose_live_set_action_v3(
                            &gs, &actions, &db, &v2_policy, plan,
                        ),
                    };
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                _ => {
                    if kind_is_v3 {
                        let plan = if active_is_p1 { &plan_p1 } else { &plan_p2 };
                        strategy_v3::choose_action_heuristic_v3(&gs, &actions, me, plan)
                    } else {
                        strategy_v2::choose_action_heuristic_v2(&gs, &actions, me)
                    }
                }
            };

            let s = sig(&action);
            if s == last_sig {
                repeat += 1;
            } else {
                repeat = 0;
            }
            last_sig = s.clone();
            if repeat > max_repeat_seen {
                max_repeat_seen = repeat;
                max_repeat_info = format!(
                    "game {game} turn {} phase {:?} sig {s}",
                    gs.turn_number, gs.current_phase
                );
            }

            let _ = game_setup::execute_action(&mut gs, &action);
            game_setup::settle_single_player_state(&mut gs);

            if repeat >= 4 {
                // 笏笏 CAPTURE 笏笏
                let mut out = format!(
                    "=== LOOP CAPTURED | game {game} | turn {} | sig {s}\n",
                    gs.turn_number
                );
                dump_state(&mut out, "state at capture (post-exec)", &gs);

                // Re-generate from the captured state and find the same action.
                let actions = game_setup::generate_possible_actions(&gs);
                out.push_str(&format!("offered actions: {}\n", actions.len()));
                for (i, a) in actions.iter().enumerate() {
                    out.push_str(&format!(
                        "  [{i}] {:?} card={:?} desc={}\n",
                        a.action_type,
                        a.parameters.as_ref().and_then(|p| p.card_id),
                        a.description.chars().take(60).collect::<String>()
                    ));
                }
                let still_offered = actions.iter().any(|a| sig(a) == s);
                out.push_str(&format!("looping action still offered: {still_offered}\n"));

                if let Some(a) = actions.iter().find(|a| sig(a) == s) {
                    // Clone-eval: what does the bot's own trial see?
                    let mut sim = gs.clone();
                    let r = game_setup::execute_action(&mut sim, a);
                    out.push_str(&format!("clone-eval execute result: {r:?}\n"));
                    if r.is_ok() {
                        game_setup::settle_single_player_state(&mut sim);
                        dump_state(&mut out, "after CLONE execution + settle", &sim);
                    }
                    // Real execution.
                    let mut real = gs.clone();
                    let r2 = game_setup::execute_action(&mut real, a);
                    out.push_str(&format!("real execute result: {r2:?}\n"));
                    if r2.is_ok() {
                        game_setup::settle_single_player_state(&mut real);
                        dump_state(&mut out, "after REAL execution + settle", &real);
                        let after = game_setup::generate_possible_actions(&real);
                        let again = after.iter().any(|x| sig(x) == s);
                        out.push_str(&format!("still offered after real exec: {again}\n"));
                        let key_used = real
                            .turn_limited_abilities_used
                            .iter()
                            .map(|(k, v)| format!("{k:?}={v}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        out.push_str(&format!(
                            "turn_limited_abilities_used: {key_used}\n"
                        ));
                    }
                }

                let path = std::path::Path::new("../test_output/loop_capture.txt");
                let _ = std::fs::create_dir_all(path.parent().unwrap());
                let _ = std::fs::write(path, &out);
                println!("LOOP CAPTURED (game {game}) 竊・{}", path.display());
                println!("{}", &out[..out.len().min(3000)]);
                break 'games;
            }
        }
        let _ = (&max_repeat_seen, &max_repeat_info);
        end_turns.push(gs.turn_number);
        end_results.push(format!("{:?}", gs.game_result));
    }
    let mut tally = std::collections::HashMap::new();
    for r in &end_results {
        *tally.entry(r.clone()).or_insert(0u32) += 1;
    }
    println!("games played: {}", end_results.len());
    for (k, v) in &tally {
        println!("  {:?} x{}", k, v);
    }
    let short = end_turns.iter().filter(|&&t| t <= 2).count();
    println!("games ending at turn ≤2: {short}");
    println!(
        "no loop captured | max consecutive identical main-phase actions: {max_repeat_seen} ({max_repeat_info})"
    );
}
