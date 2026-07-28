#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
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
    unsafe fn realloc(&self, ptr: *mut u8, _old: Layout, new: usize) -> *mut u8 {
        realloc(ptr, new)
    }
}
#[global_allocator]
static ALLOCATOR: WiiAllocator = WiiAllocator;

mod display;
mod input;

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
use rabuka_engine::deck_parser::DECK_CARD_FILES;

#[no_mangle]
pub extern "C" fn rabuka_main() {
    run_game();
}

fn run_game() {
    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("parse decks");
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
    let mut cards: Vec<Card> = rabuka_engine::deck_parser::load_two_decks(deck1_idx, deck2_idx);
    rabuka_engine::card_loader::CardLoader::attach_abilities(&mut cards);
    let mut db = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));

    let nums1: Vec<String> = decks[deck1_idx].cards.clone();
    let nums2: Vec<String> = decks[deck2_idx].cards.clone();

    let mut pd1 =
        deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums1).expect("deck");
    let mut pd2 =
        deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums2).expect("deck");
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
        game_setup::settle_auto(&mut gs);
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
        game_setup::settle_auto(&mut gs);
    }
}

#[derive(serde::Deserialize)]
struct DeckEntry {
    name: String,
    cards: Vec<String>,
}

fn select(d: &mut Display, i: &mut Input, items: &[&str], title: &str) -> usize {
    let mut sel = 0;
    loop {
        d.draw_menu(items, sel, title);
        d.swap_buffers();
        wait_vsync();
        i.poll();
        if i.just_pressed(Button::Down) {
            sel = (sel + 1).min(items.len() - 1);
        } else if i.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if i.just_pressed(Button::A) {
            return sel;
        }
    }
}

fn ai_turn(gs: &mut GameState, acts: &[game_setup::Action]) -> bool {
    execute_action(gs, &acts[rng::rand_range(acts.len())])
}

fn human_turn(
    d: &mut Display,
    i: &mut Input,
    gs: &mut GameState,
    acts: &[game_setup::Action],
) -> bool {
    let mut sel = 0;
    let mut scroll = 0;
    const VIS: usize = 12;
    loop {
        d.clear();
        d.println(&format!("Turn {} | {:?}", gs.turn_number, gs.current_phase));
        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        let tag = |a: bool| if a { ">>" } else { "  " };
        d.println(&format!(
            "{} P1 h:{} e:{} dk:{}",
            tag(is_p1),
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len()
        ));
        d.println(&format!(
            "{} P2 h:{} e:{} dk:{}",
            tag(!is_p1),
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len()
        ));
        if sel < scroll {
            scroll = sel;
        }
        if sel >= scroll + VIS {
            scroll = sel + 1 - VIS;
        }
        let end = (scroll + VIS).min(acts.len());
        for a in scroll..end {
            let p = if a == sel { " >" } else { "  " };
            let line = acts[a].description.lines().next().unwrap_or("");
            let tag = match &acts[a].parameters {
                Some(p) => p
                    .card_no
                    .as_ref()
                    .map(|n| format!(" [{}]", n))
                    .unwrap_or_default(),
                None => String::new(),
            };
            d.println(&format!("{p}[{a}] {line}{tag}"));
        }
        if acts.len() > end {
            d.println(&format!("  .. {} more", acts.len() - end));
        }
        d.swap_buffers();
        wait_vsync();
        i.poll();
        if i.just_pressed(Button::Down) {
            sel = (sel + 1).min(acts.len() - 1);
        } else if i.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if i.just_pressed(Button::A) {
            return execute_action(gs, &acts[sel]);
        } else if i.just_pressed(Button::B) {
            return false;
        }
    }
}

fn execute_action(gs: &mut GameState, act: &game_setup::Action) -> bool {
    let result = game_setup::execute_action(gs, act);
    if let Err(ref e) = result {
        let _ = e;
    }
    true
}

