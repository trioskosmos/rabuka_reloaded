#![no_std]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};

extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
}

struct WiiAllocator;

unsafe impl Sync for WiiAllocator {}

unsafe impl GlobalAlloc for WiiAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if !ptr.is_null() {
            free(ptr);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, _old_layout: Layout, new_size: usize) -> *mut u8 {
        realloc(ptr, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: WiiAllocator = WiiAllocator;

pub mod display;
pub mod input;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use display::Display;
use input::{Button, Input};
use rabuka_engine::card::Card;
use rabuka_engine::game::deck_builder;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

const DECKS_JSON: &str = include_str!("../../psp/baked/decks.json");

const DECK_CARD_FILES: &[&str] = &[
    include_str!("../../psp/baked/deck_0_cards.json"),
    include_str!("../../psp/baked/deck_1_cards.json"),
    include_str!("../../psp/baked/deck_2_cards.json"),
    include_str!("../../psp/baked/deck_3_cards.json"),
    include_str!("../../psp/baked/deck_4_cards.json"),
    include_str!("../../psp/baked/deck_5_cards.json"),
    include_str!("../../psp/baked/deck_6_cards.json"),
    include_str!("../../psp/baked/deck_7_cards.json"),
    include_str!("../../psp/baked/deck_8_cards.json"),
    include_str!("../../psp/baked/deck_9_cards.json"),
    include_str!("../../psp/baked/deck_10_cards.json"),
    include_str!("../../psp/baked/deck_11_cards.json"),
    include_str!("../../psp/baked/deck_12_cards.json"),
    include_str!("../../psp/baked/deck_13_cards.json"),
    include_str!("../../psp/baked/deck_14_cards.json"),
    include_str!("../../psp/baked/deck_15_cards.json"),
];

#[no_mangle]
pub extern "C" fn rabuka_main() {
    run_game();
}

fn run_game() {
    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();

    let mode_idx = select(&mut display, &mut input, &deck_names, "Mode");
    let vs_ai = mode_idx == 0;

    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai {
        rng::rand_range(decks.len())
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
    };

    display.println("Loading...");
    display.swap_buffers();

    let cards1: Vec<Card> =
        serde_json::from_str(DECK_CARD_FILES[deck1_idx]).expect("Failed to parse deck cards");
    let cards2: Vec<Card> =
        serde_json::from_str(DECK_CARD_FILES[deck2_idx]).expect("Failed to parse deck cards");

    let mut card_map: hashbrown::HashMap<String, Card> = hashbrown::HashMap::new();
    for c in cards1.into_iter().chain(cards2.into_iter()) {
        let key = c.card_no.to_string();
        if !card_map.contains_key(&key) {
            card_map.insert(key, c);
        }
    }

    let mut cards: Vec<Card> = card_map.into_values().collect();
    rabuka_engine::card_loader::CardLoader::attach_abilities(&mut cards);
    let mut db = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));

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
            wait_vsync();
            continue;
        }

        if gs.has_pending_choice() {
            if !handle_choice(&mut display, &mut input, &mut gs) {
                break;
            }
            continue;
        }

        let is_ai = vs_ai && gs.active_player().id != gs.player1.id;
        let ok = if is_ai {
            ai_turn(&mut gs, &actions)
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
        wait_vsync();
        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(items.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::A) {
            return sel;
        }
    }
}

fn ai_turn(gs: &mut GameState, actions: &[game_setup::Action]) -> bool {
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
    const VISIBLE: usize = 12;
    loop {
        display.clear();
        display.println(&format!("Turn {} | {:?}", gs.turn_number, gs.current_phase));

        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        let tag = |a: bool| if a { ">>" } else { "  " };
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

        if sel < scroll_offset {
            scroll_offset = sel;
        }
        if sel >= scroll_offset + VISIBLE {
            scroll_offset = sel + 1 - VISIBLE;
        }

        let end = (scroll_offset + VISIBLE).min(actions.len());
        for i in scroll_offset..end {
            let p = if i == sel { " >" } else { "  " };
            let line = actions[i].description.lines().next().unwrap_or("");
            let card_tag = match &actions[i].parameters {
                Some(ref params) => params
                    .card_no
                    .as_ref()
                    .map(|no| format!(" [{}]", no))
                    .unwrap_or_default(),
                None => String::new(),
            };
            display.println(&format!("{p}[{i}] {line}{card_tag}"));
        }
        if actions.len() > end {
            display.println(&format!("  .. {} more", actions.len() - end));
        }
        display.swap_buffers();
        wait_vsync();

        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(actions.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::A) {
            return execute_action(gs, &actions[sel]);
        } else if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
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
        Err(_) => {
            gs.reset_loop_detection();
            true
        }
    }
}

fn menu_select(
    display: &mut Display,
    input: &mut Input,
    items: &[String],
    title: &str,
    allow_skip: bool,
) -> Option<usize> {
    let total = if allow_skip {
        items.len() + 1
    } else {
        items.len()
    };
    if total == 0 {
        return None;
    }
    let mut sel = 0usize;
    loop {
        display.clear();
        display.println(title);
        for (i, item) in items.iter().enumerate() {
            let prefix = if i == sel { " >" } else { "  " };
            display.println(&format!("{prefix} {item}"));
        }
        if allow_skip {
            let prefix = if sel == items.len() { " >" } else { "  " };
            display.println(&format!("{}  [Skip]", prefix));
        }
        display.swap_buffers();
        wait_vsync();
        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(total.saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::A) {
            if allow_skip && sel >= items.len() {
                return None;
            }
            return Some(sel);
        }
    }
}

