#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

// Debug output on bottom screen rows 22-23 — writes directly to tile map, bypassing console
fn dbg_row(row: i32, text: &str) {
    let mut s = alloc::string::String::from(text);
    // Pad/truncate to 32 chars
    while s.len() < 32 {
        s.push(' ');
    }
    s.truncate(32);
    s.push('\0');
    unsafe {
        nds_dbg_direct(row, s.as_ptr());
    }
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
    fn nds_get_tick() -> u64;
    fn nds_wait_vblank();
    fn nds_dbg_direct(row: i32, text: *const u8);
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

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    dbg_row(0, "STARTED                       ");
    dbg_row(1, "abcdefghijklmnopqrstuvwxyz01234");

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();

    let modes = ["VS AI", "2 Player", "AI vs AI", "Run Tests"];
    let mode_idx = select(&mut display, &mut input, &modes, "Mode");
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;
    let run_tests = mode_idx == 3;

    if run_tests {
        run_on_device_tests_ds(&mut display, &mut input);
        return;
    }

    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    display.println("Loading deck cards...");
    display.swap_buffers();
    let mut all_cards = load_two_decks(deck1_idx, deck2_idx);

    display.println("Attaching abilities...");
    display.swap_buffers();
    CardLoader::attach_abilities(&mut all_cards);

    display.println("Building database...");
    display.swap_buffers();
    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));

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
    display.println(&format!("decks built"));
    display.swap_buffers();

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);
    display.println(&format!("players ready"));
    display.swap_buffers();

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);
    display.println(&format!("game ready: {:?}", gs.current_phase));
    display.swap_buffers();

    loop {
        dbg_row(
            0,
            &alloc::format!(
                "TOP ph={:?} t={}           ",
                gs.current_phase,
                gs.turn_number
            ),
        );
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(&mut display, &mut input, &gs);
            break;
        }

        dbg_row(
            1,
            &alloc::format!(
                "TOP2 pc={} ph={:?}              ",
                gs.has_pending_choice(),
                gs.current_phase
            ),
        );

        settle_auto(&mut display, &mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(&mut display, &mut input, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        dbg_row(
            23,
            &alloc::format!(
                "a={} pc={}              ",
                actions.len(),
                gs.has_pending_choice()
            ),
        );

        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            wait_frames(10);
            continue;
        }

        if gs.has_pending_choice() {
            let is_current_player_ai =
                ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
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
        dbg_row(
            22,
            &alloc::format!(
                "ht={} ai={} ph={:?}             ",
                ok,
                is_current_player_ai,
                gs.current_phase
            ),
        );
        if !ok {
            if !is_current_player_ai {
                break;
            }
        }
        dbg_row(
            22,
            &alloc::format!(
                "sa_bot ph={:?} pc={}            ",
                gs.current_phase,
                gs.has_pending_choice()
            ),
        );
        settle_auto(&mut display, &mut gs);
        dbg_row(
            22,
            &alloc::format!(
                "sa_done ph={:?} pc={}           ",
                gs.current_phase,
                gs.has_pending_choice()
            ),
        );
    }
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

    // Test 1: Deck count from JSON
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

    // Test 2: Load cards from first two decks and verify abilities
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

    // Test 3: Verify ability count after attaching
    if ab_count > 0 {
        passed += 1;
    } else {
        failed += 1;
    }
    display_heap_stats(display);
    display.swap_buffers();
    wait_frames(20);

    // Test 4: AI vs AI if enough decks
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
    let action_type = &action.action_type;
    TurnEngine::execute_main_phase_action(
        gs,
        action_type,
        params.as_ref().and_then(|p| p.card_id),
        params.as_ref().and_then(|p| p.card_indices.clone()),
        params
            .as_ref()
            .and_then(|p| p.stage_area.as_ref().and_then(|s| s.parse().ok())),
        params.as_ref().and_then(|p| p.use_baton_touch),
    )
    .ok();
    gs.reset_loop_detection();
    true
}

fn human_turn(
    display: &mut Display,
    input: &mut Input,
    gs: &mut GameState,
    actions: &[game_setup::Action],
) -> bool {
    dbg_row(
        0,
        &alloc::format!("HT_IN phase={:?}              ", gs.current_phase),
    );
    let mut sel = 0usize;
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
        let end = actions.len().min(VISIBLE_ACTIONS);
        for i in 0..end {
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
        if game_setup::is_automatic_phase(gs) {
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
        nds_init();
        extern "C" {
            fn iprintf(fmt: *const u8, ...) -> i32;
        }
        let msg_buf;
        if let Some(msg) = info.message().as_str() {
            msg_buf = format!("PANIC: {}\0", msg);
        } else {
            msg_buf = format!("PANIC: {:?}\0", info.message());
        }
        iprintf(msg_buf.as_ptr());
        if let Some(loc) = info.location() {
            let loc_buf = format!(" at {}:{}\0", loc.file(), loc.line());
            iprintf(loc_buf.as_ptr());
        }
    }
    loop {
        unsafe { nds_wait_vblank() }
    }
}
