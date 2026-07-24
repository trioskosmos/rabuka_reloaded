#![no_std]
#![no_main]

extern crate alloc;

use alloc::ffi::CString;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

macro_rules! top_debug {
    ($($arg:tt)*) => {
        {
            let s = alloc::format!($($arg)*);
            unsafe {
                if let Ok(c) = alloc::ffi::CString::new(&s[..]) {
                    nds_top_println(c.as_ptr() as *const u8);
                }
            }
        }
    };
}

use rabuka_ds::display::Display;
use rabuka_ds::input::{Button, Input};

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::deck_builder;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

extern "C" {
    fn nds_init();
    fn nds_top_init();
    fn nds_get_tick() -> u64;
    fn nds_wait_vblank();
    fn nds_dbg_direct(row: i32, text: *const u8);
    fn nds_top_println(text: *const u8);
    fn nds_top_clear();
}

use core::alloc::{GlobalAlloc, Layout};

extern "C" {
    fn ds_malloc(size: usize) -> *mut u8;
    fn ds_free(ptr: *mut u8);
    fn ds_realloc(ptr: *mut u8, size: usize) -> *mut u8;
}

struct DsAllocator {
    oom_count: core::cell::UnsafeCell<u32>,
}

unsafe impl Sync for DsAllocator {}

impl DsAllocator {
    const fn new() -> Self {
        DsAllocator {
            oom_count: core::cell::UnsafeCell::new(0),
        }
    }

    fn oom(&self) -> u32 {
        unsafe { *self.oom_count.get() }
    }
}

unsafe impl GlobalAlloc for DsAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = ds_malloc(layout.size());
        if ptr.is_null() {
            *self.oom_count.get() += 1;
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if !ptr.is_null() {
            ds_free(ptr);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, _old_layout: Layout, new_size: usize) -> *mut u8 {
        ds_realloc(ptr, new_size)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.alloc(layout);
        if !ptr.is_null() {
            core::ptr::write_bytes(ptr, 0, layout.size());
        }
        ptr
    }
}

#[global_allocator]
static ALLOCATOR: DsAllocator = DsAllocator::new();

const DECKS_JSON: &str = include_str!("../../../psp/baked/decks.json");

const DECK_CARD_FILES: &[&str] = &[
    include_str!("../../../psp/baked/deck_0_cards.json"),
    include_str!("../../../psp/baked/deck_1_cards.json"),
    include_str!("../../../psp/baked/deck_2_cards.json"),
    include_str!("../../../psp/baked/deck_3_cards.json"),
    include_str!("../../../psp/baked/deck_4_cards.json"),
    include_str!("../../../psp/baked/deck_5_cards.json"),
    include_str!("../../../psp/baked/deck_6_cards.json"),
    include_str!("../../../psp/baked/deck_7_cards.json"),
    include_str!("../../../psp/baked/deck_8_cards.json"),
    include_str!("../../../psp/baked/deck_9_cards.json"),
    include_str!("../../../psp/baked/deck_10_cards.json"),
    include_str!("../../../psp/baked/deck_11_cards.json"),
    include_str!("../../../psp/baked/deck_12_cards.json"),
    include_str!("../../../psp/baked/deck_13_cards.json"),
    include_str!("../../../psp/baked/deck_14_cards.json"),
    include_str!("../../../psp/baked/deck_15_cards.json"),
];

#[derive(serde::Deserialize)]
struct DeckEntry {
    name: String,
    cards: Vec<String>,
}

fn load_two_decks(deck1_idx: usize, deck2_idx: usize) -> Vec<Card> {
    let json1 = DECK_CARD_FILES[deck1_idx];
    let mut merged: Vec<Card> = serde_json::from_str(json1).unwrap_or_default();

    if deck1_idx != deck2_idx && deck2_idx < DECK_CARD_FILES.len() {
        let json2 = DECK_CARD_FILES[deck2_idx];
        let cards2: Vec<Card> = serde_json::from_str(json2).unwrap_or_default();
        for c in cards2 {
            if !merged.iter().any(|m| m.card_no == c.card_no) {
                merged.push(c);
            }
        }
    }

    merged
}

