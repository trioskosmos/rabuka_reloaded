#![no_std]

extern crate alloc;

mod decks_baked;
mod display;
mod input;

use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::alloc::{GlobalAlloc, Layout};

use rabuka_engine::card::Card;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::deck_builder::DeckBuilder;
use rabuka_engine::game::platform_ui;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

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

    let modes = ["VS AI", "2 Player", "AI vs AI"];
    let mode_idx = {
        let mut ui = JaguarUi {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &modes, "Mode")
    };
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;

    let decks = DECKS;
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    let deck1_idx = {
        let mut ui = JaguarUi {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &deck_names, "Your Deck")
    };
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        let mut ui = JaguarUi {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading cards...");
    display.swap_buffers();
    let all_cards = load_deck_cards(decks, deck1_idx, deck2_idx);

    display.println("Building DB...");
    display.swap_buffers();
    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));

    display.println("Building decks...");
    display.swap_buffers();
    let nums1: Vec<String> = decks[deck1_idx]
        .cards
        .iter()
        .map(|c| c.to_string())
        .collect();
    let nums2: Vec<String> = decks[deck2_idx]
        .cards
        .iter()
        .map(|c| c.to_string())
        .collect();

    let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums1).expect("build P1 deck");
    let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums2).expect("build P2 deck");
    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);

    display.println("Setup...");
    display.swap_buffers();
    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    loop {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = JaguarUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }
        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = JaguarUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }
        if gs.is_loop_detected() {
            let mut ui = JaguarUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            gs.reset_loop_detection();
            continue;
        }

        if gs.has_pending_choice() {
            let mut ui = JaguarUi {
                display: &mut display,
                input: &mut input,
            };
            if !platform_ui::handle_choice(&mut ui, &mut gs) {
                break;
            }
            gs.reset_loop_detection();
            continue;
        }

        let is_ai = ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
        let ok = if is_ai {
            platform_ui::ai_turn(&mut gs, &actions)
        } else {
            let mut ui = JaguarUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::human_turn(&mut ui, &mut gs, &actions)
        };
        if !ok {
            break;
        }
        gs.reset_loop_detection();
        game_setup::settle_auto(&mut gs);
    }

    loop {
        display.swap_buffers();
    }
}

