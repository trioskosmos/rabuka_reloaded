#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

use rabuka_gba::decks_baked::DECKS;
use rabuka_gba::gba_ui::GbaUi;
use rabuka_gba::input::Input;

fn load_deck_cards(
    _decks: &[rabuka_gba::decks_baked::DeckInfo],
    idx1: usize,
    idx2: usize,
) -> Vec<Card> {
    let mut cards = rabuka_engine::game::deck_parser::load_two_decks(idx1, idx2);
    CardLoader::attach_abilities(&mut cards);
    cards
}

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut display = rabuka_gba::ui::Display::new(gba.graphics.get());
    let mut input = Input::new();
    rng::seed(0x5EED);

    let decks = DECKS;
    let names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    // Run the whole flow (mode select -> deck select -> match) forever. If a
    // match ends early (e.g. the player presses B to pass at the RPS screen, or
    // a game result is reached), restart cleanly at the mode select instead of
    // dropping into a frozen black screen.
    loop {
        let ui = GbaUi::new(&mut display, &mut input);
        platform_ui::run_embedded_game(ui, &names, |i| decks[i].cards, |a, b| {
            load_deck_cards(decks, a, b)
        });
    }
}