#[no_mangle]
pub extern "C" fn main() {
    unsafe {
        nds_init();
    }
    unsafe {
        nds_top_init();
    }
    unsafe {
        nds_top_clear();
    }

    top_debug!("=== Rabuka DS ===");
    top_debug!("Init...");

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    top_debug!("Parsing decks...");
    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();
    top_debug!("{} decks loaded", decks.len());

    let modes = ["VS AI", "2 Player", "AI vs AI", "Run Tests"];
    let mode_idx = select(&mut display, &mut input, &modes, "Mode");
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;
    let run_tests = mode_idx == 3;

    if run_tests {
        run_on_device_tests_ds(&mut display, &mut input);
        return;
    }

    top_debug!("Selecting decks...");
    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
    };
    top_debug!("Deck1={} Deck2={}", deck1_idx, deck2_idx);

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    top_debug!("Loading cards from JSON...");
    let mut all_cards = load_two_decks(deck1_idx, deck2_idx);
    top_debug!("{} unique cards loaded", all_cards.len());

    display.println("Attaching abilities...");
    display.swap_buffers();
    top_debug!("Attaching abilities...");
    CardLoader::attach_abilities(&mut all_cards);
    let ab_count = all_cards.iter().filter(|c| !c.abilities.is_empty()).count();
    top_debug!("{} cards w/ abilities", ab_count);

    display.println("Building database...");
    display.swap_buffers();
    top_debug!("Building card DB...");
    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));

    display.println("Building decks...");
    display.swap_buffers();
    top_debug!("Building player decks...");
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

    top_debug!("Creating players...");
    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);

    top_debug!("Creating GameState...");
    let mut gs = GameState::new(p1, p2, db);
    top_debug!("setup_game...");
    game_setup::setup_game(&mut gs);
    top_debug!("Game started! Phase={:?}", gs.current_phase);

    let mut loop_count = 0u32;
    loop {
        loop_count += 1;
        if loop_count % 10 == 0 {
            top_debug!(
                "=== Loop {} Phase={:?} oom={} ===",
                loop_count,
                gs.current_phase,
                ALLOCATOR.oom()
            );
        }

        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(&mut display, &mut input, &gs);
            break;
        }

        settle_auto(&mut display, &mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(&mut display, &mut input, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            top_debug!("No actions, advance phase from {:?}", gs.current_phase);
            TurnEngine::advance_phase(&mut gs);
            wait_frames(10);
            continue;
        }

        if gs.has_pending_choice() {
            let is_current_player_ai =
                ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
            top_debug!("Choice pending, ai={}", is_current_player_ai);
            if is_current_player_ai {
                TurnEngine::resume_with_choice(&mut gs, Some(0), None).ok();
            } else {
                if !handle_choice(&mut display, &mut input, &mut gs) {
                    break;
                }
            }
            continue;
        }

        let is_current_player_ai = ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
        let ok = if is_current_player_ai {
            ai_turn(&mut display, &mut gs, &actions)
        } else {
            human_turn(&mut display, &mut input, &mut gs, &actions)
        };
        if !ok {
            if !is_current_player_ai {
                break;
            }
        }
        settle_auto(&mut display, &mut gs);
    }

    top_debug!("Game ended. result={:?}", gs.game_result);
}

// ── DS On-Device Tests ──────────────────────────────────────────

fn run_on_device_tests_ds(display: &mut Display, input: &mut Input) {
    display.clear();
    display.println("=== DS ON-DEVICE TESTS ===");
    display.swap_buffers();
    wait_frames(20);

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let mut passed = 0u32;
    let mut failed = 0u32;

    let dc = decks.len();
    display.println(&alloc::format!("DECKS: {}", dc));
    if dc >= 2 {
        passed += 1;
    } else {
        failed += 1;
        display.println("(need 2+ for AI test)");
    }
    display.swap_buffers();
    wait_frames(20);

    let all_cards = load_two_decks(0, 1.min(dc.saturating_sub(1)));
    let cc = all_cards.len();
    let ab_count = all_cards.iter().filter(|c| !c.abilities.is_empty()).count();
    display.println(&alloc::format!(
        "CARDS: {}, with abilities: {}",
        cc,
        ab_count
    ));
    if cc > 0 {
        passed += 1;
    } else {
        failed += 1;
    }
    display.swap_buffers();
    wait_frames(20);

    if ab_count > 0 {
        passed += 1;
    } else {
        failed += 1;
    }
    display_heap_stats(display);
    display.swap_buffers();
    wait_frames(20);

    if dc >= 2 {
        display.println("AI PLAY: 5 turns...");
        display.swap_buffers();
        wait_frames(10);
        match test_ai_vs_ai_ds(0, 1) {
            Ok(n) => {
                display.println(&alloc::format!("AI PLAY: {} actions (OK)", n));
                passed += 1;
            }
            Err(e) => {
                display.println(&alloc::format!("AI PLAY: {}", e));
                failed += 1;
            }
        }
        display_heap_stats(display);
        display.swap_buffers();
        wait_frames(30);
    }

    display.println(&alloc::format!(
        "RESULTS: {} passed, {} failed",
        passed,
        failed
    ));
    display_heap_stats(display);
    display.println("START=exit");
    display.swap_buffers();
    loop {
        input.poll();
        if input.just_pressed(Button::Start) || input.just_pressed(Button::B) {
            break;
        }
        wait_frames(2);
    }
}

