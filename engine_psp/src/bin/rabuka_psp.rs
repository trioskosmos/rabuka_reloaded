#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use psp::sys::*;

use rabuka_psp::display::Display;
use rabuka_psp::input::{Button, Input};

static RNG_STATE: AtomicU64 = AtomicU64::new(0);

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_builder::DeckBuilder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;

psp::module!("rabuka", 1, 0);

const CARDS_BIN: &[u8] = include_bytes!("../../baked/cards.bin");
const DECKS_BIN: &[u8] = include_bytes!("../../baked/decks.bin");

fn psp_main() {
    psp::enable_home_button();

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    display.println("Loading cards...");
    display.swap_buffers();

    let cards =
        rmp_serde::from_slice::<serde_json::Value>(CARDS_BIN).expect("Failed to deserialize cards");
    let mut db = CardDatabase::load_or_create(
        CardLoader::load_cards_from_value(&cards).expect("Failed to load cards"),
    );
    let db = alloc::sync::Arc::new(db);

    let decks_map: hashbrown::HashMap<String, String> =
        rmp_serde::from_slice(DECKS_BIN).expect("Failed to deserialize decks");
    let deck_names: Vec<&str> = decks_map.keys().map(|s| s.as_str()).collect();

    let deck1_index = select_deck(
        &mut display,
        &mut input,
        &deck_names,
        "Select Player 1 Deck",
    );
    let deck2_index = select_deck(
        &mut display,
        &mut input,
        &deck_names,
        "Select Player 2 Deck",
    );

    let deck1_content = &decks_map[deck_names[deck1_index]];
    let deck2_content = &decks_map[deck_names[deck2_index]];

    display.clear();
    display.println(&format!("P1: {}", deck_names[deck1_index]));
    display.println(&format!("P2: {}", deck_names[deck2_index]));
    display.println("Starting game...");
    display.swap_buffers();

    let deck_lists = DeckParser::parse_all_decks_from_strs(&[
        (deck_names[deck1_index], deck1_content),
        (deck_names[deck2_index], deck2_content),
    ])
    .expect("Failed to parse decks");

    run_game(&mut display, &mut input, &mut db.clone(), &deck_lists);
}

fn select_deck(display: &mut Display, input: &mut Input, decks: &[&str], title: &str) -> usize {
    let mut selected = 0usize;

    loop {
        display.draw_menu(decks, selected, title);
        display.swap_buffers();
        wait_frames(2);

        input.poll();
        if input.just_pressed(Button::Down) {
            selected = (selected + 1).min(decks.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            selected = selected.saturating_sub(1);
        } else if input.just_pressed(Button::Cross) {
            return selected;
        }
    }
}

fn run_game(
    display: &mut Display,
    input: &mut Input,
    db: &mut alloc::sync::Arc<CardDatabase>,
    deck_lists: &[rabuka_engine::deck_parser::DeckList],
) {
    let d1 = deck_lists[0].clone();
    let d2 = if deck_lists.len() > 1 {
        deck_lists[1].clone()
    } else {
        d1.clone()
    };

    let nums1 = DeckParser::deck_list_to_card_numbers(&d1);
    let nums2 = DeckParser::deck_list_to_card_numbers(&d2);

    let mut pd1 = DeckBuilder::build_deck_from_database(db, nums1).expect("Failed to build deck1");
    let mut pd2 = DeckBuilder::build_deck_from_database(db, nums2).expect("Failed to build deck2");

    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, db).ok();
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, db).ok();

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);

    let mut gs = GameState::new(p1, p2, db.clone());
    game_setup::setup_game(&mut gs);

    loop {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(display, input, &gs);
            break;
        }

        settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(display, input, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            display.clear();
            display.println("No actions available. Advancing...");
            display.swap_buffers();
            TurnEngine::advance_phase(&mut gs);
            wait_frames(30);
            continue;
        }

        if gs.has_pending_choice() {
            handle_auto_choice(&mut gs);
            continue;
        }

        if !choose_action(display, input, &mut gs, &actions) {
            break;
        }

        settle_auto(&mut gs);
    }
}

