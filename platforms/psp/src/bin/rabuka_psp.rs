#![no_std]
#![no_main]
#![allow(linker_messages)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use psp::dprintln;
use psp::sys::*;

use rabuka_psp::display::Display;
use rabuka_psp::input::{Button, Input};

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::deck_builder;
use rabuka_engine::game::platform_ui;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

struct PspUi<'a> {
    display: &'a mut Display,
    input: &'a mut Input,
}

impl<'a> platform_ui::PlatformUi for PspUi<'a> {
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
        wait_frames(1);
    }
}

psp::module!("rabuka", 1, 0);

const DECKS_JSON: &str = include_str!("../../baked/decks.json");

use rabuka_engine::deck_parser::DECK_CARD_FILES;

fn psp_main() {
    psp::enable_home_button();

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    display.println("Loading...");
    display.swap_buffers();

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();

    let modes = ["VS AI", "2 Player", "AI vs AI", "Run Tests"];
    let mut ui = PspUi {
        display: &mut display,
        input: &mut input,
    };
    let mode_idx = platform_ui::select(&mut ui, &modes, "Mode");
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;
    let run_tests = mode_idx == 3;

    if run_tests {
        run_on_device_tests(&mut display, &mut input);
        return;
    }

    let deck1_idx = platform_ui::select(&mut ui, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    display.println("Loading deck cards...");
    display.swap_buffers();
    let mut all_cards = deck_parser::load_two_decks(deck1_idx, deck2_idx);

    display.println("Attaching abilities...");
    display.swap_buffers();
    CardLoader::attach_abilities(&mut all_cards);

    display.println("Building database...");
    display.swap_buffers();
    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));

    display.println("Building decks...");
    display.swap_buffers();
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
            let mut ui = PspUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            let mut ui = PspUi {
                display: &mut display,
                input: &mut input,
            };
            platform_ui::show_result(&mut ui, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            wait_frames(10);
            continue;
        }

        if gs.has_pending_choice() {
            let mut ui = PspUi {
                display: &mut display,
                input: &mut input,
            };
            if !platform_ui::handle_choice(&mut ui, &mut gs) {
                break;
            }
            continue;
        }

        let is_ai = ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
        let ok = if is_ai {
            platform_ui::ai_turn(&mut gs, &actions)
        } else {
            let mut ui = PspUi {
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
    let mut tick: u64 = 0;
    unsafe {
        sceRtcGetCurrentTick(&mut tick);
    }
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
}

fn run_on_device_tests(display: &mut Display, input: &mut Input) {
    display.clear();
    display.println("=== PSP ON-DEVICE TESTS ===");
    display.println("Loading card data...");
    display.swap_buffers();
    wait_frames(30);

    let decks: Result<Vec<DeckEntry>, _> = serde_json::from_str(DECKS_JSON);
    match &decks {
        Ok(d) => display.println(&format!("DECKS: {} ok", d.len())),
        Err(e) => display.println(&format!("DECKS: FAIL - {}", e)),
    }
    display.swap_buffers();
    wait_frames(30);

    let mut passed = 0u32;
    let mut failed = 0u32;

    display.println("Loading deck 0 cards...");
    display.swap_buffers();
    wait_frames(15);
    let mut cards: Vec<Card> =
        serde_json::from_str(DECK_CARD_FILES[0]).expect("Failed to parse deck 0");
    CardLoader::attach_abilities(&mut cards);
    let wa = cards.iter().filter(|c| !c.abilities.is_empty()).count();
    display.println(&format!("DECK 0: {} cards", cards.len()));
    display.println(&format!("ABILITIES: {} cards have them", wa));
    if wa > 0 {
        passed += 1;
    } else {
        failed += 1;
    }

    let has_energy = cards.iter().any(|c| c.card_no.contains("LL-E-005"));
    display.println(if has_energy {
        "ENERGY: found"
    } else {
        "ENERGY: missing"
    });
    if has_energy {
        passed += 1;
    } else {
        failed += 1;
    }

    display.swap_buffers();
    wait_frames(30);

    if let Ok(ref d) = decks {
        if d.len() >= 2 {
            display.println("AI PLAY: 5 turns...");
            display.swap_buffers();
            wait_frames(15);
            match test_ai_vs_ai_psp() {
                Ok(n) => {
                    display.println(&format!("AI PLAY: {} actions (OK)", n));
                    passed += 1;
                }
                Err(e) => {
                    display.println(&format!("AI PLAY: {}", e));
                    failed += 1;
                }
            }
            display.swap_buffers();
            wait_frames(30);
        }
    }

    display.println(&alloc::format!(
        "RESULTS: {} passed, {} failed",
        passed,
        failed
    ));
    display.println("START=exit");
    display.swap_buffers();
    loop {
        input.poll();
        if input.just_pressed(Button::Start) || input.just_pressed(Button::Cross) {
            break;
        }
        wait_frames(2);
    }
}

fn test_ai_vs_ai_psp() -> Result<usize, alloc::string::String> {
    let decks: Vec<rabuka_engine::deck_parser::DeckEntry> =
        serde_json::from_str(DECKS_JSON).map_err(|e| alloc::format!("JSON: {}", e))?;
    if decks.len() < 2 {
        return Err("need 2+ decks".into());
    }

    let mut all_cards = deck_parser::load_two_decks(0, 1);
    CardLoader::attach_abilities(&mut all_cards);

    // Build DeckList from DeckEntry
    let to_deck_list =
        |e: &rabuka_engine::deck_parser::DeckEntry| -> rabuka_engine::deck_parser::DeckList {
            rabuka_engine::deck_parser::DeckList {
                name: e.name.clone(),
                entries: e
                    .cards
                    .iter()
                    .map(|c| rabuka_engine::deck_parser::DeckEntry {
                        card_no: c.clone(),
                        quantity: 1,
                    })
                    .collect(),
            }
        };
    let dl1 = to_deck_list(&decks[0]);
    let dl2 = to_deck_list(&decks[1]);
    rabuka_engine::game_setup::test_ai_vs_ai(&all_cards, &dl1, &dl2, 5).map_err(|e| e.into())
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe {
            sceKernelDelayThread(16_667);
        }
    }
}
