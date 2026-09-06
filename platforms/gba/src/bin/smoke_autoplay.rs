//! Auto-play smoke test: drives the full flow (mode select -> deck select ->
//! match -> board rendering) with a deterministic scripted input, so the
//! integrated board renderer, card fronts, inspect mode and detail view get
//! exercised in mGBA without manual input. A VRAM exhaustion or render panic
//! shows up in the mGBA debug log.
//!
//! Build: cargo +nightly build --release -Z build-std=core,alloc
//! (target thumbv4t-none-eabi), then agb-gbafix and run in mGBA with logging.

#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use alloc::vec::Vec;

use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

use rabuka_gba::decks_baked::DECKS;
use rabuka_gba::gba_ui::{GbaUi, InputSource};
use rabuka_gba::input::Button;

/// Deterministic button pattern cycling every 40 frames: A confirms, Up/Down
/// move through lists, and rarer taps exercise Select (inspect toggle),
/// R (detail view) and B (back).
struct AutoInput {
    frame: u32,
}

impl InputSource for AutoInput {
    fn poll(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }
    fn just_pressed(&self, btn: Button) -> bool {
        // Spam A, Down, L — 10 frames each, repeating. Confirms menus,
        // moves, opens detail; the detail loop consumes the held buttons
        // to open/close rapidly for stress coverage.
        match (self.frame / 10) % 3 {
            0 => btn == Button::A,
            1 => btn == Button::Down,
            _ => btn == Button::L,
        }
    }
}

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut display = rabuka_gba::ui::Display::new(gba.graphics.get());
    let mut input = AutoInput { frame: 0 };
    rng::seed(0x5EED);

    let decks = DECKS;
    let names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    loop {
        let ui = GbaUi::new(&mut display, &mut input);
        platform_ui::run_embedded_game(ui, &names, |i| decks[i].cards, |a, b| {
            rabuka_engine::game::deck_parser::load_two_decks_with_abilities(a, b)
        });
    }
}