fn menu_select(
    d: &mut Display,
    i: &mut Input,
    items: &[String],
    title: &str,
    skip: bool,
) -> Option<usize> {
    let total = if skip { items.len() + 1 } else { items.len() };
    if total == 0 {
        return None;
    }
    let mut sel = 0;
    loop {
        d.clear();
        d.println(title);
        for (n, item) in items.iter().enumerate() {
            d.println(&format!("{} {item}", if n == sel { " >" } else { "  " }));
        }
        if skip {
            d.println(&format!(
                "{}  [Skip]",
                if sel == items.len() { " >" } else { "  " }
            ));
        }
        d.swap_buffers();
        wait_vsync();
        i.poll();
        if i.just_pressed(Button::Down) {
            sel = (sel + 1).min(total - 1);
        } else if i.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if i.just_pressed(Button::A) {
            return if skip && sel >= items.len() {
                None
            } else {
                Some(sel)
            };
        }
    }
}

fn handle_choice(d: &mut Display, i: &mut Input, gs: &mut GameState) -> bool {
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
            let sel = menu_select(d, i, &items, &description, false).unwrap_or(0);
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
                let mut sel = 0;
                let total = if allow_skip {
                    card_ids.len() + 1
                } else {
                    card_ids.len()
                };
                if total == 0 {
                    return true;
                }
                loop {
                    d.clear();
                    d.println(&description);
                    for (n, &cid) in card_ids.iter().enumerate() {
                        d.println(&format!("{} #{}", if n == sel { " >" } else { "  " }, cid));
                    }
                    if allow_skip {
                        d.println(&format!(
                            "{}  [Skip]",
                            if sel == card_ids.len() { " >" } else { "  " }
                        ));
                    }
                    d.swap_buffers();
                    wait_vsync();
                    i.poll();
                    if i.just_pressed(Button::Down) {
                        sel = (sel + 1).min(total - 1);
                    } else if i.just_pressed(Button::Up) {
                        sel = sel.saturating_sub(1);
                    } else if i.just_pressed(Button::A) {
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
                let mut multi_sel = 0;
                let mut selected: Vec<usize> = Vec::new();
                loop {
                    d.clear();
                    d.println(&description);
                    for (n, &cid) in card_ids.iter().enumerate() {
                        let check = if selected.contains(&n) { "[X]" } else { "[ ]" };
                        let ptr = if n == multi_sel { " >" } else { "  " };
                        d.println(&format!("{ptr}{check} #{}", cid));
                    }
                    d.println(&format!(
                        "Selected: {}/{}  (A=toggle, B=done)",
                        selected.len(),
                        count
                    ));
                    d.swap_buffers();
                    wait_vsync();
                    i.poll();
                    if i.just_pressed(Button::Down) {
                        multi_sel = (multi_sel + 1).min(card_ids.len() - 1);
                    } else if i.just_pressed(Button::Up) {
                        multi_sel = multi_sel.saturating_sub(1);
                    } else if i.just_pressed(Button::A) && multi_sel < card_ids.len() {
                        if selected.contains(&multi_sel) {
                            selected.retain(|&x| x != multi_sel);
                        } else if selected.len() < count {
                            selected.push(multi_sel);
                        }
                    }
                    if i.just_pressed(Button::B) {
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
                Some(ref o) if !o.is_empty() => o.clone(),
                _ => (0..2).map(|i| format!("Option {}", i + 1)).collect(),
            };
            match menu_select(d, i, &items, &description, allow_skip) {
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
            match menu_select(d, i, &items, &description, allow_skip) {
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
            let sel = menu_select(d, i, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectHeartType {
            options,
            description,
            ..
        } => {
            let sel = menu_select(d, i, &options, &description, false).unwrap_or(0);
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
            let sel = menu_select(d, i, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, None, Some(vec![sel])).ok();
            true
        }
    }
}

fn show_result(d: &mut Display, i: &mut Input, gs: &GameState) {
    loop {
        d.clear();
        d.println("=== GAME OVER ===");
        d.println(&format!("{:?}", gs.game_result));
        d.println(&format!(
            "P1 success:{} wait:{}",
            gs.player1.success_live_card_zone.cards.len(),
            gs.player1.waitroom.cards.len()
        ));
        d.println(&format!(
            "P2 success:{} wait:{}",
            gs.player2.success_live_card_zone.cards.len(),
            gs.player2.waitroom.cards.len()
        ));
        d.println("Press A to exit");
        d.swap_buffers();
        wait_vsync();
        i.poll();
        if i.just_pressed(Button::A) {
            break;
        }
    }
}

fn init_rng() {
    let tick = get_system_tick();
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
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
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        wait_vsync();
    }
}
