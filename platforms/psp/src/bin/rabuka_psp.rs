#![no_std]
#![no_main]
#![allow(linker_messages)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use psp::dprintln;
use psp::sys::*;

use rabuka_psp::display::Display;
use rabuka_psp::input::{Button, Input};

use rabuka_engine::card::Card;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game::platform_ui;
use rabuka_engine::rng;

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
    if mode_idx == 3 {
        run_on_device_tests(&mut display, &mut input);
        return;
    }
    let mode = match mode_idx {
        1 => platform_ui::MatchMode::TwoPlayer,
        2 => platform_ui::MatchMode::AiVsAi,
        _ => platform_ui::MatchMode::VsAi,
    };

    let d1 = platform_ui::select(&mut ui, &deck_names, "Your Deck");
    let d2 = if matches!(mode, platform_ui::MatchMode::TwoPlayer) {
        platform_ui::select(&mut ui, &deck_names, "P2 Deck")
    } else {
        rng::rand_range(decks.len())
    };

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    display.println("Loading deck cards...");
    display.swap_buffers();
    let mut all_cards = rabuka_engine::deck_parser::load_two_decks(d1, d2);

    display.println("Attaching abilities...");
    display.swap_buffers();
    CardLoader::attach_abilities(&mut all_cards);

    let p1: Vec<&str> = decks[d1].cards.iter().map(|c| c.as_str()).collect();
    let p2: Vec<&str> = decks[d2].cards.iter().map(|c| c.as_str()).collect();

    platform_ui::run_match(&mut ui, &p1, &p2, all_cards, mode);
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
    use rabuka_engine::deck_parser::DECK_CARD_FILES;

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
    let decks: Vec<DeckEntry> =
        serde_json::from_str(DECKS_JSON).map_err(|e| alloc::format!("JSON: {}", e))?;
    if decks.len() < 2 {
        return Err("need 2+ decks".into());
    }

    let mut all_cards = rabuka_engine::deck_parser::load_two_decks(0, 1);
    CardLoader::attach_abilities(&mut all_cards);

    // Build DeckList from DeckListEntry
    let to_deck_list = |e: &DeckEntry| -> rabuka_engine::deck_parser::DeckList {
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
