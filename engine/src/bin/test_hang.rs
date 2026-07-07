/// Simulates the exact 3DS main loop behavior using REAL cards
use std::sync::Arc;
use std::path::Path;
use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::deck_builder::DeckBuilder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

fn main() {
    println!("=== Building game with real cards ===");
    let json_path = Path::new("../cards/cards.json");
    let mut db = if json_path.exists() {
        let cards: std::collections::HashMap<String, Card> = serde_json::from_str(&std::fs::read_to_string(json_path).unwrap()).unwrap();
        let cards_vec: Vec<Card> = cards.into_values().collect();
        Arc::new(CardDatabase::load_or_create(cards_vec))
    } else {
        panic!("Could not find cards.json at {:?}", json_path);
    };

    let deck_dir = Path::new("../web_ui/decks");
    let v = DeckParser::parse_all_decks_from_directory(deck_dir).expect("deck parse");
    let nums = DeckParser::deck_list_to_card_numbers(&v[0]);
    
    let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums.clone()).expect("pd1");
    let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums).expect("pd2");
    
    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();

    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "P2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    println!("=== Game Ready. Phase: {:?} ===", gs.current_phase);

    let script: &[(&str, usize)] = &[
        ("RPS Rock",   0),
        ("RPS Scissors", 2),
        ("Choose First Attacker", 0),
        ("Skip Mulligan P1", 99),
        ("Skip Mulligan P2", 99),
    ];

    let mut frames = 0usize;
    let mut script_step = 0usize;
    let mut acts_cache: Vec<game_setup::Action> = Vec::new();
    let mut dirty = true;
    let mut stall_count = 0usize;

    loop {
        frames += 1;
        if frames > 2000 {
            println!("!!! ABORT: {} frames without completing (stuck)", frames);
            break;
        }

        if !gs.has_pending_choice()
            && gs.game_result == GameResult::Ongoing
            && game_setup::is_automatic_phase(&gs)
        {
            let before = format!("{:?}", gs.current_phase);
            game_setup::settle_single_player_state(&mut gs);
            println!("[frame {}] AUTO-ADVANCE {} -> {:?}", frames, before, gs.current_phase);
            dirty = true;
        }

        if dirty {
            acts_cache = game_setup::generate_possible_actions(&gs);
            dirty = false;
            // Only print actions occasionally so we don't spam the log for 2000 frames
            if frames % 100 == 0 || frames < 10 {
                println!(
                    "[frame {}] ACTIONS({}): phase={:?}",
                    frames, acts_cache.len(), gs.current_phase
                );
            }
            stall_count = 0;
        } else {
            stall_count += 1;
            if stall_count > 5 && acts_cache.is_empty() {
                println!("[frame {}] !! stall: no actions and not dirty, phase={:?}", frames, gs.current_phase);
                break;
            }
        }

        if gs.game_result != GameResult::Ongoing {
            break;
        }

        if script_step < script.len() && !acts_cache.is_empty() {
            let (label, mut idx) = script[script_step];
            if idx == 99 {
                idx = acts_cache.len().saturating_sub(1);
            }
            if idx < acts_cache.len() {
                let action = acts_cache[idx].clone();
                println!(
                    "[frame {}] PRESS [{}] {:?}  \"{}\"  <script:{}>",
                    frames, idx, action.action_type, action.description, label
                );
                let p = action.parameters.clone();
                let _ = turn::TurnEngine::execute_main_phase_action(
                    &mut gs,
                    &action.action_type,
                    p.as_ref().and_then(|x| x.card_id),
                    p.as_ref().and_then(|x| x.card_indices.clone()),
                    p.as_ref().and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
                    p.as_ref().and_then(|x| x.use_baton_touch),
                );
                gs.reset_loop_detection();
                script_step += 1;
                dirty = true;
            } else {
                script_step += 1;
            }
        } else if script_step >= script.len() && !acts_cache.is_empty() {
            if let Some((_, confirm)) = acts_cache.iter().enumerate().find(|(_, a)| a.action_type == ActionType::ConfirmLiveCardSet) {
                // println!("[frame {}] AUTO-CONFIRM-LIVE at {:?}", frames, gs.current_phase);
                let _ = turn::TurnEngine::execute_main_phase_action(
                    &mut gs,
                    &confirm.action_type,
                    None, None, None, None,
                );
                gs.reset_loop_detection();
                dirty = true;
            }
            else if let Some((_, pass)) = acts_cache.iter().enumerate().find(|(_, a)| a.action_type == ActionType::Pass) {
                // println!("[frame {}] AUTO-PASS at {:?}", frames, gs.current_phase);
                let _ = turn::TurnEngine::execute_main_phase_action(
                    &mut gs,
                    &pass.action_type,
                    None, None, None, None,
                );
                gs.reset_loop_detection();
                dirty = true;
            } else {
                let idx = 1.min(acts_cache.len() - 1);
                let action = acts_cache[idx].clone();
                // println!("[frame {}] AUTO-PICK [{}] {:?} \"{}\"", frames, idx, action.action_type, action.description);
                let p = action.parameters.clone();
                let _ = turn::TurnEngine::execute_main_phase_action(
                    &mut gs,
                    &action.action_type,
                    p.as_ref().and_then(|x| x.card_id),
                    p.as_ref().and_then(|x| x.card_indices.clone()),
                    p.as_ref().and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
                    p.as_ref().and_then(|x| x.use_baton_touch),
                );
                gs.reset_loop_detection();
                dirty = true;
            }
        }
    }
    
    println!("=== Final Frame: {} ===", frames);
    println!("Final Phase: {:?}", gs.current_phase);
    println!("Result: {:?}", gs.game_result);
}
