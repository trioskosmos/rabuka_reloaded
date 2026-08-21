use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, Phase};
use rabuka_engine::turn::TurnEngine;
use rand::Rng;
use std::sync::Arc;

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let mut card_database = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).unwrap();
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let (t1, t2) =
        game_setup::build_two_decks(&mut card_database, &card_numbers, &card_numbers).unwrap();

    let mut gs = rabuka_engine::bin_common::deal_game(
        &card_database,
        &t1,
        &t2,
        "p1",
        "P1",
        "p2",
        "P2",
    );

    let mut printed_turns = std::collections::HashSet::<u8>::new();
    for _t in 0..500 {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            println!("GAME OVER: {:?}", gs.game_result);
            break;
        }

        // Print state at start of each new turn
        if !printed_turns.contains(&gs.turn_number) {
            printed_turns.insert(gs.turn_number);
            let p1z = gs.player1.success_live_card_zone.cards.len();
            let p2z = gs.player2.success_live_card_zone.cards.len();
            let e = gs.player1.energy_zone.active_count();
            let h = gs.player1.hand.cards.len();
            let b: u8 = gs
                .player1
                .stage
                .stage
                .iter()
                .filter_map(|&id| {
                    if id >= 0 {
                        gs.card_database.get_card(id).map(|c| c.blade)
                    } else {
                        None
                    }
                })
                .sum();
            // Print hand card names
            let hand_names: Vec<String> = gs
                .player1
                .hand
                .cards
                .iter()
                .filter_map(|&id| {
                    gs.card_database
                        .get_card(id)
                        .map(|c| format!("{}", &c.name[..c.name.len().min(8)]))
                })
                .collect();
            // Print stage card names with heart colors
            let stage_info: Vec<String> = gs
                .player1
                .stage
                .stage
                .iter()
                .filter_map(|&id| {
                    if id < 0 {
                        return None;
                    }
                    gs.card_database.get_card(id).map(|c| {
                        let bh: String = c
                            .base_heart
                            .as_ref()
                            .map(|h| {
                                h.hearts
                                    .iter()
                                    .map(|(_, cnt)| format!("h{}", cnt))
                                    .collect::<Vec<_>>()
                                    .join("")
                            })
                            .unwrap_or_default();
                        format!("{}[:{}b{}]", &c.name[..c.name.len().min(6)], c.blade, bh)
                    })
                })
                .collect();

            println!(
                "T{}  E={}  H={}  B={}  S={}-{}",
                gs.turn_number, e, h, b, p1z, p2z
            );
            println!("  Hand: {:?}", hand_names);
            println!("  Stage: {:?}", stage_info);
            println!("  Phase: {:?}", gs.current_phase);
        }

        if game_setup::auto_advance_one(&mut gs) {
            continue;
        }

        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            continue;
        }

        // For mulligan/live set: pick Confirm or Skip immediately (avoid toggle)
        let action = match gs.current_phase {
            Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => acts
                .iter()
                .find(|a| {
                    matches!(
                        a.action_type,
                        rabuka_engine::game_setup::ActionType::SkipMulligan
                    )
                })
                .cloned()
                .unwrap_or_else(|| acts[rand::thread_rng().gen_range(0..acts.len())].clone()),
            Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => acts
                .iter()
                .find(|a| {
                    matches!(
                        a.action_type,
                        rabuka_engine::game_setup::ActionType::ConfirmLiveCardSet
                    )
                })
                .cloned()
                .unwrap_or_else(|| acts[rand::thread_rng().gen_range(0..acts.len())].clone()),
            _ => acts[rand::thread_rng().gen_range(0..acts.len())].clone(),
        };

        println!(
            "  Action: {:?} card={:?}",
            action.action_type,
            action
                .parameters
                .as_ref()
                .and_then(|p| p.card_no.clone())
                .unwrap_or_default()
        );

        let _ = rabuka_engine::bin_common::execute_and_settle(&mut gs, &action);
    }

    println!(
        "Final: P1 success={} P2 success={}",
        gs.player1.success_live_card_zone.cards.len(),
        gs.player2.success_live_card_zone.cards.len()
    );
}