use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use crate::display::Display;
use crate::input::{Button, Input};
use rabuka_engine::card::Card;
use rabuka_engine::game::deck_builder;
use rabuka_engine::game::platform_ui;

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
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

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

    let mode_idx = select(&mut display, &mut input, &["VS AI", "2 Player"], "Mode");
    let vs_ai = mode_idx == 0;

    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai {
        rng::rand_range(decks.len())
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
    };

    display.println("Loading...");
    display.swap_buffers();

    let mut cards: Vec<Card> = rabuka_engine::deck_parser::load_two_decks(deck1_idx, deck2_idx);
    rabuka_engine::card_loader::CardLoader::attach_abilities(&mut cards);
    let mut db = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));

    let nums1: Vec<String> = decks[deck1_idx].cards.clone();
    let nums2: Vec<String> = decks[deck2_idx].cards.clone();

    let mut pd1 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums1)
        .expect("Failed to build P1 deck");
    let mut pd2 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums2)
        .expect("Failed to build P2 deck");
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
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

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    loop {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = DcUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = DcUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            wait_ms(100);
            continue;
        }

        if gs.has_pending_choice() {
            let mut ui = DcUi {
                display: &mut display,
                input: &mut input,
            };
            if !platform_ui::handle_choice(&mut ui, &mut gs) {
                break;
            }
            continue;
        }

        let is_ai = vs_ai && gs.active_player().id != gs.player1.id;
        let ok = if is_ai {
            platform_ui::ai_turn(&mut gs, &actions)
        } else {
            let mut ui = DcUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::human_turn(&mut ui, &mut gs, &actions)
        };
        if !ok {
            break;
        }
        game_setup::settle_auto(&mut gs);
    }
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
