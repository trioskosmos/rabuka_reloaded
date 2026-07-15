use std::sync::Arc;
use rand::Rng;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let card_database = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).unwrap();
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let cn = card_numbers.clone();
    let mut t1 = deck_builder::DeckBuilder::build_deck_from_database(
        &mut Arc::clone(&card_database), cn,
    ).unwrap();
    let cn2 = card_numbers.clone();
    let mut t2 = deck_builder::DeckBuilder::build_deck_from_database(
        &mut Arc::clone(&card_database), cn2,
    ).unwrap();
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut t1, &mut Arc::clone(&card_database),
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut t2, &mut Arc::clone(&card_database),
    );

    let mut d1 = t1.clone(); d1.shuffle_main_deck(); d1.shuffle_energy_deck();
    let mut d2 = t2.clone(); d2.shuffle_main_deck(); d2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    let mut p2 = Player::new("p2".into(), "P2".into(), false);
    p1.set_main_deck(d1.main_deck); p1.set_energy_deck(d1.energy_deck);
    p2.set_main_deck(d2.main_deck); p2.set_energy_deck(d2.energy_deck);

    let mut gs = GameState::new(p1, p2, Arc::clone(&card_database));
    game_setup::setup_game(&mut gs);

    let mut printed_turns = std::collections::HashSet::new();
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
            let b: u32 = gs.player1.stage.stage.iter().filter_map(|&id| {
                if id >= 0 { gs.card_database.get_card(id).map(|c| c.blade) } else { None }
            }).sum();
            // Print hand card names
            let hand_names: Vec<String> = gs.player1.hand.cards.iter().filter_map(|&id| {
                gs.card_database.get_card(id).map(|c| format!("{}", &c.name[..c.name.len().min(8)]))
            }).collect();
            // Print stage card names with heart colors
            let stage_info: Vec<String> = gs.player1.stage.stage.iter().filter_map(|&id| {
                if id < 0 { return None; }
                gs.card_database.get_card(id).map(|c| {
                    let bh: String = c.base_heart.iter().map(|(_, cnt)| format!("h{}", cnt)).collect();
                    format!("{}[:{}b{}]", &c.name[..c.name.len().min(6)], c.blade, bh)
                })
            }).collect();

            println!("T{}  E={}  H={}  B={}  S={}-{}",
                gs.turn_number, e, h, b, p1z, p2z);
            println!("  Hand: {:?}", hand_names);
            println!("  Stage: {:?}", stage_info);
            println!("  Phase: {:?}", gs.current_phase);
        }

        if !gs.has_pending_choice() {
            match gs.current_phase {
                Phase::Active | Phase::Energy | Phase::Draw
                | Phase::FirstAttackerPerformance | Phase::SecondAttackerPerformance
                | Phase::LiveVictoryDetermination => {
                    TurnEngine::advance_phase(&mut gs);
                    continue;
                }
                _ => {}
            }
        }

        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            continue;
        }

        // For mulligan/live set: pick Confirm or Skip immediately (avoid toggle)
        let action = match gs.current_phase {
            Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => {
                acts.iter().find(|a| matches!(a.action_type, rabuka_engine::game_setup::ActionType::SkipMulligan))
                    .cloned().unwrap_or_else(|| acts[rand::thread_rng().gen_range(0..acts.len())].clone())
            }
            Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => {
                acts.iter().find(|a| matches!(a.action_type, rabuka_engine::game_setup::ActionType::ConfirmLiveCardSet))
                    .cloned().unwrap_or_else(|| acts[rand::thread_rng().gen_range(0..acts.len())].clone())
            }
            _ => acts[rand::thread_rng().gen_range(0..acts.len())].clone()
        };

        println!("  Action: {:?} card={:?}",
            action.action_type,
            action.parameters.as_ref().and_then(|p| p.card_no.clone()).unwrap_or_default());

        let params = action.parameters.clone();
        let _ = TurnEngine::execute_main_phase_action(
            &mut gs, &action.action_type,
            params.as_ref().and_then(|p| p.card_id),
            params.as_ref().and_then(|p| p.card_indices.clone()),
            params.as_ref().and_then(|p| p.stage_area.as_deref().and_then(|s| s.parse().ok())),
            params.as_ref().and_then(|p| p.use_baton_touch),
        );
        game_setup::settle_single_player_state(&mut gs);
    }

    println!("Final: P1 success={} P2 success={}",
        gs.player1.success_live_card_zone.cards.len(),
        gs.player2.success_live_card_zone.cards.len());
}
