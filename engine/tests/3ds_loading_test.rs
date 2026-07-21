//! Desktop test that exactly mirrors the 3DS loading + first action.
//! Run: cargo test --test 3ds_loading_test -- --nocapture

use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_builder::DeckBuilder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;

#[allow(dead_code)]
fn build_game(json_str: &str) -> Result<GameState, String> {
    let cards = CardLoader::load_cards_from_strs(json_str)?;
    let mut db = Arc::new(CardDatabase::load_or_create(cards));
    let decks = DeckParser::parse_all_decks_from_directory(Path::new("../web_ui/decks/"))
        .map_err(|e| format!("decks: {}", e))?;
    let nums = DeckParser::deck_list_to_card_numbers(&decks[0]);
    let mut pd = DeckBuilder::build_deck_from_database(&mut db, nums)
        .map_err(|e| format!("build deck: {}", e))?;
    DeckBuilder::add_default_energy_cards_from_database(&mut pd, &mut db).ok();

    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    p1.set_main_deck(pd.main_deck.clone());
    p1.set_energy_deck(pd.energy_deck.clone());
    let mut p2 = Player::new("p2".into(), "P2".into(), false);
    p2.set_main_deck(pd.main_deck);
    p2.set_energy_deck(pd.energy_deck);

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);
    Ok(gs)
}

#[test]
#[cfg(not(feature = "bytecode_abilities"))]
fn load_and_play() {
    let json_str = std::fs::read_to_string("../cards/cards.json").expect("cards.json not found");
    eprintln!("cards: {} KB", json_str.len() / 1024,);

    let mut gs = build_game(&json_str).expect("build_game failed");
    eprintln!("phase: {}, result: {:?}", gs.current_phase, gs.game_result);
    assert_eq!(gs.game_result, GameResult::Ongoing);

    let acts = rabuka_engine::game_setup::generate_possible_actions(&gs);
    eprintln!("actions: {}", acts.len());
    assert!(
        acts.len() > 0,
        "should have at least one action at game start"
    );

    let a = &acts[0];
    let p = a.parameters.clone();
    rabuka_engine::turn::TurnEngine::execute_main_phase_action(
        &mut gs,
        &a.action_type,
        p.as_ref().and_then(|x| x.card_id),
        p.as_ref().and_then(|x| x.card_indices.clone()),
        p.as_ref()
            .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
        p.as_ref().and_then(|x| x.use_baton_touch),
    )
    .expect("execute action should succeed");
    gs.reset_loop_detection();
    eprintln!(
        "after action: phase: {}, result: {:?}",
        gs.current_phase, gs.game_result
    );
}
