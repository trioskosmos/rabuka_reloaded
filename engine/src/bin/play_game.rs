use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let db = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).unwrap();
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let (t1, t2) = game_setup::build_two_decks(&db, &card_numbers, &card_numbers).unwrap();

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

    let mut gs = GameState::new(p1, p2, Arc::clone(&db));
    game_setup::setup_game(&mut gs);

    let mut step = 0u32;
    let mut rng: u64 = 0xDEAD_BEEF;
    let mut last_phase = String::new();
    let mut phase_count = 0u32;
    for _ in 0..500 {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            println!("\n=== GAME OVER: {:?} ===", gs.game_result);
            println!(
                "P1 success zone: {}",
                gs.player1.success_live_card_zone.cards.len()
            );
            println!(
                "P2 success zone: {}",
                gs.player2.success_live_card_zone.cards.len()
            );
            break;
        }

        step += 1;
        let active = gs.active_player();
        let pid = active.id.clone();
        let phase = gs.current_phase.clone();
        let turn = gs.turn_number;

        let phase_key = format!("{pid}:{phase:?}");
        if phase_key == last_phase {
            phase_count += 1;
            if phase_count > 20 {
                println!("STUCK in {phase_key} after {phase_count} repeats. Stopping.");
                break;
            }
        } else {
            last_phase = phase_key;
            phase_count = 0;
        }

        if game_setup::auto_advance_one(&mut gs) {
            println!("  {pid} auto: {phase:?}");
            continue;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            continue;
        }

        let is_p1 = pid == "p1";
        println!(
            "\n--- Step {step}: T{turn} {pid} {phase:?} ({}) ---",
            actions.len()
        );

        // Print board state
        print_board(&gs, is_p1, &db);

        // Random action
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let idx = (rng as usize) % actions.len();
        let action = &actions[idx];

        let ctype = format!("{:?}", action.action_type);
        let card_name = action
            .parameters
            .as_ref()
            .and_then(|p| p.card_id)
            .and_then(|cid| db.get_card(cid))
            .map(|c| c.name.to_string())
            .unwrap_or_default();
        println!("  -> {ctype} {card_name}");

        let params = action.parameters.clone();
        let _ = TurnEngine::execute_main_phase_action(
            &mut gs,
            &action.action_type,
            params.as_ref().and_then(|p| p.card_id),
            params.as_ref().and_then(|p| p.card_indices.clone()),
            params
                .as_ref()
                .and_then(|p| p.stage_area.as_deref().and_then(|s| s.parse().ok())),
            params.as_ref().and_then(|p| p.use_baton_touch),
        );
        game_setup::settle_single_player_state(&mut gs);
    }
}

fn print_board(gs: &GameState, show_p1: bool, db: &CardDatabase) {
    for (label, player) in [("P1", &gs.player1), ("P2", &gs.player2)] {
        let show = (label == "P1") == show_p1;
        let tag = if show { ">>>" } else { "   " };

        let hand: Vec<String> = player
            .hand
            .cards
            .iter()
            .filter_map(|&id| {
                db.get_card(id).map(|c| {
                    let cost = c.cost.map_or(0, |v| v);
                    let blade = c.blade;
                    format!("{}[c{}b{}]", &c.name[..c.name.len().min(8)], cost, blade)
                })
            })
            .collect();

        let stage: Vec<String> = player
            .stage
            .stage
            .iter()
            .map(|&id| {
                if id < 0 {
                    return "[--]".into();
                }
                db.get_card(id)
                    .map(|c| {
                        let cost = c.cost.map_or(0, |v| v);
                        let blade = c.blade;
                        let heart_str = c
                            .base_heart
                            .as_ref()
                            .map(|h| {
                                h.hearts
                                    .iter()
                                    .map(|(k, v)| {
                                        format!(
                                            "{}{}",
                                            format!("{:?}", k).chars().last().unwrap(),
                                            v
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("")
                            })
                            .unwrap_or_default();
                        format!(
                            "{}[c{}b{}{}]",
                            &c.name[..c.name.len().min(8)],
                            cost,
                            blade,
                            heart_str
                        )
                    })
                    .unwrap_or_else(|| format!("[{}]", id))
            })
            .collect();

        let energy = player.energy_zone.active_count();
        let success = player.success_live_card_zone.cards.len();
        let live: Vec<String> = player
            .live_card_zone
            .cards
            .iter()
            .filter_map(|&id| {
                db.get_card(id).map(|c| {
                    let score = c.score.map_or(0, |v| v);
                    let nh = c
                        .need_heart
                        .as_ref()
                        .map(|h| {
                            h.hearts
                                .iter()
                                .map(|(k, v)| {
                                    format!("{}{}", format!("{:?}", k).chars().last().unwrap(), v)
                                })
                                .collect::<Vec<_>>()
                                .join("")
                        })
                        .unwrap_or_default();
                    format!("{}[s{} n{}]", &c.name[..c.name.len().min(8)], score, nh)
                })
            })
            .collect();

        println!(
            "{tag} {label} E={energy} S={success} Hand: [{}]",
            hand.join(", ")
        );
        println!("       Stage: [{}]", stage.join(", "));
        if !live.is_empty() {
            println!("       Live:  [{}]", live.join(", "));
        }
    }
}
