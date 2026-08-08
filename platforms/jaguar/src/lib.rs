#![no_std]

extern crate alloc;

mod decks_baked;
mod display;
mod input;

use alloc::vec::Vec;

use core::alloc::{GlobalAlloc, Layout};

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

use crate::decks_baked::DECKS;
use crate::display::Display;
use crate::input::{Button, Input};

// ---- Global allocator: bump allocator over Jaguar DRAM ----
const HEAP_START: usize = 0x0006_0000;
const HEAP_END: usize = 0x000F_FFFF;

struct BumpAlloc;
static ALLOC: BumpAlloc = BumpAlloc;

#[global_allocator]
static GLOBAL_ALLOC: BumpAlloc = BumpAlloc;

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        static mut CURSOR: usize = HEAP_START;
        let start = CURSOR;
        let aligned = (start + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > HEAP_END {
            core::ptr::null_mut()
        } else {
            CURSOR = end;
            aligned as *mut u8
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

struct JaguarUi<'a> {
    display: &'a mut Display,
    input: &'a mut Input,
}

impl<'a> platform_ui::PlatformUi for JaguarUi<'a> {
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
        self.input.just_pressed(Button::Pause) || self.input.just_pressed(Button::Option)
    }
    fn wait_vblank(&mut self) {
        self.display.wait();
    }
}

fn load_deck_cards(
    decks: &[crate::decks_baked::DeckInfo],
    idx1: usize,
    idx2: usize,
) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    let _ = (decks, idx1, idx2);
    cards
}

#[no_mangle]
pub extern "C" fn rabuka_jaguar_main() -> ! {
    let mut display = Display::new();
    let mut input = Input::new();
    rng::seed(0x5EED);

    display.clear();
    display.println("Rabuka Jaguar");
    display.swap_buffers();

    let decks = DECKS;
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    let ui = JaguarUi {
        display: &mut display,
        input: &mut input,
    };
    platform_ui::run_embedded_game(ui, &deck_names, |i| decks[i].cards, |a, b| {
        load_deck_cards(decks, a, b)
    });

    loop {
        display.swap_buffers();
    }
}

