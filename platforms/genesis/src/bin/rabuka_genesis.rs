#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

extern crate alloc;

use alloc::vec::Vec;
use core::panic::PanicInfo;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui::PlatformUi;
use rabuka_engine::rng;
use rabuka_genesis::decks_baked::DECKS;

// Vectors + header for Genesis 68000
#[link_section = ".vectors"]
#[used]
static VECTORS: [u32; 8] = [
    0x00FF0000, // SSP = top of 64KB RAM
    0x00000200, // PC = _start
    0, 0, 0, 0, 0, 0,
];

#[link_section = ".header"]
#[used]
static HEADER: [u8; 0x100] = {
    let mut h = [0u8; 0x100];
    let tag = b"SEGA MEGA DRIVE ";
    let mut i = 0;
    while i < tag.len() { h[i] = tag[i]; i += 1; }
    h
};

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        ALLOCATOR.lock().init(0x00FF4000 as *mut u8, 0xC000);
    }
    main_genesis()
}

fn load_deck_cards(idx1: usize, idx2: usize) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    cards
}

struct GenesisUi {
    display: rabuka_genesis::display::Display,
    input: rabuka_genesis::input::Input,
    // track last press to implement just_pressed via poll delta
    last_a: bool,
    last_b: bool,
    last_up: bool,
    last_down: bool,
    last_start: bool,
}

impl GenesisUi {
    fn new() -> Self {
        Self {
            display: rabuka_genesis::display::Display::new(),
            input: rabuka_genesis::input::Input::new(),
            last_a: false,
            last_b: false,
            last_up: false,
            last_down: false,
            last_start: false,
        }
    }
}

impl PlatformUi for GenesisUi {
    fn clear_screen(&mut self) { self.display.clear(); }
    fn println(&mut self, text: &str) { self.display.println(text); }
    fn swap_buffers(&mut self) { self.display.swap_buffers(); }
    fn poll_input(&mut self) { let _ = self.input.poll(); }
    fn just_pressed_a(&self) -> bool { false }
    fn just_pressed_b(&self) -> bool { false }
    fn just_pressed_up(&self) -> bool { false }
    fn just_pressed_down(&self) -> bool { false }
    fn just_pressed_start(&self) -> bool { false }
    fn wait_vblank(&mut self) { self.display.wait_vblank(); }
}

fn main_genesis() -> ! {
    rng::seed(0x5EED);
    let decks = DECKS;
    let names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    loop {
        let mut ui = GenesisUi::new();
        rabuka_engine::game::platform_ui::run_embedded_game(
            ui,
            &names,
            |i| decks[i].cards,
            |a, b| load_deck_cards(a, b),
        );
    }
}
