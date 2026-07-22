#![no_std]
#![no_main]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::hash::{BuildHasher, Hasher};

use rabuka_ds::display::Display;
use rabuka_ds::input::{Button, Input};

use rabuka_engine::ability::ability_store::AbilityRef;
use rabuka_engine::card::{
    BaseHeart, BladeHeart, Card, CardDatabase, CardType, HeartColor, HeartMap, SpecialHeart,
};
use rabuka_engine::core::types::ArcStr;
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

// Flat binary card database (zero-copy, no heap allocation for parsing)
const CARDS_DB: &[u8] = include_bytes!("../../../../output/cards_database.bin");

struct DsCardDb<'a> {
    data: &'a [u8],
    card_count: u16,
    deck_count: u16,
    str_table_offset: usize,
    deck_index_offset: usize,
}

impl<'a> DsCardDb<'a> {
    fn new(data: &'a [u8]) -> Self {
        assert!(data.len() >= 12);
        assert!(&data[0..4] == b"RBCD");
        let card_count = u16::from_le_bytes([data[4], data[5]]);
        let deck_count = u16::from_le_bytes([data[6], data[7]]);
        let str_table_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let str_table_offset = 12 + card_count as usize * 20;
        let deck_index_offset = str_table_offset + str_table_size;
        Self {
            data,
            card_count,
            deck_count,
            str_table_offset,
            deck_index_offset,
        }
    }

    fn card_count(&self) -> u16 {
        self.card_count
    }

    fn deck_count(&self) -> u16 {
        self.deck_count
    }

    fn str_at(&self, offset: u16) -> &str {
        let start = self.str_table_offset + offset as usize;
        let end = self.data[start..].iter().position(|&b| b == 0).unwrap_or(0);
        core::str::from_utf8(&self.data[start..start + end]).unwrap_or("")
    }

    fn card_no(&self, card_idx: u16) -> &str {
        let rec = 12 + card_idx as usize * 20;
        let off = u16::from_le_bytes([self.data[rec], self.data[rec + 1]]);
        self.str_at(off)
    }

    fn card_name(&self, card_idx: u16) -> &str {
        let rec = 12 + card_idx as usize * 20;
        let off = u16::from_le_bytes([self.data[rec + 2], self.data[rec + 3]]);
        self.str_at(off)
    }

    fn card_type(&self, card_idx: u16) -> u8 {
        self.data[12 + card_idx as usize * 20 + 4]
    }

    fn card_cost(&self, card_idx: u16) -> u8 {
        self.data[12 + card_idx as usize * 20 + 5]
    }

    fn card_blade(&self, card_idx: u16) -> u8 {
        self.data[12 + card_idx as usize * 20 + 6]
    }

    fn card_score(&self, card_idx: u16) -> u8 {
        self.data[12 + card_idx as usize * 20 + 7]
    }

    fn card_ability_ref(&self, card_idx: u16) -> u16 {
        let rec = 12 + card_idx as usize * 20 + 14;
        u16::from_le_bytes([self.data[rec], self.data[rec + 1]])
    }

    fn deck_start_offset(&self, deck_idx: usize) -> usize {
        let mut offset = self.deck_index_offset;
        for _ in 0..deck_idx {
            let cc = u16::from_le_bytes([self.data[offset + 2], self.data[offset + 3]]) as usize;
            offset += 4 + cc * 2;
        }
        offset
    }

    fn deck_name(&self, deck_idx: usize) -> &str {
        let offset = self.deck_start_offset(deck_idx);
        let off = u16::from_le_bytes([self.data[offset], self.data[offset + 1]]);
        self.str_at(off)
    }

    fn deck_card_count(&self, deck_idx: usize) -> u16 {
        let offset = self.deck_start_offset(deck_idx);
        u16::from_le_bytes([self.data[offset + 2], self.data[offset + 3]])
    }

