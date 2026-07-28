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
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

psp::module!("rabuka", 1, 0);

const DECKS_JSON: &str = include_str!("../../baked/decks.json");

use rabuka_engine::deck_parser::DECK_CARD_FILES;

fn truncate_chars(s: &str, max_chars: usize) -> &str {
    let mut char_count = 0;
    for (i, _) in s.char_indices() {
        if char_count >= max_chars {
            return &s[..i];
        }
        char_count += 1;
    }
    s
}

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
    let mode_idx = select(&mut display, &mut input, &modes, "Mode");
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;
    let run_tests = mode_idx == 3;

    if run_tests {
        run_on_device_tests(&mut display, &mut input);
        return;
    }

    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(decks.len())
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
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
            show_result(&mut display, &mut input, &gs);
            break;
        }

        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(&mut display, &mut input, &gs);
            break;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            wait_frames(10);
            continue;
        }

        if gs.has_pending_choice() {
            if !handle_choice(&mut display, &mut input, &mut gs) {
                break;
            }
            continue;
        }

        let is_ai = ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
        let ok = if is_ai {
            ai_turn(&mut display, &mut gs, &actions)
        } else {
            human_turn(&mut display, &mut input, &mut gs, &actions)
        };
        if !ok {
            break;
        }
        game_setup::settle_auto(&mut gs);
    }
}

#[derive(serde::Deserialize)]
struct DeckEntry {
    name: String,
    cards: Vec<String>,
}

fn select(display: &mut Display, input: &mut Input, items: &[&str], title: &str) -> usize {
    let mut sel = 0usize;
    loop {
        display.draw_menu(items, sel, title);
        display.swap_buffers();
        wait_frames(2);
        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(items.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::Cross) {
            return sel;
        }
    }
}

fn ai_turn(_display: &mut Display, gs: &mut GameState, actions: &[game_setup::Action]) -> bool {
    let idx = rng::rand_range(actions.len());
    execute_action(gs, &actions[idx])
}

fn human_turn(
    display: &mut Display,
    input: &mut Input,
    gs: &mut GameState,
    actions: &[game_setup::Action],
) -> bool {
    let mut sel = 0usize;
    let mut scroll_offset = 0usize;
    const VISIBLE_ACTIONS: usize = 10;
    loop {
        display.clear();
        display.println(&format!("Turn {} | {:?}", gs.turn_number, gs.current_phase));

        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        let tag = |active: bool| if active { ">>" } else { "  " };
        display.println(&format!(
            "{} P1 h:{} e:{} dk:{}",
            tag(is_p1),
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len()
        ));
        display.println(&format!(
            "{} P2 h:{} e:{} dk:{}",
            tag(!is_p1),
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len()
        ));
        display.println("");

        if sel < scroll_offset {
            scroll_offset = sel;
        }
        if sel >= scroll_offset + VISIBLE_ACTIONS {
            scroll_offset = sel + 1 - VISIBLE_ACTIONS;
        }

        let end = (scroll_offset + VISIBLE_ACTIONS).min(actions.len());
        for i in scroll_offset..end {
            let p = if i == sel { " >" } else { "  " };
            let line = actions[i].description.lines().next().unwrap_or("");
            let card_tag = match &actions[i].parameters {
                Some(params) => params
                    .card_no
                    .as_ref()
                    .map(|no| alloc::format!(" [{}]", no))
                    .unwrap_or_default(),
                None => alloc::string::String::new(),
            };
            let desc_max = 50usize.saturating_sub(card_tag.len());
            let truncated = truncate_chars(line, desc_max);
            display.println(&format!("{p}[{i}] {truncated}{card_tag}"));
        }
        if actions.len() > end {
            display.println(&format!("  .. {} more", actions.len() - end));
        }
        display.swap_buffers();
        wait_frames(2);

        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(actions.len().saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::Cross) {
            return execute_action(gs, &actions[sel]);
        } else if input.just_pressed(Button::Circle) || input.just_pressed(Button::Start) {
            return false;
        }
    }
}

fn execute_action(gs: &mut GameState, action: &game_setup::Action) -> bool {
    if let Err(e) = game_setup::execute_action(gs, action) {
        dprintln!("Action error: {}", e);
    }
    true
}

fn menu_select(
    display: &mut Display,
    input: &mut Input,
    items: &[String],
    title: &str,
    allow_skip: bool,
) -> Option<usize> {
    let total = if allow_skip {
        items.len() + 1
    } else {
        items.len()
    };
    if total == 0 {
        return None;
    }
    let mut sel = 0usize;
    loop {
        display.clear();
        display.println(title);
        for (i, item) in items.iter().enumerate() {
            let prefix = if i == sel { " >" } else { "  " };
            display.println(&format!("{prefix} {item}"));
        }
        if allow_skip {
            let prefix = if sel == items.len() { " >" } else { "  " };
            display.println(&format!("{}  [Skip]", prefix));
        }
        display.println("");
        display.println("  D-Pad: navigate  X: select");
        display.swap_buffers();
        wait_frames(2);
        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(total.saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::Cross) {
            if allow_skip && sel >= items.len() {
                return None;
            }
            return Some(sel);
        }
    }
}

