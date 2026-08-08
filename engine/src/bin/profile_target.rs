use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use std::sync::Arc;

fn parse_stage_area(s: &str) -> Option<MemberArea> {
    match s {
        "left" => Some(MemberArea::LeftSide),
        "center" => Some(MemberArea::Center),
        "right" => Some(MemberArea::RightSide),
        _ => None,
    }
}

fn run_game_to_completion(gs: &mut GameState, _trace: bool) -> u64 {
    let mut actions = 0u64;
    let mut last_turn = 0u8;
    let mut stuck = 0u32;
    let mut iterations = 0u32;

    for _ in 0..2000 {
        iterations += 1;
        TurnEngine::check_victory_condition(gs);
        if gs.game_result != GameResult::Ongoing {
            break;
        }
        if gs.turn_number == last_turn {
            stuck += 1;
            if stuck > 300 {
                break;
            }
        } else {
            stuck = 0;
            last_turn = gs.turn_number;
        }

        // If there's a pending choice, let the bot resolve it before auto-advancing.
        // (Otherwise a LiveSuccess ability that creates a choice during
        // LiveVictoryDetermination leaves it orphaned forever.)
        if game_setup::auto_advance_one(gs) {
            continue;
        }

        let action_list = game_setup::generate_possible_actions(gs);
        if action_list.is_empty() {
            TurnEngine::advance_phase(gs);
            continue;
        }

        use rand::seq::SliceRandom;
        let action = action_list.choose(&mut rand::thread_rng()).unwrap();

        let _ = TurnEngine::execute_main_phase_action(
            gs,
            &action.action_type,
            action.parameters.as_ref().and_then(|p| p.card_id),
            action
                .parameters
                .as_ref()
                .and_then(|p| p.card_indices.clone()),
            action
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref().and_then(parse_stage_area)),
            action.parameters.as_ref().and_then(|p| p.use_baton_touch),
        );
        actions += 1;
    }
    if gs.game_result != GameResult::Ongoing || stuck > 300 || iterations >= 2000 {
        let tag = if gs.game_result == GameResult::Draw {
            "loop"
        } else if stuck > 300 {
            "stuck"
        } else if iterations >= 2000 {
            "timeout"
        } else {
            "finished"
        };
        if tag != "finished" {
            eprintln!(
                "  [{}] turn={} phase={:?} p1_live={} p2_live={} p1_stage={:?} p2_stage={:?} p1_hand={} p2_hand={} iter={} act={}",
                tag,
                gs.turn_number,
                gs.current_phase,
                gs.player1.success_live_card_zone.cards.len(),
                gs.player2.success_live_card_zone.cards.len(),
                gs.player1.stage.stage,
                gs.player2.stage.stage,
                gs.player1.hand.cards.len(),
                gs.player2.hand.cards.len(),
                iterations,
                actions,
            );
        }
    }
    actions
}

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards =
        card_loader::CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards");
    let card_database = Arc::new(CardDatabase::load_or_create(cards));

    let deck_lists = deck_parser::DeckParser::parse_all_decks().expect("Failed to load decks");
    let deck = &deck_lists[0];
    let card_numbers = deck_parser::DeckParser::deck_list_to_card_numbers(deck);

    let (p1_template, p2_template) = game_setup::build_two_decks(&card_database, &card_numbers, &card_numbers)
        .expect("Failed to build decks");

    let mut total_actions = 0u64;
    let mut outcomes: std::collections::HashMap<String, u8> = std::collections::HashMap::default();
    let mut p1_first_count = 0u32;
    let num_games = 5000;
    for _ in 0..num_games {
        let mut p1_deck = p1_template.clone();
        let mut p2_deck = p2_template.clone();
        p1_deck.shuffle_main_deck();
        p1_deck.shuffle_energy_deck();
        p2_deck.shuffle_main_deck();
        p2_deck.shuffle_energy_deck();

        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
        let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        player1.set_main_deck(p1_deck.main_deck);
        player1.set_energy_deck(p1_deck.energy_deck);
        player2.set_main_deck(p2_deck.main_deck);
        player2.set_energy_deck(p2_deck.energy_deck);

        let mut gs = GameState::new(player1, player2, Arc::clone(&card_database));
        game_setup::setup_game(&mut gs);
        total_actions += run_game_to_completion(&mut gs, false);
        // Record who is first attacker at game end
        if gs.player1.is_first_attacker {
            p1_first_count += 1;
        }
        let p1_success = gs.player1.success_live_card_zone.cards.len();
        let p2_success = gs.player2.success_live_card_zone.cards.len();
        let label = if p1_success >= 3 && p2_success <= 2 {
            "P1 wins".to_string()
        } else if p2_success >= 3 && p1_success <= 2 {
            "P2 wins".to_string()
        } else if p1_success >= 3 && p2_success >= 3 {
            "Draw (permanent loop)".to_string()
        } else if gs.game_result == GameResult::Draw {
            "Draw (permanent loop)".to_string()
        } else {
            "Draw (stuck)".to_string()
        };
        *outcomes.entry(label).or_insert(0) += 1;
    }

    eprintln!(
        "\nRan {} games, total actions: {}",
        num_games, total_actions
    );
    eprintln!(
        "P1 first attacker at end: {} / {} ({:.1}%)",
        p1_first_count,
        num_games,
        p1_first_count as f64 / num_games as f64 * 100.0
    );
    eprintln!("\n=== Game Outcomes ===");
    let mut sorted: Vec<_> = outcomes.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for (label, count) in &sorted {
        eprintln!(
            "  {:30} {:>5} ({:>4.1}%)",
            label,
            count,
            *count as f64 / num_games as f64 * 100.0
        );
    }
    if cfg!(feature = "profiling") {
        rabuka_engine::timer::print_results();
        rabuka_engine::timer::print_folded();
    }
}
