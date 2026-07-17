#![no_std]
#![no_main]
#![allow(linker_messages)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::hash::{BuildHasher, Hasher};

use psp::sys::*;

use rabuka_psp::display::Display;
use rabuka_psp::input::{Button, Input};

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::game::deck_builder;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

#[derive(Default, Clone, Copy)]
struct PspHasher(u64);

impl Hasher for PspHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.wrapping_mul(131).wrapping_add(b as u64);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

impl BuildHasher for PspHasher {
    type Hasher = PspHasher;
    fn build_hasher(&self) -> PspHasher {
        PspHasher(0)
    }
}

psp::module!("rabuka", 1, 0);

macro_rules! deck_cards {
    ($n:literal) => {
        include_str!(concat!("../../baked/deck_", $n, "_cards.json"))
    };
}

const DECK_CARDS: &[&str] = &[
    deck_cards!("0"),
    deck_cards!("1"),
    deck_cards!("2"),
    deck_cards!("3"),
    deck_cards!("4"),
    deck_cards!("5"),
    deck_cards!("6"),
    deck_cards!("7"),
    deck_cards!("8"),
    deck_cards!("9"),
    deck_cards!("10"),
    deck_cards!("11"),
    deck_cards!("12"),
    deck_cards!("13"),
    deck_cards!("14"),
    deck_cards!("15"),
];
const DECKS_JSON: &str = include_str!("../../baked/decks.json");

/// Truncate string to at most `max_chars` characters (not bytes).
/// Avoids panicking on multi-byte UTF-8 like Japanese text.
fn truncate_chars(s: &str, max_chars: usize) -> &str {
    let mut char_count = 0;
    for (i, _) in s.char_indices() {
        if char_count >= max_chars {
            return &s[..i];
        }
        char_count += 1;
    }
    s
}

fn psp_main() {
    psp::enable_home_button();

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    display.println("Loading...");
    display.swap_buffers();

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();

    // Mode select
    let modes = ["VS AI", "2 Player"];
    let mode_idx = select(&mut display, &mut input, &modes, "Mode");
    let vs_ai = mode_idx == 0;

    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai {
        rng::rand_range(decks.len())
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    display.println("Parsing deck 1 cards...");
    display.swap_buffers();
    let cards1: Vec<Card> =
        serde_json::from_str(DECK_CARDS[deck1_idx]).expect("Failed to parse deck cards");

    display.println("Parsing deck 2 cards...");
    display.swap_buffers();
    let cards2: Vec<Card> =
        serde_json::from_str(DECK_CARDS[deck2_idx]).expect("Failed to parse deck cards");

    // Merge and deduplicate by card_no, converting baked_abilities to abilities
    display.println("Merging cards...");
    display.swap_buffers();
    let mut card_map: hashbrown::HashMap<String, Card, PspHasher> =
        hashbrown::HashMap::with_hasher(PspHasher(0));
    for c in cards1.into_iter().chain(cards2.into_iter()) {
        let key = c.card_no.to_string();
        if !card_map.contains_key(&key) {
            let mut card = c;
            if let Some(baked) = card.baked_abilities.take() {
                card.abilities = baked
                    .into_iter()
                    .map(|a| alloc::sync::Arc::new(a))
                    .collect();
            }
            card_map.insert(key, card);
        }
    }

    display.println("Building database...");
    display.swap_buffers();
    let cards: Vec<Card> = card_map.into_values().collect();
    let mut db = Arc::new(CardDatabase::load_or_create(cards));

    // Build decks using DeckBuilder
    display.println("Building decks...");
    display.swap_buffers();
    let nums1: Vec<String> = decks[deck1_idx].cards.clone();
    let nums2: Vec<String> = decks[deck2_idx].cards.clone();

    let mut pd1 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums1)
        .expect("Failed to build P1 deck");
    let mut pd2 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums2)
        .expect("Failed to build P2 deck");
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    loop {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(&mut display, &mut input, &gs);
            break;
        }

        settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(&mut display, &mut input, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            wait_frames(10);
            continue;
        }

        if gs.has_pending_choice() {
            handle_auto_choice(&mut gs);
            continue;
        }

        let is_ai = vs_ai && gs.active_player().id != gs.player1.id;
        let ok = if is_ai {
            ai_turn(&mut display, &mut gs, &actions)
        } else {
            human_turn(&mut display, &mut input, &mut gs, &actions)
        };
        if !ok {
            break;
        }
        settle_auto(&mut gs);
    }
}

