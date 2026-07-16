#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use psp::sys::*;

use rabuka_psp::display::Display;
use rabuka_psp::input::{Button, Input};

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

psp::module!("rabuka", 1, 0);

const CARDS_JSON: &str = include_str!("../../baked/cards.json");
const DECKS_JSON: &str = include_str!("../../baked/decks.json");

fn psp_main() {
    psp::enable_home_button();

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    display.println("Loading cards...");
    display.swap_buffers();

    // cards.json is a JSON object mapping card_no -> Card
    let cards_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(CARDS_JSON).expect("Failed to parse card data");
    let mut cards: Vec<Card> = Vec::new();
    for (_key, value) in cards_map {
        if let Ok(card) = serde_json::from_value(value) {
            cards.push(card);
        }
    }
    let db = CardDatabase::load_or_create(cards);
    let db = alloc::sync::Arc::new(db);

    let decks: Vec<DeckEntry> =
        serde_json::from_str(DECKS_JSON).expect("Failed to parse deck data");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();

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

    display.clear();
    display.println(&format!("P1: {}", deck_names[deck1_index]));
    display.println(&format!("P2: {}", deck_names[deck2_index]));
    display.println("Starting game...");
    display.swap_buffers();

    run_game(
        &mut display,
        &mut input,
        &mut db.clone(),
        &decks,
        deck1_index,
        deck2_index,
    );
}

#[derive(serde::Deserialize)]
struct DeckEntry {
    name: String,
    cards: Vec<String>,
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
    decks: &[DeckEntry],
    deck1_idx: usize,
    deck2_idx: usize,
) {
    let d1 = &decks[deck1_idx];
    let d2 = &decks[deck2_idx];

    let nums1 = resolve_deck(d1, db);
    let nums2 = resolve_deck(d2, db);

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);

    for &card_id in &nums1 {
        p1.main_deck.cards.push(card_id);
    }
    for &card_id in &nums2 {
        p2.main_deck.cards.push(card_id);
    }

    p1.main_deck.shuffle();
    p2.main_deck.shuffle();

    add_basic_energy(&mut p1, db);
    add_basic_energy(&mut p2, db);
    rng::shuffle_slice(&mut p1.energy_deck.cards);
    rng::shuffle_slice(&mut p2.energy_deck.cards);

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

fn resolve_deck(deck: &DeckEntry, db: &CardDatabase) -> Vec<i16> {
    let mut ids = Vec::new();
    for card_no in &deck.cards {
        if let Some(id) = db.get_card_id(card_no) {
            ids.push(id);
        }
    }
    ids
}

fn add_basic_energy(player: &mut Player, db: &CardDatabase) {
    let mut energy_count = 0u32;
    for (&id, card) in &db.cards {
        if matches!(card.card_type, rabuka_engine::card::CardType::Energy) {
            for _ in 0..10 {
                if energy_count >= 30 {
                    return;
                }
                player.energy_deck.cards.push(id);
                energy_count += 1;
            }
        }
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
    rng::seed(seed);
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe {
            sceKernelDelayThread(16_667);
        }
    }
}
