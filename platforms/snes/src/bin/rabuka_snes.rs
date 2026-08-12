#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![allow(linker_messages)]

extern crate rabuka_snes as _;
extern crate alloc;

use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

use rabuka_snes::display::Display;
use rabuka_snes::input::{Button, Input};

core::arch::global_asm!(
    ".section .snes_header,\"a\",@progbits",
    ".ascii \"RABUKA SNES           \"",
    ".byte 0x20", // LoROM map mode
    ".byte 0x00", // ROM type
    ".byte 0x05", // ROM size (32 KiB)
    ".byte 0x00", // SRAM size
    ".byte 0x01", // destination
    ".byte 0x00", // fixed
    ".byte 0x00", // version
    ".word 0xFFFF", // checksum complement
    ".word 0x0000", // checksum
    options(),
);

struct SnesUi {
    display: Display,
    input: Input,
}

impl platform_ui::PlatformUi for SnesUi {
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
        self.display.wait();
    }
}

fn load_deck_cards(idx1: usize, idx2: usize) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    cards
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    rng::seed(0x5EED);

    let ui = SnesUi {
        display: Display::new(),
        input: Input::new(),
    };

    static DECKS: [&str; 2] = ["Deck 1", "Deck 2"];
    let names: Vec<&str> = DECKS.to_vec();

    platform_ui::run_embedded_game(
        ui,
        &names,
        |i| {
            if i < DECKS.len() {
                &DECKS[i..i + 1]
            } else {
                &[]
            }
        },
        |a, b| load_deck_cards(a, b),
    );

    loop {}
}
