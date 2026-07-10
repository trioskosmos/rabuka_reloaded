use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
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

fn run_game_to_completion(gs: &mut GameState) -> u64 {
    let mut actions = 0u64;
    let mut last_turn = 0u32;
    let mut stuck = 0u32;

    for _ in 0..2000 {
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

        match gs.current_phase {
            Phase::Active
            | Phase::Energy
            | Phase::Draw
            | Phase::FirstAttackerPerformance
            | Phase::SecondAttackerPerformance
            | Phase::LiveVictoryDetermination => {
                TurnEngine::advance_phase(gs);
                continue;
            }
            _ => {}
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

    let mut p1_template = deck_builder::DeckBuilder::build_deck_from_database(
        &mut Arc::clone(&card_database),
        card_numbers.clone(),
    )
    .expect("Failed to build P1 deck");
    let mut p2_template = deck_builder::DeckBuilder::build_deck_from_database(
        &mut Arc::clone(&card_database),
        card_numbers,
    )
    .expect("Failed to build P2 deck");
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut p1_template,
        &mut Arc::clone(&card_database),
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut p2_template,
        &mut Arc::clone(&card_database),
    );

    let mut total_actions = 0u64;
    let num_games = 50;
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
        total_actions += run_game_to_completion(&mut gs);
    }

    eprintln!("Ran {} games, total actions: {}", num_games, total_actions);
    rabuka_engine::timer::print_results();
}