#[derive(serde::Deserialize)]
struct DeckEntry {
    name: String,
    cards: Vec<String>,
}

fn select(display: &mut Display, input: &mut Input, items: &[&str], title: &str) -> usize {
    let mut sel = 0usize;
    loop {
        display.draw_menu(items, sel, title);
        display.swap_buffers();
        wait_frames(2);
        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(items.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::Cross) {
            return sel;
        }
    }
}

fn ai_turn(_display: &mut Display, gs: &mut GameState, actions: &[game_setup::Action]) -> bool {
    let idx = rng::rand_range(actions.len());
    execute_action(gs, &actions[idx])
}

fn human_turn(
    display: &mut Display,
    input: &mut Input,
    gs: &mut GameState,
    actions: &[game_setup::Action],
) -> bool {
    let mut sel = 0usize;
    let mut scroll_offset = 0usize;
    const VISIBLE_ACTIONS: usize = 10;
    loop {
        display.clear();
        display.println(&format!("Turn {} | {:?}", gs.turn_number, gs.current_phase));

        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        let tag = |active: bool| if active { ">>" } else { "  " };
        display.println(&format!(
            "{} P1 h:{} e:{} dk:{}",
            tag(is_p1),
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len()
        ));
        display.println(&format!(
            "{} P2 h:{} e:{} dk:{}",
            tag(!is_p1),
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len()
        ));
        display.println("");

        // Clamp scroll offset so selection stays visible
        if sel < scroll_offset {
            scroll_offset = sel;
        }
        if sel >= scroll_offset + VISIBLE_ACTIONS {
            scroll_offset = sel + 1 - VISIBLE_ACTIONS;
        }

        let end = (scroll_offset + VISIBLE_ACTIONS).min(actions.len());
        for i in scroll_offset..end {
            let p = if i == sel { " >" } else { "  " };
            let line = actions[i].description.lines().next().unwrap_or("");
            let card_tag = match &actions[i].parameters {
                Some(params) => params
                    .card_no
                    .as_ref()
                    .map(|no| alloc::format!(" [{}]", no))
                    .unwrap_or_default(),
                None => alloc::string::String::new(),
            };
            let desc_max = 50usize.saturating_sub(card_tag.len());
            let truncated = truncate_chars(line, desc_max);
            display.println(&format!("{p}[{i}] {truncated}{card_tag}"));
        }
        if actions.len() > end {
            display.println(&format!("  .. {} more", actions.len() - end));
        }
        display.swap_buffers();
        wait_frames(2);

        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(actions.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::Cross) {
            return execute_action(gs, &actions[sel]);
        } else if input.just_pressed(Button::Circle) || input.just_pressed(Button::Start) {
            return false;
        }
    }
}

fn execute_action(gs: &mut GameState, action: &game_setup::Action) -> bool {
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
            true
        }
        Err(_e) => {
            gs.reset_loop_detection();
            true
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
        if gs.has_pending_choice() || gs.game_result != GameResult::Ongoing {
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
    display.println(&format!("{:?}", gs.game_result));
    display.println(&format!(
        "P1 success:{} wait:{}",
        gs.player1.success_live_card_zone.cards.len(),
        gs.player1.waitroom.cards.len()
    ));
    display.println(&format!(
        "P2 success:{} wait:{}",
        gs.player2.success_live_card_zone.cards.len(),
        gs.player2.waitroom.cards.len()
    ));
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
    rng::seed(if tick == 0 { 1 } else { tick });
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe {
            sceKernelDelayThread(16_667);
        }
    }
}
