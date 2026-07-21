#![no_std]
#![no_main]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::hash::{BuildHasher, Hasher};

use rabuka_ds::display::Display;
use rabuka_ds::input::{Button, Input};

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::game::deck_builder;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::rng;
use rabuka_engine::turn::TurnEngine;

extern "C" {
    static __heap_start_ntr: u8;
    fn nds_init();
    fn nds_get_tick() -> u64;
    fn nds_wait_vblank();
}

const HEAP_END: usize = 0x0238_0000;

struct DsAllocator {
    next: AtomicUsize,
}

unsafe impl GlobalAlloc for DsAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align().max(4);
        loop {
            let curr = self.next.load(Ordering::SeqCst);
            let aligned = (curr + align - 1) & !(align - 1);
            let new_next = aligned + size;
            if new_next > HEAP_END {
                return core::ptr::null_mut();
            }
            if self
                .next
                .compare_exchange_weak(curr, new_next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return aligned as *mut u8;
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: DsAllocator = DsAllocator {
    next: AtomicUsize::new(0),
};

#[derive(Default, Clone, Copy)]
struct DsHasher(u64);

impl Hasher for DsHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.wrapping_mul(131).wrapping_add(b as u64);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

impl BuildHasher for DsHasher {
    type Hasher = DsHasher;
    fn build_hasher(&self) -> DsHasher {
        DsHasher(0)
    }
}

macro_rules! deck_cards {
    ($n:literal) => {
        include_str!(concat!("../../../ports/psp/baked/deck_", $n, "_cards.json"))
    };
}

const DECK_CARDS: &[&str] = &[
    deck_cards!("0"),
    deck_cards!("1"),
    deck_cards!("2"),
    deck_cards!("3"),
    deck_cards!("4"),
    deck_cards!("5"),
    deck_cards!("6"),
    deck_cards!("7"),
    deck_cards!("8"),
];
const DECKS_JSON: &str = include_str!("../../../ports/psp/baked/decks.json");

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

#[no_mangle]
pub extern "C" fn main() {
    unsafe {
        ALLOCATOR
            .next
            .store(&__heap_start_ntr as *const u8 as usize, Ordering::SeqCst);
        nds_init();
    }

    let mut display = Display::new();
    let mut input = Input::new();
    init_rng();

    display.println("Loading...");
    display.swap_buffers();

    let decks: Vec<DeckEntry> = serde_json::from_str(DECKS_JSON).expect("Failed to parse decks");
    let deck_names: Vec<&str> = decks.iter().map(|d| d.name.as_str()).collect();

    let modes = ["VS AI", "2 Player"];
    let mode_idx = select(&mut display, &mut input, &modes, "Mode");
    let vs_ai = mode_idx == 0;

    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai {
        rng::rand_range(decks.len())
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    let cards1: Vec<Card> =
        serde_json::from_str(DECK_CARDS[deck1_idx]).expect("Failed to parse deck cards");
    let cards2: Vec<Card> =
        serde_json::from_str(DECK_CARDS[deck2_idx]).expect("Failed to parse deck cards");

    let mut card_map: hashbrown::HashMap<String, Card, DsHasher> =
        hashbrown::HashMap::with_hasher(DsHasher(0));
    for c in cards1.into_iter().chain(cards2.into_iter()) {
        let key = c.card_no.to_string();
        if !card_map.contains_key(&key) {
            card_map.insert(key, c);
        }
    }

    let cards: Vec<Card> = card_map.into_values().collect();
    let mut db = Arc::new(CardDatabase::load_or_create(cards));

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

        settle_auto(&mut gs);
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

        let is_ai = vs_ai && gs.active_player().id != gs.player1.id;
        let ok = if is_ai {
            ai_turn(&mut display, &mut gs, &actions)
        } else {
            human_turn(&mut display, &mut input, &mut gs, &actions)
        };
        if !ok {
            break;
        }
        settle_auto(&mut gs);
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
        } else if input.just_pressed(Button::A) {
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
            let p = if i == sel { ">" } else { " " };
            let line = actions[i].description.lines().next().unwrap_or("");
            let card_tag = match &actions[i].parameters {
                Some(params) => params
                    .card_no
                    .as_ref()
                    .map(|no| alloc::format!(" [{}]", no))
                    .unwrap_or_default(),
                None => alloc::string::String::new(),
            };
            let desc_max = 28usize.saturating_sub(card_tag.len());
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
        } else if input.just_pressed(Button::A) {
            return execute_action(gs, &actions[sel]);
        } else if input.just_pressed(Button::B) || input.just_pressed(Button::Start) {
            return false;
        }
    }
}

fn execute_action(gs: &mut GameState, action: &game_setup::Action) -> bool {
    let params = action.parameters.clone();
    let result = TurnEngine::execute_main_phase_action(
        gs,
        &action.action_type,
        params.as_ref().and_then(|p| p.card_id),
        params.as_ref().and_then(|p| p.card_indices.clone()),
        params
            .as_ref()
            .and_then(|p| p.stage_area.as_ref().and_then(|s| s.parse().ok())),
        params.as_ref().and_then(|p| p.use_baton_touch),
    );
    match result {
        Ok(_) => {
            gs.reset_loop_detection();
            true
        }
        Err(_e) => {
            gs.reset_loop_detection();
            true
        }
    }
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
            let prefix = if i == sel { ">" } else { " " };
            display.println(&format!("{prefix} {item}"));
        }
        if allow_skip {
            let prefix = if sel == items.len() { ">" } else { " " };
            display.println(&format!("{}  [Skip]", prefix));
        }
        display.println("");
        display.println("DPAD:navigate A:select");
        display.swap_buffers();
        wait_frames(2);
        input.poll();
        if input.just_pressed(Button::Down) {
            sel = (sel + 1).min(total.saturating_sub(1));
        } else if input.just_pressed(Button::Up) {
            sel = sel.saturating_sub(1);
        } else if input.just_pressed(Button::A) {
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
            let player = target_player_id.as_ref().map_or_else(
                || gs.active_player(),
                |pid| {
                    if pid == &gs.player1.id {
                        &gs.player1
                    } else {
                        &gs.player2
                    }
                },
            );
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
                    None => TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok(),
                    Some(idx) => {
                        let actual_idx = filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx);
                        TurnEngine::resume_with_choice(gs, None, Some(vec![actual_idx])).ok()
                    }
                };
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
                        TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).ok()
                    }
                    _ => TurnEngine::resume_with_choice(gs, Some(-1), None).ok(),
                },
                Some(idx) => TurnEngine::resume_with_choice(gs, Some(idx as i16), None).ok(),
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

