#![no_std]
extern crate alloc;

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
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

const DECKS_JSON: &str = include_str!("../../psp/baked/decks.json");

struct WiiUi<'a> {
    display: &'a mut Display,
    input: &'a mut Input,
}

impl<'a> platform_ui::PlatformUi for WiiUi<'a> {
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
        self.display.wait_vsync();
    }
}

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

    let mut ui = WiiUi {
        display: &mut display,
        input: &mut input,
    };
    let mode_idx = platform_ui::select(&mut ui, &["VS AI", "2 Player"], "Mode");
    let mode = if mode_idx == 0 {
        platform_ui::MatchMode::VsAi
    } else {
        platform_ui::MatchMode::TwoPlayer
    };

    let d1 = platform_ui::select(&mut ui, &deck_names, "Your Deck");
    let d2 = if matches!(mode, platform_ui::MatchMode::TwoPlayer) {
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    } else {
        rng::rand_range(decks.len())
    };

    display.println("Loading...");
    let mut cards: Vec<Card> = rabuka_engine::deck_parser::load_two_decks(d1, d2);
    rabuka_engine::card_loader::CardLoader::attach_abilities(&mut cards);

    let p1: Vec<&str> = decks[d1].cards.iter().map(|c| c.as_str()).collect();
    let p2: Vec<&str> = decks[d2].cards.iter().map(|c| c.as_str()).collect();

    platform_ui::run_match(&mut ui, &p1, &p2, cards, mode);
}

use rabuka_engine::deck_parser::DeckListEntry as DeckEntry;

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