fn choose_action(
    display: &mut Display,
    input: &mut Input,
    gs: &mut GameState,
    actions: &[game_setup::Action],
) -> bool {
    let mut selected = 0usize;

    loop {
        display.clear();
        display.println(&format!(
            "Turn {} | Phase: {:?}",
            gs.turn_number, gs.current_phase
        ));
        display.println("");

        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let ap = gs.active_player();
        let p1_active = ap.id == "p1";

        let p1_tag = if p1_active { ">>" } else { "  " };
        let p2_tag = if !p1_active { ">>" } else { "  " };
        display.println(&format!(
            "{p1_tag} P1 hand:{} energy:{} deck:{} wait:{}",
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len(),
            p1.waitroom.cards.len(),
        ));
        display.println(&format!(
            "{p2_tag} P2 hand:{} energy:{} deck:{} wait:{}",
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len(),
            p2.waitroom.cards.len(),
        ));
        display.println("");

        use rabuka_psp::display::{COLS, ROWS};
        let desc_count = actions.len().min(ROWS as usize - 6);
        for i in 0..desc_count {
            let prefix = if i == selected { " >" } else { "  " };
            let first_line = actions[i].description.lines().next().unwrap_or("");
            let max_w = (COLS - 4) as usize;
            let truncated = if first_line.len() > max_w {
                &first_line[..max_w]
            } else {
                first_line
            };
            display.println(&format!("{prefix} [{i}] {truncated}"));
        }

        if actions.len() > desc_count {
            display.println(&format!("  ... and {} more", actions.len() - desc_count));
        }

        display.swap_buffers();
        wait_frames(2);

        input.poll();
        if input.just_pressed(Button::Down) {
            selected = (selected + 1).min(actions.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            selected = selected.saturating_sub(1);
        } else if input.just_pressed(Button::Cross) {
            let action = &actions[selected];
            let params = action.parameters.clone();
            let result = TurnEngine::execute_main_phase_action(
                gs,
                &action.action_type,
                params.as_ref().and_then(|p| p.card_id),
                params.as_ref().and_then(|p| p.card_indices.clone()),
                params
                    .as_ref()
                    .and_then(|p| p.stage_area.as_ref().and_then(|s| s.parse().ok())),
                params.as_ref().and_then(|p| p.use_baton_touch),
            );
            match result {
                Ok(_) => {
                    gs.reset_loop_detection();
                    return true;
                }
                Err(e) => {
                    display.println(&format!("Error: {e}"));
                    display.swap_buffers();
                    wait_frames(60);
                }
            }
        } else if input.just_pressed(Button::Circle) || input.just_pressed(Button::Start) {
            return false;
        }
    }
}

fn handle_auto_choice(gs: &mut GameState) {
    use rabuka_engine::ability::types::Choice;
    if let Some(choice) = gs.get_pending_choice() {
        match choice {
            Choice::SelectAutoAbility { .. } => {
                TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
            }
            Choice::SelectCard {
                count: 0,
                allow_skip: true,
                ..
            } => {
                TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
            }
            _ => {}
        }
    }
}

fn settle_auto(gs: &mut GameState) {
    for _ in 0..500 {
        if gs.has_pending_choice() {
            break;
        }
        if gs.game_result != GameResult::Ongoing {
            break;
        }
        if game_setup::is_automatic_phase(gs)
            || matches!(
                gs.current_phase,
                Phase::RockPaperScissors | Phase::ChooseFirstAttacker
            )
        {
            TurnEngine::advance_phase(gs);
        } else {
            break;
        }
    }
}

fn show_result(display: &mut Display, input: &mut Input, gs: &GameState) {
    display.clear();
    display.println("=== GAME OVER ===");
    display.println(&format!("Result: {:?}", gs.game_result));
    display.println("");
    display.println(&format!(
        "P1 success: {} wait: {}",
        gs.player1.success_live_card_zone.cards.len(),
        gs.player1.waitroom.cards.len(),
    ));
    display.println(&format!(
        "P2 success: {} wait: {}",
        gs.player2.success_live_card_zone.cards.len(),
        gs.player2.waitroom.cards.len(),
    ));
    display.println("");
    display.println("Press X to exit");
    display.swap_buffers();

    loop {
        input.poll();
        if input.just_pressed(Button::Cross) || input.just_pressed(Button::Start) {
            break;
        }
        wait_frames(2);
    }
}

fn init_rng() {
    let mut tick: u64 = 0;
    unsafe {
        sceRtcGetCurrentTick(&mut tick);
    }
    let seed = if tick == 0 { 1 } else { tick };
    RNG_STATE.store(seed, Ordering::Relaxed);
    // Patch the engine's RNG to use our seed
    rabuka_engine::rng::set_seed(seed);
}

fn rand_u32() -> u32 {
    let mut s = RNG_STATE.load(Ordering::Relaxed);
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    RNG_STATE.store(s, Ordering::Relaxed);
    s as u32
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe {
            sceKernelDelayThread(16_667);
        }
    }
}