fn test_ai_vs_ai_ds(d1: usize, d2: usize) -> Result<usize, alloc::string::String> {
    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let mut all_cards = load_two_decks(d1, d2);
    CardLoader::attach_abilities(&mut all_cards);

    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));
    let nums1: Vec<String> = decks[d1].cards.clone();
    let nums2: Vec<String> = decks[d2].cards.clone();
    let mut pd1 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums1)
        .map_err(|e| alloc::format!("D1:{}", e))?;
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    let mut pd2 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums2)
        .map_err(|e| alloc::format!("D2:{}", e))?;
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "P2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);
    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    let mut count = 0usize;
    let mut turns = 0u32;
    while gs.game_result == GameResult::Ongoing && turns < 10 {
        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            turns += 1;
            continue;
        }
        let a = acts[0].clone();
        let p = a.parameters.clone();
        let _ = TurnEngine::execute_main_phase_action(
            &mut gs,
            &a.action_type,
            p.as_ref().and_then(|x| x.card_id),
            p.as_ref().and_then(|x| x.card_indices.clone()),
            p.as_ref()
                .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
            p.as_ref().and_then(|x| x.use_baton_touch),
        );
        count += 1;
        while gs.game_result == GameResult::Ongoing && game_setup::is_automatic_phase(&gs) {
            TurnEngine::advance_phase(&mut gs);
            turns += 1;
        }
        if gs.current_phase == Phase::Active || gs.current_phase == Phase::Draw {
            turns += 1;
        }
    }
    Ok(count)
}

// ── End of tests ─────────────────────────────────────────────────

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
        } else if input.just_pressed(Button::A) {
            return sel;
        }
    }
}

fn ai_turn(_display: &mut Display, gs: &mut GameState, actions: &[game_setup::Action]) -> bool {
    let idx = rng::rand_range(actions.len());
    execute_action(gs, &actions[idx])
}

fn execute_action(gs: &mut GameState, action: &game_setup::Action) -> bool {
    let params = action.parameters.clone();
    let desc_first_line = action.description.lines().next().unwrap_or("");
    top_debug!("ACT: {} {:?}", desc_first_line, action.action_type);
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
            top_debug!("ACT OK");
        }
        Err(ref e) => {
            top_debug!("ACT ERR: {}", e);
            dbg_row(22, &alloc::format!("act err: {}", e));
        }
    }
    gs.reset_loop_detection();
    true
}

