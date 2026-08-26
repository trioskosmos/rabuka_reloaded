//! Fidelity-check player: plays real games and prints, for every staged /
//! activated / resolved card, its PRINTED ability text next to what the
//! engine actually did (rule-log lines + prompts). Purpose: spot divergences
//! between card text and engine behavior, then pin correct behavior in tests.
//!
//! Run from engine/:
//!   cargo run --release --example play_and_observe -- [games] [deck.txt]

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use std::collections::HashMap;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n_games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let deck_path = std::path::PathBuf::from(
        args.get(2)
            .cloned()
            .unwrap_or_else(|| "../web_ui/decks/5ZNN5 sakkakubibi.txt".to_string()),
    );

    let raw_cards = std::fs::read_to_string("../cards/cards.json").expect("cards.json");
    let cards_json: serde_json::Value =
        serde_json::from_str(&raw_cards).expect("parse cards.json");
    // card_no -> printed ability text (for fidelity echo)
    let mut printed: HashMap<String, String> = HashMap::new();
    if let Some(obj) = cards_json.as_object() {
        for (no, c) in obj {
            if let Some(ab) = c.get("ability").and_then(|v| v.as_str()) {
                // strip {{icon}} tags for readability
                let mut t = ab.to_string();
                while let Some(s) = t.find("{{") {
                    if let Some(e) = t[s..].find("}}") {
                        t.replace_range(s..s + e + 2, "");
                    } else {
                        break;
                    }
                }
                printed.insert(no.clone(), t);
            }
        }
    }

    let cards = card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
        "../cards/cards.json",
    ))
    .expect("cards.json");
    let mut db = Arc::new(CardDatabase::load_or_create(cards));

    for game_no in 1..=n_games {
        println!("\n=========== GAME {game_no} ({}) ============",
            deck_path.file_name().and_then(|s| s.to_str()).unwrap_or("?"));
        let mut gs = build_game(&mut db, &deck_path);
        let mut rng: u64 = 0xC0FFEE ^ (game_no as u64).wrapping_mul(0x9E37_79B9);
        let mut seen = 0usize;

        for _step in 0..4000u32 {
            rabuka_engine::turn::TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                flush_log(&gs, &mut seen);
                println!("== GAME OVER: {:?}", gs.game_result);
                break;
            }

            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                rabuka_engine::turn::TurnEngine::advance_phase(&mut gs);
                continue;
            }

            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let action = &actions[(rng as usize) % actions.len()];

            // Echo the PRINTED text of any member being played / activated.
            if let Some(p) = &action.parameters {
                if let Some(cid) = p.card_id {
                    if let Some(card) = gs.card_database.get_card(cid) {
                        let no = card.card_no.as_ref();
                        if let Some(txt) = printed.get(no) {
                            if !txt.trim().is_empty() && action.action_type != rabuka_engine::game_setup::ActionType::Pass {
                                println!("--- {} [{}] prints:\n{}", card.name, no, txt.trim());
                            }
                        }
                    }
                }
            }
            let ctype = format!("{:?}", action.action_type);
            println!(">> T{} {:?} {}", gs.turn_number, gs.current_phase, ctype);

            let _ = rabuka_engine::bin_common::execute_and_settle(&mut gs, action);
            flush_log(&gs, &mut seen);

            // Any pending prompt after settling: show its description.
            if let Some(json) = gs.get_pending_choice_json() {
                if let Some(desc) = json.get("description").and_then(|d| d.as_str()) {
                    println!("   ?PROMPT {desc}");
                }
            }
        }
    }
}

fn build_game(db: &mut Arc<CardDatabase>, deck_path: &std::path::Path) -> GameState {
    let deck = DeckParser::parse_deck_file(deck_path).expect("parse deck");
    let numbers = DeckParser::deck_list_to_card_numbers(&deck);
    let (t1, t2) =
        game_setup::build_two_decks(db, &numbers, &numbers).expect("build two decks");
    rabuka_engine::bin_common::deal_game(&mut db.clone(), &t1, &t2, "p1", "P1", "p2", "P2")
}

fn flush_log(gs: &GameState, seen: &mut usize) {
    while *seen < gs.rule_log.len() {
        println!("    LOG {}", gs.rule_log[*seen]);
        *seen += 1;
    }
}
