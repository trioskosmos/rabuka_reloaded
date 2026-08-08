#![no_std]
#![no_main]

extern crate alloc;

use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;

use rabuka_ds::decks_baked::DECKS;
use rabuka_ds::display::Display;
use rabuka_ds::ffi;
use rabuka_ds::input::{Button, Input};

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::core::card_binary;
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

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

    display.println("Rabuka DS - loading...");
    display.swap_buffers();

    let decks = DECKS;
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name).collect();

    let ui = DsUi {
        display: &mut display,
        input: &mut input,
    };
    platform_ui::run_embedded_game(ui, &deck_names, |i| decks[i].cards, |a, b| {
        load_deck_cards_from_blob(decks, a, b)
    });

    loop {
        unsafe { ffi::nds_wait_vblank() }
    }
}

fn init_rng() {
    let tick = unsafe { ffi::nds_get_tick() };
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe { ffi::nds_wait_vblank() }
    }
}