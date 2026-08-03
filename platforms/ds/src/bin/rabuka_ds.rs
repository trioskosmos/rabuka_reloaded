#![no_std]
#![no_main]

extern crate alloc;

use alloc::ffi::CString;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use rabuka_ds::decks_baked::DECKS;
use rabuka_ds::display::Display;
use rabuka_ds::ffi;
use rabuka_ds::input::{Button, Input};

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

struct DsUi<'a> {
    display: &'a mut Display,
    input: &'a mut Input,
}

impl<'a> platform_ui::PlatformUi for DsUi<'a> {
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
        wait_frames(1);
    }
}

/// Normalize a card number to the canonical form used by deck lists
/// (uppercase ASCII + halfwidth punctuation). cards.json stores some card_nos
/// with fullwidth chars (＋, ！, －, ａ-ｚ); the blob keeps those raw, so match
/// must fold them to ASCII too.
fn normalize_blob_card_no(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'a'..='z' => out.push((ch as u8 - b'a' + b'A') as char),
            'ａ'..='ｚ' => out.push((ch as u32 - 'ａ' as u32 + 'A' as u32) as u8 as char),
            '０'..='９' => out.push((ch as u32 - '０' as u32 + '0' as u32) as u8 as char),
            '＋' => out.push('+'),
            '！' => out.push('!'),
            '－' => out.push('-'),
            _ => out.push(ch),
        }
    }
    out
}

/// Build the union of two decks' cards directly from the engine's compact blob
/// (canonical mixed-case keys) — no runtime JSON, no serde.
fn load_deck_cards_from_blob(
    decks: &[rabuka_ds::decks_baked::DeckInfo],
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
        if wanted.is_empty() {
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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("PANIC: {}", info);
    if let Ok(c) = CString::new(msg.clone()) {
        unsafe {
            ffi::nds_clear();
            ffi::nds_nocash_log(c.as_ptr());
            ffi::nds_print(b"FROZEN - PANIC\n\0".as_ptr());
            ffi::nds_print(c.as_ptr());
            ffi::nds_set_backdrop_color(0x001F); // red bottom screen
        }
    }
    loop {
        unsafe { ffi::nds_wait_vblank() }
    }
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    unsafe {
        ffi::nds_init();
    }

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();
    stage("stage02 rng", ST_DARKRED);

    display.println("Rabuka DS - loading...");
    display.swap_buffers();

    let decks = DECKS;
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name).collect();
    stage("stage03 decks", ST_GREEN);

    let modes = ["VS AI", "2 Player", "AI vs AI"];
    stage("stage04 mode-sel", ST_BLUE);
    let mode_idx = {
        let mut ui = DsUi {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &modes, "Mode")
    };
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;

    stage("stage05 deck1-sel", ST_WHITE);
    let deck1_idx = {
        let mut ui = DsUi {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &deck_names, "Your Deck")
    };
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        let mut ui = DsUi {
            display: &mut display,
            input: &mut input,
        };
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading deck cards...");
    display.swap_buffers();
    stage("stage06 load-cards", ST_CYAN);
    let all_cards = load_deck_cards_from_blob(decks, deck1_idx, deck2_idx);

    display.println("Building database...");
    display.swap_buffers();
    stage("stage07 build-db", ST_MAGENTA);
    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));

    display.println("Building decks...");
    display.swap_buffers();
    stage("stage08 build-decks", ST_YELLOW);
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

    display.println("Setting up game...");
    display.swap_buffers();
    stage("stage09 setup_game", ST_ORANGE);
    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);
    stage("stage10 loop-running", ST_LT_CYAN);

    loop {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = DsUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = DsUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        // A permanently-flagged ability loop would stall the game; break out.
        if gs.is_loop_detected() {
            let mut ui = DsUi {
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
            wait_frames(8);
            continue;
        }

        if gs.has_pending_choice() {
            let mut ui = DsUi {
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
            let mut ui = DsUi {
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
        unsafe { ffi::nds_wait_vblank() }
    }
}

fn init_rng() {
    let tick = unsafe { ffi::nds_get_tick() };
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
}

// 15-bit RGB backdrop colors used as "stage LEDs" on the bottom screen.
const ST_RED: u16 = 0x001F; // 1  init
const ST_DARKRED: u16 = 0x0010; // 2  rng
const ST_GREEN: u16 = 0x03E0; // 3  decks
const ST_BLUE: u16 = 0x7C00; // 4  mode select
const ST_WHITE: u16 = 0x7FFF; // 5  deck select
const ST_CYAN: u16 = 0x3DE0; // 6  load cards
const ST_MAGENTA: u16 = 0x7C1F; // 7 build DB
const ST_YELLOW: u16 = 0x7FE0; // 8  build decks
const ST_ORANGE: u16 = 0x4A5F; // 9  setup_game
const ST_LT_CYAN: u16 = 0x03FF; // 10 loop running

fn stage(tag: &str, color: u16) {
    let c = CString::new(tag).unwrap_or_default();
    unsafe {
        ffi::nds_nocash_log(c.as_ptr());
        ffi::nds_set_backdrop_color(color);
    }
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe { ffi::nds_wait_vblank() }
    }
}
