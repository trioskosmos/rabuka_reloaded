#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui::{self, MatchMode, PlatformUi};
use rabuka_engine::rng;

use rabuka_gba::decks_baked::DECKS;
use rabuka_gba::gba_ui::GbaUi;
use rabuka_gba::input::Input;
use rabuka_gba::screens::Screen;
use rabuka_gba::match_runner::run_match_with_mixer;

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
    // Initialize audio mixer at 18157 Hz (good quality, ~10% CPU for 4 channels)
    let mut mixer = gba.mixer.mixer(Frequency::Hz18157);
    let vblank = agb::interrupt::VBlank::get();

    let mut display = rabuka_gba::ui::Display::new(gba.graphics.get());
    let mut input = Input::new();
    rng::seed(0x5EED);

    let decks = DECKS;
    let names: Vec<&str> = decks.iter().map(|d| d.name).collect();
    let modes = ["VS AI", "2 Player", "AI vs AI"];

    // Explicit boot flow — see `screens::Screen` for the full button map:
    // ModeSelect -> DeckSelectP1 -> (DeckSelectP2) -> Match -> Result -> ...
    // A finished match restarts cleanly at ModeSelect instead of freezing.
    loop {
        let _ = Screen::ModeSelect;
        let mut ui = GbaUi::new(&mut display, &mut input);
        let as_ui = &mut ui as &mut dyn PlatformUi;
        let mode_idx = platform_ui::select(as_ui, &modes, "MODE Up/Dn:A/Start");
        let mode = match mode_idx {
            1 => MatchMode::TwoPlayer,
            2 => MatchMode::AiVsAi,
            _ => MatchMode::VsAi,
        };

        let _ = Screen::DeckSelectP1;
        let d1 = platform_ui::select(as_ui, &names, "P1 DECK Up/Dn:A/Start");
        let _ = Screen::DeckSelectP2;
        let d2 = if matches!(mode, MatchMode::TwoPlayer) {
            platform_ui::select(as_ui, &names, "P2 DECK Up/Dn:A/Start")
        } else {
            rng::rand_range(names.len())
        };

        // Match (Screen::Board/Actions/StartMenu/CardDetail/ChoiceGrid are
        // driven by the engine from here; Screen::Result shows at the end).
        let _ = Screen::Board;
        let p1_cards = decks[d1].cards;
        let p2_cards = decks[d2].cards;
        let all_cards = load_deck_cards(decks, d1, d2);
        
        // Run match with audio mixer frame updates
        run_match_with_mixer(&mut ui, p1_cards, p2_cards, all_cards, mode, &mut mixer, &vblank);
        
        let _ = Screen::Result;
    }
}
