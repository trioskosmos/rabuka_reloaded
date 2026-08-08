use alloc::vec::Vec;

use crate::display::Display;
use crate::input::{Button, Input};
use rabuka_engine::card::Card;
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

struct DcUi<'a> {
    display: &'a mut Display,
    input: &'a mut Input,
}

impl<'a> platform_ui::PlatformUi for DcUi<'a> {
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
        wait_ms(16);
    }
}

extern "C" {
    fn timer_ms_gettime64() -> u64;
    fn thd_sleep(duration: i32);
}

const DECKS_JSON: &str = include_str!("../../psp/baked/decks.json");

#[no_mangle]
pub extern "C" fn rabuka_main() {
    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();

    let mut ui = DcUi {
        display: &mut display,
        input: &mut input,
    };
    let mode_idx = platform_ui::select(&mut ui, &["VS AI", "2 Player"], "Mode");
    let mode = if mode_idx == 0 {
        platform_ui::MatchMode::VsAi
    } else {
        platform_ui::MatchMode::TwoPlayer
    };

    let d1 = platform_ui::select(&mut ui, &deck_names, "Your Deck");
    let d2 = if matches!(mode, platform_ui::MatchMode::TwoPlayer) {
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    } else {
        rng::rand_range(decks.len())
    };

    display.println("Loading...");
    display.swap_buffers();

    let mut cards: Vec<Card> = rabuka_engine::deck_parser::load_two_decks(d1, d2);
    rabuka_engine::card_loader::CardLoader::attach_abilities(&mut cards);

    let p1: Vec<&str> = decks[d1].cards.iter().map(|c| c.as_str()).collect();
    let p2: Vec<&str> = decks[d2].cards.iter().map(|c| c.as_str()).collect();

    platform_ui::run_match(&mut ui, &p1, &p2, cards, mode);
}

use rabuka_engine::deck_parser::DeckListEntry as DeckEntry;

fn init_rng() {
    let tick = unsafe { timer_ms_gettime64() };
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
}

fn wait_ms(ms: u32) {
    unsafe {
        thd_sleep(ms as i32);
    }
}
