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

// ---- Host imports: the console shell (KOS C code on Dreamcast, etc.)
// implements these; the engine's PlatformUi calls forward to them. ----
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "host")]
extern "C" {
    fn host_clear_screen();
    fn host_println(ptr: *const u8, len: u32);
    /// Returns a bitmask of currently-held buttons: A=1, B=2, Up=4, Down=8, Start=16.
    fn host_poll_buttons() -> u32;
    fn host_wait_vblank();
}

/// Button bits mirrored on both sides of the boundary.
mod btns {
    pub const A: u32 = 1;
    pub const B: u32 = 2;
    pub const UP: u32 = 4;
    pub const DOWN: u32 = 8;
    pub const START: u32 = 16;
}

/// Playable UI: text out to the host console, edge-detected buttons in.
struct HostUi {
    prev: u32,
    cur: u32,
}

impl HostUi {
    fn new() -> Self {
        HostUi { prev: 0, cur: 0 }
    }
    fn edge(&self, bit: u32) -> bool {
        self.cur & bit != 0 && self.prev & bit == 0
    }
}

impl PlatformUi for HostUi {
    fn clear_screen(&mut self) {
        unsafe { host_clear_screen() }
    }
    fn println(&mut self, text: &str) {
        unsafe { host_println(text.as_ptr(), text.len() as u32) }
    }
    fn swap_buffers(&mut self) {}
    fn poll_input(&mut self) {
        self.prev = self.cur;
        self.cur = unsafe { host_poll_buttons() };
    }
    fn just_pressed_a(&self) -> bool {
        self.edge(btns::A)
    }
    fn just_pressed_b(&self) -> bool {
        self.edge(btns::B)
    }
    fn just_pressed_up(&self) -> bool {
        self.edge(btns::UP)
    }
    fn just_pressed_down(&self) -> bool {
        self.edge(btns::DOWN)
    }
    fn just_pressed_start(&self) -> bool {
        self.edge(btns::START)
    }
    fn wait_vblank(&mut self) {
        unsafe { host_wait_vblank() }
    }
    // ---- layout (see engine PlatformUi docs) ----
    // DC BIOS font: 12px thin (ASCII) / 24px wide (CJK) glyphs on a
    // 640px line => 53 half-width columns. CJK = 2 columns matches the
    // trait's convention exactly.
    fn option_cols(&self) -> usize {
        53
    }
    // 20 rows of 24px; menus get title + ".. N more" lines around the
    // list, so offer 17 item rows.
    fn option_rows(&self) -> usize {
        17
    }
}

fn load_deck_cards(idx1: usize, idx2: usize) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    cards
}

/// Playable game: mode select -> deck select -> full match, all rendered
/// through the host's text console and driven by the host's controller.
/// Returns the GameResult code (0 FirstAttacker, 1 SecondAttacker, 2 Draw).
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn rabuka_wasm_game_run(seed: u32) -> u32 {
    rng::seed(seed);

    let deck_names: Vec<&str> = DECKS.iter().map(|d| d.name).collect();
    let ui = HostUi::new();
    let result = rabuka_engine::game::match_runner::run_embedded_game(
        ui,
        &deck_names,
        |i| DECKS[i].cards,
        |a, b| load_deck_cards(a, b),
    );

    match result {
        rabuka_engine::core::types::GameResult::FirstAttackerWins => 0,
        rabuka_engine::core::types::GameResult::SecondAttackerWins => 1,
        rabuka_engine::core::types::GameResult::Draw => 2,
        rabuka_engine::core::types::GameResult::Ongoing => 3,
    }
}

// ---- Heap over a static array in linear memory ----
// Bump allocator PLUS recycling: dealloc'd blocks are recorded in a table and
// reused best-fit by later allocs. The engine drops Vecs/Strings constantly
// (per-screen buffers, choice lists); a pure bump ratchet leaked all of it,
// which is why 8MB survived where 2MB died mid-match. Reuse bounds the heap
// near the true working set. If the recycle table fills, we fall back to
// leaking (the old behavior) rather than failing.
#[cfg(feature = "jaguar")]
const HEAP_SIZE: usize = 512 * 1024;
#[cfg(not(feature = "jaguar"))]
const HEAP_SIZE: usize = 2 * 1024 * 1024;

/// Max simultaneously-tracked freed blocks. Each entry is (offset, len).
const RECYCLE_CAP: usize = 4096;

struct RecycleAlloc;

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut CURSOR: usize = 0;
static mut HIGH_WATER: usize = 0;
static mut RECYCLE: [(u32, u32); RECYCLE_CAP] = [(0, 0); RECYCLE_CAP];
static mut RECYCLE_LEN: usize = 0;

#[global_allocator]
static GLOBAL_ALLOC: RecycleAlloc = RecycleAlloc;

unsafe impl GlobalAlloc for RecycleAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(4);
        // First-fit reuse: take the first tracked free block that satisfies
        // size + alignment. Best-fit scanned the whole table per alloc inside
        // interpreted code; first-fit exits early and keeps allocation nearly
        // free. Fragmentation risk is acceptable — block sizes repeat heavily
        // in this engine (per-screen buffers, choice lists).
        let mut hit: usize = usize::MAX;
        for i in 0..RECYCLE_LEN {
            let (off, len) = RECYCLE[i];
            if len as usize >= layout.size() && (off as usize) % align == 0 {
                hit = i;
                break;
            }
        }
        let off = if hit != usize::MAX {
            let (off, _) = RECYCLE[hit];
            RECYCLE[hit] = RECYCLE[RECYCLE_LEN - 1];
            RECYCLE_LEN -= 1;
            off as usize
        } else {
            // fresh bump slice
            let start = (CURSOR + align - 1) & !(align - 1);
            let end = start + layout.size();
            if end > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            CURSOR = end;
            start
        };
        let end = off + layout.size();
        if end > HIGH_WATER {
            HIGH_WATER = end;
        }
        HEAP.as_mut_ptr().add(off)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() == 0 || RECYCLE_LEN >= RECYCLE_CAP {
            return; // nothing to track, or table full -> leak (old behavior)
        }
        let base = HEAP.as_mut_ptr() as usize;
        let off = ptr as usize - base;
        if off >= HEAP_SIZE {
            return;
        }
        RECYCLE[RECYCLE_LEN] = (off as u32, layout.size() as u32);
        RECYCLE_LEN += 1;
    }
}

/// Peak bytes handed out by the bump allocator across all allocations so far
/// (allocations are never freed, so this equals total bytes ever allocated).
/// Used to size HEAP_SIZE for RAM-constrained consoles (Jaguar: 2MB DRAM).
#[no_mangle]
pub extern "C" fn rabuka_wasm_heap_highwater() -> u32 {
    unsafe { HIGH_WATER as u32 }
}

/// Zero-arg diagnostic for `wasm-interp --run-all-exports` (which only invokes
/// nullary exports): runs a full headless match at the standard seed, then
/// returns `(heap_highwater << 8) | match_result`.
#[no_mangle]
pub extern "C" fn rabuka_wasm_match_probe() -> u32 {
    let r = rabuka_wasm_match(0x5EED);
    (unsafe { HIGH_WATER as u32 } << 8) | (r & 0xFF)
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