fn handle_choice(display: &mut Display, input: &mut Input, gs: &mut GameState) -> bool {
    use rabuka_engine::ability::types::Choice;
    use rabuka_engine::ability::util::zone_cards;

    let choice = match gs.get_pending_choice() {
        Some(c) => c.clone(),
        None => return true,
    };

    match choice {
        Choice::SelectAutoAbility {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options
                .iter()
                .map(|o| format!("{}: {}", o.card_name, o.ability_text))
                .collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, Some(0), None).ok();
                return true;
            }
            let sel = menu_select(display, input, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            target_player_id,
            description,
            filtered_indices,
            ..
        } => {
            let player = target_player_id
                .as_ref()
                .and_then(|pid| {
                    if pid == &gs.player1.id {
                        Some(&gs.player1)
                    } else if pid == &gs.player2.id {
                        Some(&gs.player2)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| gs.active_player());
            let card_ids = zone_cards(player, &zone);
            if count <= 1 {
                let mut sel = 0usize;
                let total = if allow_skip {
                    card_ids.len() + 1
                } else {
                    card_ids.len()
                };
                if total == 0 {
                    return true;
                }
                loop {
                    display.clear();
                    display.println(&description);
                    for i in 0..card_ids.len() {
                        let prefix = if i == sel { " >" } else { "  " };
                        let cid = card_ids[i];
                        let name = card_name(&gs, cid);
                        display.println(&format!("{prefix} {name}"));
                    }
                    if allow_skip {
                        let prefix = if sel == card_ids.len() { " >" } else { "  " };
                        display.println(&format!("{}  [Skip]", prefix));
                    }
                    display.swap_buffers();
                    wait_vsync();
                    input.poll();
                    if input.just_pressed(Button::Down) {
                        sel = (sel + 1).min(total.saturating_sub(1));
                    } else if input.just_pressed(Button::Up) {
                        sel = sel.saturating_sub(1);
                    } else if input.just_pressed(Button::A) {
                        if allow_skip && sel >= card_ids.len() {
                            TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                        } else {
                            let actual = filtered_indices.as_ref().map(|fi| fi[sel]).unwrap_or(sel);
                            TurnEngine::resume_with_choice(gs, None, Some(vec![actual])).ok();
                        }
                        break;
                    }
                }
            } else {
                let mut multi_sel = 0usize;
                let mut selected: Vec<usize> = Vec::new();
                loop {
                    display.clear();
                    display.println(&description);
                    for i in 0..card_ids.len() {
                        let cid = card_ids[i];
                        let name = card_name(&gs, cid);
                        let check = if selected.contains(&i) { "[X]" } else { "[ ]" };
                        let ptr = if i == multi_sel { " >" } else { "  " };
                        display.println(&format!("{ptr}{check} {name}"));
                    }
                    display.println(&format!(
                        "Selected: {}/{}  (A=toggle, B=done)",
                        selected.len(),
                        count
                    ));
                    display.swap_buffers();
                    wait_vsync();
                    input.poll();
                    if input.just_pressed(Button::Down) {
                        multi_sel = (multi_sel + 1).min(card_ids.len().saturating_sub(1));
                    } else if input.just_pressed(Button::Up) {
                        multi_sel = multi_sel.saturating_sub(1);
                    } else if input.just_pressed(Button::A) && multi_sel < card_ids.len() {
                        if selected.contains(&multi_sel) {
                            selected.retain(|&x| x != multi_sel);
                        } else if selected.len() < count {
                            selected.push(multi_sel);
                        }
                    }
                    if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
                        break;
                    }
                }
                let actual: Vec<usize> = filtered_indices
                    .as_ref()
                    .map(|fi| selected.iter().map(|&i| fi[i]).collect())
                    .unwrap_or(selected);
                TurnEngine::resume_with_choice(gs, None, Some(actual)).ok();
            }
            true
        }
        Choice::SelectTarget {
            options,
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = match options {
                Some(ref opts) if !opts.is_empty() => opts.clone(),
                _ => (0..2).map(|i| format!("Option {}", i + 1)).collect(),
            };
            let sel = menu_select(display, input, &items, &description, allow_skip);
            match sel {
                None => TurnEngine::resume_with_choice(gs, Some(-1), None).ok(),
                Some(idx) => TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok(),
            };
            true
        }
        Choice::SelectPosition {
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = ["Left", "Center", "Right"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let sel = menu_select(display, input, &items, &description, allow_skip);
            match sel {
                None => TurnEngine::resume_with_choice(gs, Some(-1), None).ok(),
                Some(idx) => TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok(),
            };
            true
        }
        Choice::SelectHeartColor {
            options,
            description,
            ..
        } => {
            let sel = menu_select(display, input, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectHeartType {
            options,
            description,
            ..
        } => {
            let sel = menu_select(display, input, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectLiveSuccess {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options.iter().map(|o| o.card_name.clone()).collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                return true;
            }
            let sel = menu_select(display, input, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, None, Some(vec![sel])).ok();
            true
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
    loop {
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
        display.println("Press A to exit");
        display.swap_buffers();
        wait_vsync();
        input.poll();
        if input.just_pressed(Button::A) || input.just_pressed(Button::Start) {
            break;
        }
    }
}

fn init_rng() {
    let tick = get_system_tick();
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
}

fn card_name(gs: &GameState, cid: i16) -> String {
    if let Some(c) = gs.card_database.get_card(cid) {
        c.name.to_string()
    } else {
        format!("#{}", cid)
    }
}

extern "C" {
    fn SYS_Time() -> u64;
}

fn get_system_tick() -> u64 {
    unsafe { SYS_Time() }
}

fn wait_vsync() {
    unsafe {
        extern "C" {
            fn VIDEO_WaitVSync();
        }
        VIDEO_WaitVSync();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            extern "C" {
                fn VIDEO_WaitVSync();
            }
            VIDEO_WaitVSync();
        }
    }
}