fn human_turn(
    display: &mut Display,
    input: &mut Input,
    gs: &mut GameState,
    actions: &[game_setup::Action],
) -> bool {
    let mut sel = 0usize;
    let mut scroll_offset = 0usize;
    const VISIBLE_ACTIONS: usize = 9;
    loop {
        display.clear();
        let oom = ALLOCATOR.oom();
        display.println(&alloc::format!(
            "T{} {} oom:{}",
            gs.turn_number,
            format!("{:?}", gs.current_phase)
                .chars()
                .take(5)
                .collect::<String>(),
            oom
        ));
        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        display.println(&alloc::format!(
            "{}P1 h:{} e:{} dk:{}",
            if is_p1 { ">>" } else { "  " },
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len()
        ));
        display.println(&alloc::format!(
            "{}P2 h:{} e:{} dk:{}",
            if !is_p1 { ">>" } else { "  " },
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len()
        ));
        display.println("--------------------------------");

        if sel < scroll_offset {
            scroll_offset = sel;
        }
        if sel >= scroll_offset + VISIBLE_ACTIONS {
            scroll_offset = sel + 1 - VISIBLE_ACTIONS;
        }

        let end = (scroll_offset + VISIBLE_ACTIONS).min(actions.len());
        for i in scroll_offset..end {
            let p = if i == sel { ">" } else { " " };
            let line = actions[i].description.lines().next().unwrap_or("");
            let id = actions[i]
                .parameters
                .as_ref()
                .and_then(|p| p.card_no.as_deref())
                .unwrap_or("");
            display.println(&alloc::format!("{p}[{}] {} {}", i, id, line));
        }
        if actions.len() > end {
            display.println(&alloc::format!("  .. {} more", actions.len() - end));
        }
        display.println(&alloc::format!("oom:{} A=sel", oom));
        display.swap_buffers();
        wait_frames(2);
        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(actions.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::A) {
            execute_action(gs, &actions[sel]);
            return true;
        } else if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
            return false;
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
            let prefix = if i == sel { ">" } else { " " };
            display.println(&format!("{prefix} {item}"));
        }
        if allow_skip {
            let prefix = if sel == items.len() { ">" } else { " " };
            display.println(&format!("{}  [Skip]", prefix));
        }
        display.println("");
        display.println("DPAD:navigate A:select");
        display.swap_buffers();
        wait_frames(2);
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
            let player = target_player_id.as_ref().map_or_else(
                || gs.active_player(),
                |pid| {
                    if pid == &gs.player1.id {
                        &gs.player1
                    } else {
                        &gs.player2
                    }
                },
            );
            let card_ids = zone_cards(player, &zone);
            let items: Vec<String> = match filtered_indices {
                Some(ref indices) => indices
                    .iter()
                    .map(|&i| {
                        if i < card_ids.len() {
                            gs.card_database
                                .get_card(card_ids[i])
                                .map(|c| c.name.to_string())
                                .unwrap_or_else(|| format!("#{}", card_ids[i]))
                        } else {
                            format!("#{}", i)
                        }
                    })
                    .collect(),
                None => card_ids
                    .iter()
                    .map(|cid| {
                        gs.card_database
                            .get_card(*cid)
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| format!("#{}", cid))
                    })
                    .collect(),
            };
            if count <= 1 {
                let sel = menu_select(display, input, &items, &description, allow_skip);
                match sel {
                    None => TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok(),
                    Some(idx) => {
                        let actual_idx = filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx);
                        TurnEngine::resume_with_choice(gs, None, Some(vec![actual_idx])).ok()
                    }
                };
            } else {
                let mut selected: Vec<usize> = Vec::new();
                while selected.len() < count.min(items.len()) {
                    let display_items: Vec<String> = items
                        .iter()
                        .enumerate()
                        .map(|(i, name)| {
                            if selected.contains(&i) {
                                format!("[X] {}", name)
                            } else {
                                format!("[ ] {}", name)
                            }
                        })
                        .collect();
                    let sel = menu_select(display, input, &display_items, &description, allow_skip);
                    match sel {
                        None => break,
                        Some(idx) => {
                            if !selected.contains(&idx) {
                                selected.push(idx);
                            }
                        }
                    }
                }
                let actual_indices: Vec<usize> = match filtered_indices {
                    Some(ref fi) => selected.iter().map(|&i| fi[i]).collect(),
                    None => selected,
                };
                TurnEngine::resume_with_choice(gs, None, Some(actual_indices)).ok();
            }
            true
        }
        Choice::SelectTarget {
            target,
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
                None => match target.as_str() {
                    "choice" | "choice_string" | "conditional_optional" => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok()
                    }
                    _ => TurnEngine::resume_with_choice(gs, Some(-1), None).ok(),
                },
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

fn settle_auto(_display: &mut Display, gs: &mut GameState) {
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
    display.println("Press A to exit");
    display.swap_buffers();
    loop {
        input.poll();
        if input.just_pressed(Button::A) || input.just_pressed(Button::Start) {
            break;
        }
        wait_frames(2);
    }
}

fn display_heap_stats(display: &mut Display) {
    let oom = ALLOCATOR.oom();
    display.println(&alloc::format!("oom:{}", oom));
}

fn dbg_row(row: i32, text: &str) {
    let c_str = CString::new(text).unwrap_or_default();
    unsafe {
        nds_dbg_direct(row, c_str.as_ptr());
    }
}

fn init_rng() {
    let tick = unsafe { nds_get_tick() };
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe { nds_wait_vblank() }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        nds_top_clear();
    }
    let line1 = alloc::format!("PANIC: {}", info);
    let c1 = CString::new(&line1[..]).unwrap_or_default();
    unsafe {
        nds_top_println(c1.as_ptr() as *const u8);
    }
    let line2 = alloc::format!("OOM: {}", ALLOCATOR.oom());
    let c2 = CString::new(&line2[..]).unwrap_or_default();
    unsafe {
        nds_top_println(c2.as_ptr() as *const u8);
    }
    if let Some(loc) = info.location() {
        let line3 = alloc::format!("at {}:{}", loc.file(), loc.line());
        let c3 = CString::new(&line3[..]).unwrap_or_default();
        unsafe {
            nds_top_println(c3.as_ptr() as *const u8);
        }
    }
    loop {
        unsafe { nds_wait_vblank() }
    }
}