fn handle_choice(display: &mut Display, input: &mut Input, gs: &mut GameState) -> bool {
    use rabuka_engine::ability::types::Choice;
    use rabuka_engine::ability::util::zone_cards;

    let choice = match gs.get_pending_choice() {
        Some(c) => c.clone(),
        None => return true,
    };

    match choice {
        Choice::SelectAutoAbility {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options
                .iter()
                .map(|o| format!("{}: {}", o.card_name, o.ability_text))
                .collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, Some(0), None).ok();
                return true;
            }
            let sel = menu_select(display, input, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            target_player_id,
            description,
            filtered_indices,
            ..
        } => {
            let player = if let Some(ref pid) = target_player_id {
                if pid == &gs.player1.id {
                    &gs.player1
                } else {
                    &gs.player2
                }
            } else {
                gs.active_player()
            };
            let card_ids = zone_cards(player, &zone);
            let items: Vec<String> = match filtered_indices {
                Some(ref indices) => indices
                    .iter()
                    .map(|&i| {
                        if i < card_ids.len() {
                            gs.card_database
                                .get_card(card_ids[i])
                                .map(|c| c.name.to_string())
                                .unwrap_or_else(|| format!("#{}", card_ids[i]))
                        } else {
                            format!("#{}", i)
                        }
                    })
                    .collect(),
                None => card_ids
                    .iter()
                    .map(|cid| {
                        gs.card_database
                            .get_card(*cid)
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| format!("#{}", cid))
                    })
                    .collect(),
            };

            if count <= 1 {
                let sel = menu_select(display, input, &items, &description, allow_skip);
                match sel {
                    None => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                    }
                    Some(idx) => {
                        let actual_idx = filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx);
                        TurnEngine::resume_with_choice(gs, None, Some(vec![actual_idx])).ok();
                    }
                }
            } else {
                let mut selected: Vec<usize> = Vec::new();
                while selected.len() < count.min(items.len()) {
                    let display_items: Vec<String> = items
                        .iter()
                        .enumerate()
                        .map(|(i, name)| {
                            if selected.contains(&i) {
                                format!("[X] {}", name)
                            } else {
                                format!("[ ] {}", name)
                            }
                        })
                        .collect();
                    let sel = menu_select(display, input, &display_items, &description, allow_skip);
                    match sel {
                        None => break,
                        Some(idx) => {
                            if !selected.contains(&idx) {
                                selected.push(idx);
                            }
                        }
                    }
                }
                let actual_indices: Vec<usize> = match filtered_indices {
                    Some(ref fi) => selected.iter().map(|&i| fi[i]).collect(),
                    None => selected,
                };
                TurnEngine::resume_with_choice(gs, None, Some(actual_indices)).ok();
            }
            true
        }
        Choice::SelectTarget {
            target,
            options,
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = match options {
                Some(ref opts) if !opts.is_empty() => opts.clone(),
                _ => (0..2).map(|i| format!("Option {}", i + 1)).collect(),
            };
            let sel = menu_select(display, input, &items, &description, allow_skip);
            match sel {
                None => match target.as_str() {
                    "choice" | "choice_string" | "conditional_optional" => {
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                    }
                    _ => {
                        TurnEngine::resume_with_choice(gs, Some(-1), None).ok();
                    }
                },
                Some(idx) => {
                    TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok();
                }
            };
            true
        }
        Choice::SelectPosition {
            description,
            allow_skip,
            ..
        } => {
            let items: Vec<String> = ["Left", "Center", "Right"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let sel = menu_select(display, input, &items, &description, allow_skip);
            match sel {
                None => TurnEngine::resume_with_choice(gs, Some(-1), None).ok(),
                Some(idx) => TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok(),
            };
            true
        }
        Choice::SelectHeartColor {
            options,
            description,
            ..
        } => {
            let sel = menu_select(display, input, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectHeartType {
            options,
            description,
            ..
        } => {
            let sel = menu_select(display, input, &options, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, Some(sel as i16), None).ok();
            true
        }
        Choice::SelectLiveSuccess {
            options,
            description,
            ..
        } => {
            let items: Vec<String> = options.iter().map(|o| o.card_name.clone()).collect();
            if items.is_empty() {
                TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok();
                return true;
            }
            let sel = menu_select(display, input, &items, &description, false).unwrap_or(0);
            TurnEngine::resume_with_choice(gs, None, Some(vec![sel])).ok();
            true
        }
    }
}

fn show_result(display: &mut Display, input: &mut Input, gs: &GameState) {
    display.clear();
    display.println("=== GAME OVER ===");
    display.println(&format!("{:?}", gs.game_result));
    display.println(&format!(
        "P1 success:{} wait:{}",
        gs.player1.success_live_card_zone.cards.len(),
        gs.player1.waitroom.cards.len()
    ));
    display.println(&format!(
        "P2 success:{} wait:{}",
        gs.player2.success_live_card_zone.cards.len(),
        gs.player2.waitroom.cards.len()
    ));
    display.println("Press X to exit");
    display.swap_buffers();
    loop {
        input.poll();
        if input.just_pressed(Button::Cross) || input.just_pressed(Button::Start) {
            break;
        }
        wait_frames(2);
    }
}

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
