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
use rabuka_engine::game::platform_ui;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

struct DsUi<'a> {
    display: &'a mut Display,
    input: &'a mut Input,
}

impl<'a> platform_ui::PlatformUi for DsUi<'a> {
    fn clear_screen(&mut self) {
        self.display.clear();
    }
    fn println(&mut self, text: &str) {
        self.display.println(text);
    }
    fn swap_buffers(&mut self) {
        self.display.swap_buffers();
    }
    fn poll_input(&mut self) {
        self.input.poll();
    }
    fn just_pressed_a(&self) -> bool {
        self.input.just_pressed(Button::A)
    }
    fn just_pressed_b(&self) -> bool {
        self.input.just_pressed(Button::B)
    }
    fn just_pressed_up(&self) -> bool {
        self.input.just_pressed(Button::Up)
    }
    fn just_pressed_down(&self) -> bool {
        self.input.just_pressed(Button::Down)
    }
    fn just_pressed_start(&self) -> bool {
        self.input.just_pressed(Button::Start)
    }
    fn wait_vblank(&mut self) {
        wait_frames(1);
    }
}

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

use rabuka_engine::deck_parser::DECK_CARD_FILES;

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
    let mut ui = DsUi {
        display: &mut display,
        input: &mut input,
    };
    let mode_idx = platform_ui::select(&mut ui, &modes, "Mode");
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;
    let run_tests = mode_idx == 3;

    if run_tests {
        run_on_device_tests_ds(&mut display, &mut input);
        return;
    }

    top_debug!("Selecting decks...");
    let deck1_idx = platform_ui::select(&mut ui, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    };
    top_debug!("Deck1={} Deck2={}", deck1_idx, deck2_idx);

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    top_debug!("Loading cards from JSON...");
    let mut all_cards = deck_parser::load_two_decks(deck1_idx, deck2_idx);
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
            let mut ui = DsUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = DsUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
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
                let mut ui = DsUi {
                    display: &mut display,
                    input: &mut input,
                };
                if !platform_ui::handle_choice(&mut ui, &mut gs) {
                    break;
                }
            }
            continue;
        }

        let is_current_player_ai = ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
        let ok = if is_current_player_ai {
            platform_ui::ai_turn(&mut gs, &actions)
        } else {
            let mut ui = DsUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::human_turn(&mut ui, &mut gs, &actions)
        };
        if !ok {
            if !is_current_player_ai {
                break;
            }
        }
        game_setup::settle_auto(&mut gs);
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

    let all_cards = deck_parser::load_two_decks(0, 1.min(dc.saturating_sub(1)));
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
    let decks: Vec<rabuka_engine::deck_parser::DeckEntry> =
        serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let mut all_cards = deck_parser::load_two_decks(d1, d2);
    CardLoader::attach_abilities(&mut all_cards);

    let to_deck_list =
        |e: &rabuka_engine::deck_parser::DeckEntry| -> rabuka_engine::deck_parser::DeckList {
            rabuka_engine::deck_parser::DeckList {
                name: e.name.clone(),
                entries: e
                    .cards
                    .iter()
                    .map(|c| rabuka_engine::deck_parser::DeckEntry {
                        card_no: c.clone(),
                        quantity: 1,
                    })
                    .collect(),
            }
        };
    let dl1 = to_deck_list(&decks[d1]);
    let dl2 = to_deck_list(&decks[d2]);
    rabuka_engine::game_setup::test_ai_vs_ai(&all_cards, &dl1, &dl2, 5).map_err(|e| e.into())
}

// ── End of tests ─────────────────────────────────────────────────

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