    fn deck_card_index(&self, deck_idx: usize, i: usize) -> u16 {
        let offset = self.deck_start_offset(deck_idx);
        let cc = u16::from_le_bytes([self.data[offset + 2], self.data[offset + 3]]) as usize;
        if i >= cc {
            return 0;
        }
        let idx_off = offset + 4 + i * 2;
        u16::from_le_bytes([self.data[idx_off], self.data[idx_off + 1]])
    }

    fn build_card(&self, card_idx: u16) -> Card {
        let rec = 12 + card_idx as usize * 20;
        let card_no_off = u16::from_le_bytes([self.data[rec], self.data[rec + 1]]);
        let name_off = u16::from_le_bytes([self.data[rec + 2], self.data[rec + 3]]);
        let card_type_val = self.data[rec + 4];
        let cost_val = self.data[rec + 5];
        let blade_val = self.data[rec + 6];
        let score_val = self.data[rec + 7];
        let base_heart_val = self.data[rec + 8];
        let group_val = self.data[rec + 9];
        let series_val = self.data[rec + 10];
        let unit_val = self.data[rec + 11];
        let blade_heart_val = self.data[rec + 12];
        let special_heart_val = self.data[rec + 13];
        let ability_ref_val = u16::from_le_bytes([self.data[rec + 14], self.data[rec + 15]]);

        let ct = match card_type_val {
            1 => CardType::Live,
            2 => CardType::Energy,
            _ => CardType::Member,
        };

        let base_heart = make_base_heart_from_u8(base_heart_val);
        let blade_heart = make_blade_heart_from_u8(blade_heart_val);
        let special_heart = make_special_heart_from_u8(special_heart_val);

        let mut abilities = Vec::new();
        if ability_ref_val > 0 {
            abilities.push(AbilityRef::index(ability_ref_val));
        }

        Card {
            card_no: String::from(self.str_at(card_no_off)).into(),
            name: String::from(self.str_at(name_off)).into(),
            card_type: ct,
            series: series_from_u8(series_val),
            group: group_from_u8(group_val),
            unit: if unit_val != 0 {
                Some(ArcStr::from(""))
            } else {
                None
            },
            cost: if cost_val > 0 {
                Some(cost_val as u32)
            } else {
                None
            },
            base_heart,
            blade_heart,
            blade: blade_val as u32,
            score: if score_val > 0 {
                Some(score_val as u32)
            } else {
                None
            },
            need_heart: None,
            special_heart,
            abilities,
        }
    }
}

fn make_base_heart_from_u8(v: u8) -> Option<BaseHeart> {
    let color = match v {
        1 => HeartColor::Heart01,
        2 => HeartColor::Heart02,
        3 => HeartColor::Heart03,
        4 => HeartColor::Heart04,
        5 => HeartColor::Heart05,
        _ => return None,
    };
    let mut hearts = HeartMap::new();
    hearts.insert(color, 1);
    Some(BaseHeart { hearts })
}

fn make_blade_heart_from_u8(v: u8) -> Option<BladeHeart> {
    let color = match v {
        1 => HeartColor::Heart01,
        2 => HeartColor::Heart02,
        3 => HeartColor::Heart03,
        4 => HeartColor::Heart04,
        5 => HeartColor::Heart05,
        _ => return None,
    };
    let mut hearts = HeartMap::new();
    hearts.insert(color, 1);
    Some(BladeHeart { hearts })
}

fn make_special_heart_from_u8(v: u8) -> Option<SpecialHeart> {
    let color = match v {
        1 => HeartColor::Heart01,
        2 => HeartColor::Heart02,
        3 => HeartColor::Heart03,
        4 => HeartColor::Heart04,
        5 => HeartColor::Heart05,
        _ => return None,
    };
    let mut hearts = HeartMap::new();
    hearts.insert(color, 1);
    Some(SpecialHeart { hearts })
}

