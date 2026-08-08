#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

use rabuka_ps1::decks_baked::DECKS;
use rabuka_ps1::display::Display;
use rabuka_ps1::input::{Button, Input};

struct Ps1Ui<'a> {
    display: &'a mut Display,
    input: &'a mut Input,
}

impl<'a> platform_ui::PlatformUi for Ps1Ui<'a> {
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
        self.input.just_pressed(Button::Cross)
    }
    fn just_pressed_b(&self) -> bool {
        self.input.just_pressed(Button::Circle)
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

// The full card database (532KB) cannot live in RAM next to ~1.8MB of code/data.
// The engine bakes a compact per-deck blob for each deck (15KB total for all 9);
// load_two_decks() decodes only the two selected decks' cards, so RAM holds only
// the cards actually in play. No CD read, no 532KB buffer, no baked subset.
fn load_deck_cards(
    decks: &[rabuka_ps1::decks_baked::DeckInfo],
    idx1: usize,
    idx2: usize,
) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    let _ = (decks, idx1, idx2);
    cards
}

#[no_mangle]
fn main() {
    let mut display = Display::new();
    let mut input = Input::new();
    rng::seed(0x5EED);

    let decks = DECKS;
    let names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    let ui = Ps1Ui {
        display: &mut display,
        input: &mut input,
    };
    platform_ui::run_embedded_game(ui, &names, |i| decks[i].cards, |a, b| {
        load_deck_cards(decks, a, b)
    });

    loop {
        display.swap_buffers();
    }
}
