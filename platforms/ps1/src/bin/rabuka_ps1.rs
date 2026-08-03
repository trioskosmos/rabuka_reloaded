#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::{String, ToString};
// PS1 (MIPS-I) has no atomics: the engine's compat Arc is alloc::rc::Rc here.
use alloc::rc::Rc as Arc;
use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::core::card_binary;
use rabuka_engine::game::deck_builder::DeckBuilder;
use rabuka_engine::game::platform_ui;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

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
        self.display.swap_buffers();
    }
}

fn normalize_blob_card_no(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'a'..='z' => out.push((ch as u8 - b'a' + b'A') as char),
            _ => out.push(ch),
        }
    }
    out
}

/// Build the union of two decks' cards directly from the engine's compact blob.
fn load_deck_cards_from_blob(
    decks: &[rabuka_ps1::decks_baked::DeckInfo],
    idx1: usize,
    idx2: usize,
) -> Vec<Card> {
    let mut wanted: alloc::collections::BTreeSet<String> = alloc::collections::BTreeSet::new();
    for &idx in &[idx1, idx2] {
        if idx < decks.len() {
            for cn in decks[idx].cards {
                wanted.insert(normalize_blob_card_no(cn));
            }
        }
    }
    let mut indices: Vec<usize> = Vec::with_capacity(wanted.len());
    for i in 0..card_binary::blob_card_count() {
        if indices.len() == wanted.len() {
            break;
        }
        let Some(card) = card_binary::decode_card_from_blob(i) else {
            continue;
        };
        if wanted.remove(&normalize_blob_card_no(card.card_no.as_ref())) {
            indices.push(i);
        }
    }
    let mut cards: Vec<Card> = Vec::with_capacity(indices.len());
    for idx in indices {
        if let Some(card) = card_binary::decode_card_from_blob(idx) {
            cards.push(card);
        }
    }
    CardLoader::attach_abilities(&mut cards);
    cards
}

// The card blob lives on the CD (2MB RAM can't hold it baked). Load it into a
// static buffer once at boot, then point card_binary at it.
static mut CARD_BUF: [u32; (CARD_BLOB_SIZE + 3) / 4] = [0; (CARD_BLOB_SIZE + 3) / 4];
const CARD_BLOB_SIZE: usize = 532_378;

fn load_card_blob_from_cd() -> bool {
    use psx::sys::fs::{File, CDROM};
    let file = match File::<CDROM>::open("cdrom:\\CARDDATA.BIN") {
        Ok(f) => f,
        Err(_) => return false,
    };
    let buf: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(CARD_BUF.as_mut_ptr() as *mut u8, CARD_BUF.len() * 4)
    };
    let mut read_total = 0usize;
    while read_total < buf.len() {
        let want = core::cmp::min(2048, buf.len() - read_total);
        match file.read(&mut buf[read_total..read_total + want]) {
            Ok(n) if n > 0 => read_total += n,
            _ => break,
        }
    }
    unsafe {
        card_binary::EXTERN_CARD_BLOB = CARD_BUF.as_ptr() as *const u8;
        card_binary::EXTERN_CARD_BLOB_LEN = read_total;
    }
    read_total > 0
}

#[no_mangle]
fn main() {
    let mut display = Display::new();
    let mut input = Input::new();
    rng::seed(0x5EED);

    display.clear();
    display.println("Rabuka PS1");
    display.swap_buffers();

    let modes = ["VS AI", "2 Player", "AI vs AI"];
    let mode_idx = {
        let mut ui = Ps1Ui {
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
        let mut ui = Ps1Ui {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &deck_names, "Your Deck")
    };
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        let mut ui = Ps1Ui {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading card data...");
    display.swap_buffers();
    let blob_ok = load_card_blob_from_cd();
    if !blob_ok {
        display.println("ERROR: no carddata.bin on disc");
        display.swap_buffers();
        loop {
            display.swap_buffers();
        }
    }

    display.clear();
    display.println("Loading cards...");
    display.swap_buffers();
    let all_cards = load_deck_cards_from_blob(decks, deck1_idx, deck2_idx);

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
            let mut ui = Ps1Ui {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }
        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = Ps1Ui {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }
        if gs.is_loop_detected() {
            let mut ui = Ps1Ui {
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
            let mut ui = Ps1Ui {
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
            let mut ui = Ps1Ui {
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