fn series_from_u8(v: u8) -> Box<str> {
    match v {
        1 => Box::from("ラブライブ！"),
        2 => Box::from("ラブライブ！サンシャイン!!"),
        3 => Box::from("ラブライブ！虹ヶ咲学園スクールアイドル同好会"),
        4 => Box::from("ラブライブ！スーパースター!!"),
        5 => Box::from("蓮ノ空女学院スクールアイドルクラブ"),
        _ => Box::from(""),
    }
}

fn group_from_u8(v: u8) -> Box<str> {
    match v {
        1 => Box::from("μ's"),
        2 => Box::from("Aqours"),
        3 => Box::from("虹ヶ咲"),
        4 => Box::from("Liella!"),
        5 => Box::from("蓮ノ空"),
        _ => Box::from(""),
    }
}

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

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    let card_db = DsCardDb::new(CARDS_DB);

    let modes = ["VS AI", "2 Player", "AI vs AI", "Run Tests"];
    let mode_idx = select(&mut display, &mut input, &modes, "Mode");
    let vs_ai = mode_idx == 0;
    let ai_vs_ai = mode_idx == 2;
    let run_tests = mode_idx == 3;

    if run_tests {
        run_on_device_tests_ds(&mut display, &mut input, &card_db);
        return;
    }

    let deck_count = card_db.deck_count() as usize;
    let mut deck_names: Vec<&str> = Vec::new();
    for i in 0..deck_count {
        deck_names.push(card_db.deck_name(i));
    }

    let deck1_idx = select(&mut display, &mut input, &deck_names, "Your Deck");
    let deck2_idx = if vs_ai || ai_vs_ai {
        rng::rand_range(deck_count)
    } else {
        select(&mut display, &mut input, &deck_names, "P2 Deck")
    };

    display.clear();
    display.println("Loading...");
    display.swap_buffers();

    // Collect unique cards from both decks and build Card structs from binary
    let mut seen: hashbrown::HashSet<u16, DsHasher> = hashbrown::HashSet::with_hasher(DsHasher(0));
    let mut cards: Vec<Card> = Vec::new();
    let mut nums1: Vec<String> = Vec::new();
    let mut nums2: Vec<String> = Vec::new();

    let cc1 = card_db.deck_card_count(deck1_idx) as usize;
    for i in 0..cc1 {
        let ci = card_db.deck_card_index(deck1_idx, i);
        nums1.push(String::from(card_db.card_no(ci)));
        if seen.insert(ci) {
            cards.push(card_db.build_card(ci));
        }
    }
    display.println(&format!("1: {} cards", nums1.len()));
    display.swap_buffers();

    let cc2 = card_db.deck_card_count(deck2_idx) as usize;
    for i in 0..cc2 {
        let ci = card_db.deck_card_index(deck2_idx, i);
        nums2.push(String::from(card_db.card_no(ci)));
        if seen.insert(ci) {
            cards.push(card_db.build_card(ci));
        }
    }
    display.println(&format!("2: {} cards", nums2.len()));
    display.swap_buffers();

    let mut card_map: hashbrown::HashMap<String, Card, DsHasher> =
        hashbrown::HashMap::with_hasher(DsHasher(0));
    for c in cards.into_iter() {
        let key = c.card_no.to_string();
        if !card_map.contains_key(&key) {
            card_map.insert(key, c);
        }
    }
    display.println(&format!("map: {}", card_map.len()));
    display.swap_buffers();

    let cards: Vec<Card> = card_map.into_values().collect();
    let mut db = Arc::new(CardDatabase::load_or_create(cards));
    display.println(&format!("db ok"));
    display.swap_buffers();

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
    display.println(&format!("decks built"));
    display.swap_buffers();

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);
    display.println(&format!("players ready"));
    display.swap_buffers();

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);
    display.println(&format!("game ready: {:?}", gs.current_phase));
    display.swap_buffers();

    loop {
        display.println(&format!(
            "Loop: ph={:?} ch={} rs={:?}",
            gs.current_phase,
            gs.has_pending_choice(),
            gs.game_result
        ));
        display.swap_buffers();

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

        if gs.has_pending_choice() {
            if !handle_choice(&mut display, &mut input, &mut gs) {
                break;
            }
            continue;
        }

        let actions = game_setup::generate_possible_actions(&gs);
        display.println(&format!("actions: {}", actions.len()));
        display.swap_buffers();

        if actions.is_empty() {
            display.println("empty → advance");
            display.swap_buffers();
            TurnEngine::advance_phase(&mut gs);
            wait_frames(30);
            continue;
        }

        // Show first action for debugging
        if let Some(a) = actions.first() {
            display.println(&format!("act0: {:?}", a.action_type));
            display.swap_buffers();
        }

        let is_ai = ai_vs_ai || (vs_ai && gs.active_player().id != gs.player1.id);
        display.println(&format!("is_ai={} act={}", is_ai, gs.active_player().id));
        display.swap_buffers();

        let ok = if is_ai {
            ai_turn(&mut display, &mut gs, &actions)
        } else {
            human_turn(&mut display, &mut input, &mut gs, &actions)
        };
        if !ok {
            break;
        }
        // In RPS, after P1 picks, let AI pick for P2 immediately
        if gs.current_phase == Phase::RockPaperScissors
            && gs.player1_rps_choice.is_some()
            && gs.player2_rps_choice.is_none()
        {
            display.println("AI picks RPS...");
            display.swap_buffers();
            let ai_actions = game_setup::generate_possible_actions(&gs);
            if !ai_actions.is_empty() {
                ai_turn(&mut display, &mut gs, &ai_actions);
            }
        }
        wait_frames(15);
        settle_auto(&mut gs);
    }
}