fn settle_auto(gs: &mut GameState) {
    for _ in 0..500 {
        if gs.has_pending_choice() || gs.game_result != GameResult::Ongoing {
            break;
        }
        if game_setup::is_automatic_phase(gs)
            || matches!(
                gs.current_phase,
                Phase::RockPaperScissors | Phase::ChooseFirstAttacker
            )
        {
            TurnEngine::advance_phase(gs);
        } else {
            break;
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
    display.println("Press A to exit");
    display.swap_buffers();
    loop {
        input.poll();
        if input.just_pressed(Button::A) || input.just_pressed(Button::Start) {
            break;
        }
        wait_frames(2);
    }
}

fn init_rng() {
    let tick = unsafe { nds_get_tick() };
    rng::seed(if tick == 0 { 1 } else { tick as u32 });
}

fn wait_frames(n: u32) {
    for _ in 0..n {
        unsafe { nds_wait_vblank() }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        nds_init();
        extern "C" {
            fn iprintf(fmt: *const u8, ...) -> i32;
        }
        let msg_buf;
        if let Some(msg) = info.message().as_str() {
            msg_buf = format!("PANIC: {}\0", msg);
        } else {
            msg_buf = format!("PANIC: {:?}\0", info.message());
        }
        iprintf(msg_buf.as_ptr());
        if let Some(loc) = info.location() {
            let loc_buf = format!(" at {}:{}\0", loc.file(), loc.line());
            iprintf(loc_buf.as_ptr());
        }
    }
    loop {
        unsafe { nds_wait_vblank() }
    }
}
