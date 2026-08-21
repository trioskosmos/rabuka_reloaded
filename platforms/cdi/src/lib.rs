//! Philips CD-i — native m68k port.
//!
//! SCC68070 @ 15.5MHz (68000-compatible), 1MB RAM, CD-ROM media.
//! Native LLVM `m68k-unknown-none-elf` via `m68k-cdi.json` (M68000 cpu,
//! big-endian, `max-atomic-width=16` → `Rc`/`Cell` fallbacks from
//! `engine/src/compat.rs`). Uses the same compact bytecode engine as
//! GBA/Jaguar/wasm: `cdi = no_std + bytecode_abilities + compact_cards +
//! compact_card_data + compact_state`. Proves the engine fits 1MB without
//! the `wasm2c` 3-4MB text tax.
//!
//! For bringup the binary is a `staticlib` — link it with a tiny CD-i stub
//! (crt0 + `cdi_main.c` calling `rabuka_cdi_match_probe` and printing the
//! result via the Green Book `ss_play`/`printf` or serial stub). The
//! `rabuka_cdi_*` exports mirror `platforms/wasm` for easy comparison.

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

// 1MB total CD-i RAM: code+rodata+heap+stack must fit.
// The wasm2c DC build is ~4.3MB text + 9.3MB heap = impossible on 1MB.
// Native m68k build is ~150-300KB code+data; we size the bump heap for that.
// Start small and bump if `heap_highwater` shows pressure; the probe export
// below reports it. GBA (288KB) proved 512KB heap is generous for a match.
const HEAP_SIZE: usize = 384 * 1024;

struct BumpAlloc;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut CURSOR: usize = 0;
static mut HIGH_WATER: usize = 0;

#[global_allocator]
static GLOBAL_ALLOC: BumpAlloc = BumpAlloc;

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let start = CURSOR;
        let aligned = (start + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > HEAP_SIZE {
            return core::ptr::null_mut();
        }
        CURSOR = end;
        if end > HIGH_WATER {
            HIGH_WATER = end;
        }
        HEAP.as_mut_ptr().add(aligned)
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

// ---- Headless UI (same as wasm: auto-press A) ----
struct HeadlessUi;
impl PlatformUi for HeadlessUi {
    fn clear_screen(&mut self) {}
    fn println(&mut self, _text: &str) {}
    fn swap_buffers(&mut self) {}
    fn poll_input(&mut self) {}
    fn just_pressed_a(&self) -> bool { true }
    fn just_pressed_b(&self) -> bool { false }
    fn just_pressed_up(&self) -> bool { false }
    fn just_pressed_down(&self) -> bool { false }
    fn just_pressed_start(&self) -> bool { false }
    fn wait_vblank(&mut self) {}
}

fn load_deck_cards(idx1: usize, idx2: usize) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    cards
}

/// Returns card count for decks 0+1 (proves baked blob + bytecode path).
#[no_mangle]
pub extern "C" fn rabuka_cdi_card_count() -> u32 {
    let cards = load_deck_cards(0, 1);
    if cards.is_empty() { u32::MAX } else { cards.len() as u32 }
}

/// Run one full AI-vs-AI match. Returns 0/1/2/3 GameResult code.
#[no_mangle]
pub extern "C" fn rabuka_cdi_match(seed: u32) -> u32 {
    rng::seed(seed);
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

/// Peak bump-allocator usage (for sizing `HEAP_SIZE` to 1MB budget).
#[no_mangle]
pub extern "C" fn rabuka_cdi_heap_highwater() -> u32 {
    unsafe { HIGH_WATER as u32 }
}

/// Nullary probe for quick checks: `(heap_highwater << 8) | match_result`.
#[no_mangle]
pub extern "C" fn rabuka_cdi_match_probe() -> u32 {
    let r = rabuka_cdi_match(0x5EED);
    (unsafe { HIGH_WATER as u32 } << 8) | (r & 0xFF)
}

/// CD-i entry point called from `cdi_main.c` crt0. Loops forever after
/// one probe so the serial stub can read back `heap_highwater` if needed.
/// Replace with a real menu loop once display/input glue exists.
#[no_mangle]
pub extern "C" fn rabuka_cdi_boot() -> u32 {
    rabuka_cdi_match_probe()
}