// ── DS On-Device Tests ──────────────────────────────────────────

fn run_on_device_tests_ds(display: &mut Display, input: &mut Input, card_db: &DsCardDb) {
    display.clear();
    display.println("=== DS ON-DEVICE TESTS ===");
    display.swap_buffers();
    wait_frames(20);

    let mut passed = 0u32;
    let mut failed = 0u32;

    // Test 1: Card count from binary DB
    let cc = card_db.card_count();
    display.println(&alloc::format!("CARDS: {} in RBCD", cc));
    if cc > 0 {
        passed += 1;
    } else {
        failed += 1;
    }
    display.swap_buffers();
    wait_frames(20);

    // Test 2: Deck count
    let dc = card_db.deck_count();
    display.println(&alloc::format!("DECKS: {}", dc));
    if dc >= 2 {
        passed += 1;
    } else {
        failed += 1;
        display.println("(need 2+ for AI test)");
    }
    display.swap_buffers();
    wait_frames(20);

    // Test 3: Verify ability refs on first few cards
    let mut ab_count = 0u32;
    for i in 0..cc.min(50) {
        let c = card_db.build_card(i);
        if !c.abilities.is_empty() {
            ab_count += 1;
        }
    }
    display.println(&alloc::format!("ABILITIES: {} cards (of 50)", ab_count));
    if ab_count > 0 {
        passed += 1;
    } else {
        failed += 1;
    }
    display.swap_buffers();
    wait_frames(20);

    // Test 4: AI vs AI (5 turns) if enough decks
    if dc >= 2 {
        display.println("AI PLAY: 5 turns...");
        display.swap_buffers();
        wait_frames(10);
        match test_ai_vs_ai_ds(card_db, 0, 1) {
            Ok(n) => {
                display.println(&alloc::format!("AI PLAY: {} actions (OK)", n));
                passed += 1;
            }
            Err(e) => {
                display.println(&alloc::format!("AI PLAY: {}", e));
                failed += 1;
            }
        }
        display.swap_buffers();
        wait_frames(30);
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
        if input.just_pressed(Button::Start) || input.just_pressed(Button::B) {
            break;
        }
        wait_frames(2);
    }
}

