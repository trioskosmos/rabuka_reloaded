//! Rabuka Reloaded — WebAssembly smoke-test harness.
//!
//! Compiles the full no_std engine to wasm32 and exports a minimal C ABI:
//!   rabuka_wasm_card_count()  -> decodes two baked decks + attaches bytecode
//!   abilities, returns the card count (proves the compact card blob +
//!   bytecode VM data path works under wasm).
//!   rabuka_wasm_match(seed)   -> runs a complete AI-vs-AI match headlessly
//!   (auto-confirm UI picks option 0 every time), returns the GameResult code.
//!
//! The transpiled-C/interpreter console ports would replace these exports
//! with real display/input shims, exactly like the other platform crates.

#![no_std]
extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::match_runner::{run_match, MatchMode};
use rabuka_engine::game::platform_ui::PlatformUi;
use rabuka_engine::rng;

mod decks_baked;
use decks_baked::DECKS;

// ---- Bump allocator over a static heap in linear memory ----
const HEAP_SIZE: usize = 8 * 1024 * 1024;

struct BumpAlloc(u32);

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut CURSOR: usize = 0;

#[global_allocator]
static GLOBAL_ALLOC: BumpAlloc = BumpAlloc(0);

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let start = CURSOR;
        let aligned = (start + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > HEAP_SIZE {
            return core::ptr::null_mut();
        }
        CURSOR = end;
        HEAP.as_mut_ptr().add(aligned)
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

// ---- Headless UI: always "presses A", i.e. confirms option 0 every time ----
struct HeadlessUi;

impl PlatformUi for HeadlessUi {
    fn clear_screen(&mut self) {}
    fn println(&mut self, _text: &str) {}
    fn swap_buffers(&mut self) {}
    fn poll_input(&mut self) {}
    fn just_pressed_a(&self) -> bool {
        true
    }
    fn just_pressed_b(&self) -> bool {
        false
    }
    fn just_pressed_up(&self) -> bool {
        false
    }
    fn just_pressed_down(&self) -> bool {
        false
    }
    fn just_pressed_start(&self) -> bool {
        false
    }
    fn wait_vblank(&mut self) {}
}

fn load_deck_cards(idx1: usize, idx2: usize) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    cards
}

/// Decode two baked decks and attach bytecode abilities. Returns card count,
/// or usize::MAX on empty (data path failure).
#[no_mangle]
pub extern "C" fn rabuka_wasm_card_count() -> u32 {
    let cards = load_deck_cards(0, 1);
    if cards.is_empty() {
        u32::MAX
    } else {
        cards.len() as u32
    }
}

/// Run one full AI-vs-AI match with the given RNG seed.
/// Returns the GameResult as a code: 0=FirstAttackerWins, 1=SecondAttackerWins,
/// 2=Draw, 3=Ongoing (should not happen), u32::MAX on panic-free-but-stuck guard.
#[no_mangle]
pub extern "C" fn rabuka_wasm_match(seed: u32) -> u32 {
    rng::seed(seed);

    // Two different baked decks so the match exercises more ability paths.
    let p1_cards: &[&str] = DECKS[0].cards;
    let p2_cards: &[&str] = DECKS[1].cards;
    let all_cards = load_deck_cards(0, 1);

    let mut ui = HeadlessUi;
    let result = run_match(&mut ui, p1_cards, p2_cards, all_cards, MatchMode::AiVsAi);

    match result {
        rabuka_engine::core::types::GameResult::FirstAttackerWins => 0,
        rabuka_engine::core::types::GameResult::SecondAttackerWins => 1,
        rabuka_engine::core::types::GameResult::Draw => 2,
        rabuka_engine::core::types::GameResult::Ongoing => 3,
    }
}
