#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::core::card_binary;
use rabuka_ps1::display::Display;
use rabuka_ps1::input::Input;

#[no_mangle]
fn main() {
    let mut display = Display::new();
    let mut input = Input::new();

    display.clear();
    display.println("Rabuka PS1");
    display.println("Loading cards...");
    display.swap_buffers();

    // Load all cards from the engine's compact blob (baked in for now; will
    // stream from the CD at runtime like real PS1 homebrew).
    let mut cards = Vec::new();
    for i in 0..card_binary::blob_card_count() {
        if let Some(c) = card_binary::decode_card_from_blob(i) {
            cards.push(c);
        }
    }
    CardLoader::attach_abilities(&mut cards);
    display.println(&format!("decoded {} cards", cards.len()));
    display.swap_buffers();

    let _db = CardDatabase::load_or_create(cards);
    display.println("database built");
    display.swap_buffers();

    // Keep the frame alive; poll input so the pad handler is serviced.
    loop {
        input.poll();
        display.swap_buffers();
    }
}