fn test_ai_vs_ai_ds(card_db: &DsCardDb, d1: u16, d2: u16) -> Result<usize, alloc::string::String> {
    let mut seen: hashbrown::HashSet<u16, DsHasher> = hashbrown::HashSet::with_hasher(DsHasher(0));
    let mut cards: Vec<Card> = Vec::new();
    let mut nums1: Vec<String> = Vec::new();
    let mut nums2: Vec<String> = Vec::new();
    let cc1 = card_db.deck_card_count(d1 as usize) as usize;
    for i in 0..cc1 {
        let ci = card_db.deck_card_index(d1 as usize, i);
        nums1.push(String::from(card_db.card_no(ci)));
        if seen.insert(ci) {
            cards.push(card_db.build_card(ci));
        }
    }
    let cc2 = card_db.deck_card_count(d2 as usize) as usize;
    for i in 0..cc2 {
        let ci = card_db.deck_card_index(d2 as usize, i);
        nums2.push(String::from(card_db.card_no(ci)));
        if seen.insert(ci) {
            cards.push(card_db.build_card(ci));
        }
    }

    let mut db = Arc::new(CardDatabase::load_or_create(cards));
    let mut pd1 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums1)
        .map_err(|e| alloc::format!("D1:{}", e))?;
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    let mut pd2 = deck_builder::DeckBuilder::build_deck_from_database(&mut db, nums2)
        .map_err(|e| alloc::format!("D2:{}", e))?;
    deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "P2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);
    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    let mut count = 0usize;
    let mut turns = 0u32;
    while gs.game_result == GameResult::Ongoing && turns < 10 {
        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            turns += 1;
            continue;
        }
        let a = acts[0].clone();
        let p = a.parameters.clone();
        let _ = TurnEngine::execute_main_phase_action(
            &mut gs,
            &a.action_type,
            p.as_ref().and_then(|x| x.card_id),
            p.as_ref().and_then(|x| x.card_indices.clone()),
            p.as_ref()
                .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
            p.as_ref().and_then(|x| x.use_baton_touch),
        );
        count += 1;
        while gs.game_result == GameResult::Ongoing && game_setup::is_automatic_phase(&gs) {
            TurnEngine::advance_phase(&mut gs);
            turns += 1;
        }
        if gs.current_phase == Phase::Active || gs.current_phase == Phase::Draw {
            turns += 1;
        }
    }
    Ok(count)
}

// ── End of tests ─────────────────────────────────────────────────

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
    const VISIBLE_ACTIONS: usize = 10;
    loop {
        display.clear();
        display.println(&alloc::format!(
            "Turn {} | {:?}",
            gs.turn_number,
            gs.current_phase
        ));
        let p1 = &gs.player1;
        let p2 = &gs.player2;
        let is_p1 = gs.active_player().id == "p1";
        display.println(&alloc::format!(
            "{}P1 h:{} e:{} dk:{}",
            if is_p1 { ">>" } else { "  " },
            p1.hand.cards.len(),
            p1.energy_zone.active_count(),
            p1.main_deck.cards.len()
        ));
        display.println(&alloc::format!(
            "{}P2 h:{} e:{} dk:{}",
            if !is_p1 { ">>" } else { "  " },
            p2.hand.cards.len(),
            p2.energy_zone.active_count(),
            p2.main_deck.cards.len()
        ));
        display.println("--------------------------------");
        let end = actions.len().min(VISIBLE_ACTIONS);
        for i in 0..end {
            let p = if i == sel { ">" } else { " " };
            let line = actions[i].description.lines().next().unwrap_or("");
            display.println(&alloc::format!("{p}[{}] {}", i, line));
        }
        if actions.len() > end {
            display.println(&alloc::format!("  .. {} more", actions.len() - end));
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
    for _ in 0..10 {
        if gs.has_pending_choice() || gs.game_result != GameResult::Ongoing {
            break;
        }
        if game_setup::is_automatic_phase(gs) {
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
