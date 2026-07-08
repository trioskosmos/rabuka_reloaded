// Rabuka 3DS — interactive card game with direct framebuffer text rendering.
// Uses the 3DS shared system font (fontGetSystemFont) which includes full
// Japanese on JPN/USA/EUR consoles. No font files or extra libraries needed.
//
// RAM constraints (measured with --release on desktop, 3DS ARM11 ~10x slower):
//   sizeof(Ability) = 19968 bytes  // 20 KB each — dozens of Option<Box<...>> fields
//   sizeof(Card) = 504 bytes
//   2280 cards → ~1.1 MB
//   cards.bin: 2100 KB (MessagePack, 33% smaller than JSON)
//   abilities.json: 1453 KB on disk
//
// Loading strategy (abilities deferred to after deck selection):
//   1) Read cards.bin via rmp_serde + YieldReader → Vec<Card> (no abilities, ~2s)
//   2) Select two player decks → ~120 unique card_nos
//   3) Read abilities.json, build ability map, attach ONLY for deck cards
//      (~120 clones × 20KB = ~3MB instead of 33MB for all 1727)
//   4) Build game and play
// This avoids the 3DS watchdog (no extended JSON parsing) and saves ~30MB RAM.
//
// TEXT RENDERING:
// Renders text directly to RGB565 framebuffers using fontGetSystemFont().
// The system font texture sheets (A4 format, 8x8 tiled) are in shared memory
// and read via CPU-side tiled texture decoding. No GPU or extra libraries.
// ~7ms per frame at 268MHz (memset + half-scale glyph blit for 600 chars).
// See ctru_shim.c for detailed memory breakdown.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_builder::DeckBuilder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

/// 3DS system tick rate: 268.12 MHz (ARM11)
const TICK_HZ: u64 = 268_120_000;
/// Print debug timing every N frames (0 = disabled)
const DBG_EVERY_N: u64 = 60;

/// Reader wrapper that calls aptMainLoop() every `threshold` bytes without
/// any GPU buffer operations. Keeps the 3DS OS alive during long deserialization
/// without the overhead/cost of _3ds_keep_alive().
#[cfg(feature = "3ds")]
struct YieldReader<R> {
    inner: R,
    threshold: usize,
    counter: usize,
}

#[cfg(feature = "3ds")]
impl<R: Read> Read for YieldReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.counter += n;
        if self.counter >= self.threshold {
            self.counter = 0;
            if unsafe { _3ds_main_loop() } == 0 {
                // App should exit; return empty read to signal EOF
                return Ok(0);
            }
        }
        Ok(n)
    }
}

// dprintln! — game output on BOTTOM screen (action list).
// Also sends to debug console via svcOutputDebugString (3dslink).
#[cfg(feature = "3ds")]
macro_rules! dprintln {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\n\0", msg);
        unsafe { _3ds_debug_print(s.as_ptr()); }
        unsafe { _3ds_text_add_bot(s.as_ptr()); }
    }};
}

// tprintln! — debug output on TOP screen (timing/status).
// Appends to top text buffer, rendered in _3ds_swap_buffers().
#[cfg(feature = "3ds")]
macro_rules! tprintln {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\n\0", msg);
        unsafe { _3ds_debug_print(s.as_ptr()); }
        unsafe { _3ds_text_add_top(s.as_ptr()); }
    }};
}

#[cfg(feature = "3ds")]
fn ticks_to_ms(ticks: u64) -> f64 {
    (ticks as f64) / (TICK_HZ as f64) * 1000.0
}

#[derive(Clone)]
struct CardAtlas {
    /// Map card_no -> (atlas_filename, index)
    map: HashMap<String, (String, usize)>,
}

impl CardAtlas {
    fn load() -> Self {
        let path = Path::new("romfs:/cards_manifest.json");
        let mut f = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                return CardAtlas {
                    map: HashMap::new(),
                }
            }
        };
        let mut s = String::new();
        if f.read_to_string(&mut s).is_err() {
            return CardAtlas {
                map: HashMap::new(),
            };
        }
        let raw: HashMap<String, serde_json::Value> = match serde_json::from_str(&s) {
            Ok(m) => m,
            Err(_) => {
                return CardAtlas {
                    map: HashMap::new(),
                }
            }
        };
        let map = raw
            .into_iter()
            .filter_map(|(k, v)| {
                let atlas = v.get("atlas")?.as_str()?.to_string();
                let index = v.get("index")?.as_u64()? as usize;
                Some((k, (atlas, index)))
            })
            .collect();
        CardAtlas { map }
    }

    fn lookup(&self, card_no: &str) -> Option<&(String, usize)> {
        self.map.get(card_no)
    }
}

#[cfg(feature = "3ds")]
enum Step {
    ReadCardsBin,
    ParseCards(Vec<u8>),
    LoadDecks(Vec<Card>),
    Play(
        GameState,
        usize,
        Vec<game_setup::Action>,
        bool,
        bool,
        CardAtlas,
    ),
    Done(Result<(), String>),
}

#[cfg(feature = "3ds")]
#[no_mangle]
pub unsafe extern "C" fn pthread_atfork(
    _prepare: Option<unsafe extern "C" fn()>,
    _parent: Option<unsafe extern "C" fn()>,
    _child: Option<unsafe extern "C" fn()>,
) -> i32 {
    0
}

#[cfg(feature = "3ds")]
fn main() {
    std::panic::set_hook(Box::new(|info| {
        unsafe {
            _3ds_clear_both();
        }

        let payload: String = if let Some(s) = info.payload().downcast_ref::<&str>() {
            format!("PANIC!\n{}\n", s)
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            format!("PANIC!\n{}\n", s)
        } else {
            "PANIC!\n(no message)\n".to_string()
        };
        let loc_str: String = info
            .location()
            .map(|l| format!("at {}:{}\n", l.file(), l.line()))
            .unwrap_or_default();

        unsafe {
            let debug = format!("{}{}\0", payload, loc_str);
            _3ds_debug_print(debug.as_ptr());
            let s = format!("{}\0", payload);
            _3ds_text_add_top(s.as_ptr());
            if !loc_str.is_empty() {
                let s = format!("{}\0", loc_str);
                _3ds_text_add_top(s.as_ptr());
            }
        }

        loop {
            unsafe {
                _3ds_swap_buffers();
            }
        }
    }));
    unsafe {
        _3ds_init();
    }

    let mut frame: u64 = 0;
    let mut step = Step::ReadCardsBin;

    while unsafe { _3ds_main_loop() != 0 } {
        let tick_start = unsafe { _3ds_system_tick() };

        unsafe {
            _3ds_scan_input();
        }
        let keys = unsafe { _3ds_keys_down() };
        if keys & 0x00000008 != 0 {
            break;
        }

        if frame >= DBG_EVERY_N && frame % DBG_EVERY_N == 0 {
            tprintln!("[DBG] frame={} step={}", frame, step_name(&step));
        }
        let current_step = step_name(&step);
        frame += 1;

        step = match step {
            Step::ReadCardsBin => {
                let t0 = unsafe { _3ds_system_tick() };
                dprintln!("[1/2] Reading cards.bin...");
                let path = Path::new("romfs:/cards.bin");
                match File::open(path).and_then(|mut f| {
                    let mut v = Vec::new();
                    f.read_to_end(&mut v).map(|_| v)
                }) {
                    Ok(v) => {
                        let t1 = unsafe { _3ds_system_tick() };
                        dprintln!("  {} B ({} ms)", v.len(), ticks_to_ms(t1 - t0));
                        Step::ParseCards(v)
                    }
                    Err(e) => Step::Done(Err(format!("Read: {}", e))),
                }
            }
            Step::ParseCards(bytes) => {
                let t0 = unsafe { _3ds_system_tick() };
                dprintln!("[2/3] Deserializing cards...");
                let reader = YieldReader {
                    inner: std::io::Cursor::new(&bytes),
                    threshold: 8192,
                    counter: 0,
                };
                match rmp_serde::from_read::<_, HashMap<String, Card>>(reader) {
                    Ok(map) => {
                        let t1 = unsafe { _3ds_system_tick() };
                        let cards: Vec<_> = map.into_values().collect();
                        dprintln!("  {} cards ({} ms)", cards.len(), ticks_to_ms(t1 - t0));
                        drop(bytes);
                        Step::LoadDecks(cards)
                    }
                    Err(e) => Step::Done(Err(format!("Parse: {}", e))),
                }
            }
            Step::LoadDecks(cards) => {
                let t0 = unsafe { _3ds_system_tick() };
                dprintln!("[3/3] Building game...");

                let mut db = Arc::new(CardDatabase::load_or_create(cards));

                // Build decks
                let decks =
                    match DeckParser::parse_all_decks_from_directory(Path::new("romfs:/decks/")) {
                        Ok(v) if !v.is_empty() => v,
                        _ => {
                            step = Step::Done(Err("No decks".into()));
                            continue;
                        }
                    };
                let nums = DeckParser::deck_list_to_card_numbers(&decks[0]);
                let (mut pd1, mut pd2) = match (
                    DeckBuilder::build_deck_from_database(&mut db, nums.clone()),
                    DeckBuilder::build_deck_from_database(&mut db, nums),
                ) {
                    (Ok(pd1), Ok(pd2)) => (pd1, pd2),
                    _ => {
                        step = Step::Done(Err("Failed to build decks".into()));
                        continue;
                    }
                };
                pd1.shuffle_main_deck();
                pd1.shuffle_energy_deck();
                pd2.shuffle_main_deck();
                pd2.shuffle_energy_deck();
                let decks_t = unsafe { _3ds_system_tick() };
                dprintln!("  Decks built ({} ms)", ticks_to_ms(decks_t - t0));

                // 3. Collect unique card_nos from both decks
                let mut deck_nos: HashSet<String> = HashSet::new();
                for cid in pd1
                    .main_deck
                    .iter()
                    .chain(pd1.energy_deck.iter())
                    .chain(pd2.main_deck.iter())
                    .chain(pd2.energy_deck.iter())
                {
                    if let Some(card) = db.get_card(*cid) {
                        deck_nos.insert(card.card_no.clone());
                    }
                }

                // 4. Attach abilities ONLY for deck cards
                let ab_path = Path::new("romfs:/abilities.json");
                match File::open(ab_path).and_then(|mut f| {
                    let mut v = String::new();
                    f.read_to_string(&mut v).map(|_| v)
                }) {
                    Ok(json) => {
                        let attach_t0 = unsafe { _3ds_system_tick() };
                        if let Ok(abilities_data) = CardLoader::load_abilities_from_str(&json) {
                            let ability_map = CardLoader::build_abilities_map(&abilities_data);
                            drop(abilities_data);
                            let db_inner = Arc::make_mut(&mut db);
                            for (_, card) in db_inner.cards.iter_mut() {
                                if deck_nos.contains(&card.card_no) {
                                    if let Some(ab) = ability_map.get(&card.card_no) {
                                        card.abilities = ab.clone();
                                    }
                                }
                            }
                            let attach_t1 = unsafe { _3ds_system_tick() };
                            dprintln!(
                                "  Abilities attached ({} ms, {} deck cards)",
                                ticks_to_ms(attach_t1 - attach_t0),
                                deck_nos.len()
                            );
                        } else {
                            dprintln!("  abilities.json parse failed");
                        }
                    }
                    Err(e) => dprintln!("  abilities.json read failed: {}", e),
                }

                // 5. Add energy cards and build players
                DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
                DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();

                let mut p1 = Player::new("p1".into(), "P1".into(), true);
                p1.set_main_deck(pd1.main_deck);
                p1.set_energy_deck(pd1.energy_deck);
                let mut p2 = Player::new("p2".into(), "P2".into(), false);
                p2.set_main_deck(pd2.main_deck);
                p2.set_energy_deck(pd2.energy_deck);

                let mut gs = GameState::new(p1, p2, db);
                game_setup::setup_game(&mut gs);
                let atlas = CardAtlas::load();
                unsafe {
                    _3ds_board_enable(true);
                }
                let t1 = unsafe { _3ds_system_tick() };
                dprintln!("  Game ready ({} ms)", ticks_to_ms(t1 - t0));
                Step::Play(gs, 0, Vec::new(), true, true, atlas)
            }
            Step::Play(mut gs, mut cur, mut acts_cache, mut dirty, mut redraw, ref atlas) => {
                // Input handling
                if keys & 0x00000040 != 0 {
                    // DPAD_UP
                    if cur > 0 {
                        cur -= 1;
                        redraw = true;
                    }
                } else if keys & 0x00000080 != 0 {
                    // DPAD_DOWN
                    if cur + 1 < acts_cache.len() {
                        cur += 1;
                        redraw = true;
                    }
                }

                // SELECT toggles opponent board view
                if keys & 0x00000004 != 0 {
                    unsafe {
                        _3ds_board_toggle_side();
                    }
                    redraw = true;
                }

                // A button executes selected action
                if keys & 0x00000001 != 0 && cur < acts_cache.len() {
                    let action = acts_cache[cur].clone();
                    let p = action.parameters.clone();
                    let result = turn::TurnEngine::execute_main_phase_action(
                        &mut gs,
                        &action.action_type,
                        p.as_ref().and_then(|x| x.card_id),
                        p.as_ref().and_then(|x| x.card_indices.clone()),
                        p.as_ref()
                            .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
                        p.as_ref().and_then(|x| x.use_baton_touch),
                    );
                    if let Err(ref e) = result {
                        unsafe {
                            _3ds_debug_print(format!("[ERR] {}\n\0", e).as_ptr());
                        }
                    }
                    gs.reset_loop_detection();
                    gs.reset_loop_detection();
                    cur = 0;
                    dirty = true;
                    redraw = true;
                }

                let n2 = acts_cache.len();
                if n2 > 0 && cur >= n2 {
                    cur = n2 - 1;
                }

                let auto = !gs.has_pending_choice()
                    && gs.game_result == GameResult::Ongoing
                    && game_setup::is_automatic_phase(&gs);
                if auto {
                    settle_3ds(&mut gs);
                    cur = 0;
                    dirty = true;
                }

                if dirty || redraw {
                    acts_cache = game_setup::generate_possible_actions(&gs);

                    let show_opp = unsafe { _3ds_board_is_opponent() };
                    let view_player = if show_opp { &gs.player2 } else { &gs.player1 };
                    let ap = gs.active_player();

                    // ---- Set up board slots ----
                    unsafe {
                        _3ds_clear_top();
                    }

                    // Top screen: stats + turn info
                    unsafe {
                        _3ds_text_add_top(
                            format!(
                                "Turn {} | {:?} | P{}\n\0",
                                gs.turn_number,
                                gs.current_phase,
                                if ap.id == gs.player1.id { "1" } else { "2" }
                            )
                            .as_ptr(),
                        );
                    }

                    // Helper: get card_no from card ID
                    let card_no = |cid: i16| -> Option<String> {
                        gs.card_database.get_card(cid).map(|c| c.card_no.clone())
                    };

                    // Helper: set a card slot (returns card_no if set)
                    let set_slot =
                        |slot_fn: unsafe extern "C" fn(i32, bool, *const u8, i32, bool, bool),
                         slot_i: i32,
                         cid: i16,
                         landscape: bool,
                         tapped: bool|
                         -> Option<String> {
                            if cid == -1 {
                                unsafe {
                                    slot_fn(slot_i, false, std::ptr::null(), 0, false, false);
                                }
                                return None;
                            }
                            let cn = card_no(cid);
                            if let Some(ref no) = cn {
                                if let Some((ref atl, idx)) = atlas.lookup(no) {
                                    let c_str =
                                        std::ffi::CString::new(atl.as_bytes()).unwrap_or_default();
                                    unsafe {
                                        slot_fn(
                                            slot_i,
                                            true,
                                            c_str.as_ptr() as *const u8,
                                            *idx as i32,
                                            landscape,
                                            tapped,
                                        );
                                    }
                                    return Some(no.clone());
                                }
                            }
                            unsafe {
                                slot_fn(slot_i, false, std::ptr::null(), 0, false, false);
                            }
                            cn
                        };

                    // Helper: check if a card is tapped/waited
                    let is_tapped = |cid: i16| -> bool {
                        gs.mods.orientation_modifiers.get(&cid).map(|s| s.as_str()) == Some("Wait")
                    };

                    // ---- STAGE ----
                    let st = &view_player.stage.stage;
                    for i in 0..3 {
                        let cid = st[i];
                        let tapped = if cid != -1 { is_tapped(cid) } else { false };
                        set_slot(_3ds_board_set_stage, i as i32, cid, false, tapped);
                    }

                    // Top screen: stage info
                    unsafe {
                        let s0 = if st[0] == -1 {
                            "-".into()
                        } else {
                            card_no(st[0]).unwrap_or("?".into())
                        };
                        let s1 = if st[1] == -1 {
                            "-".into()
                        } else {
                            card_no(st[1]).unwrap_or("?".into())
                        };
                        let s2 = if st[2] == -1 {
                            "-".into()
                        } else {
                            card_no(st[2]).unwrap_or("?".into())
                        };
                        _3ds_text_add_top(
                            format!(
                                "P{} St:{} {} {}\n\0",
                                if view_player.id == gs.player1.id {
                                    "1"
                                } else {
                                    "2"
                                },
                                s0,
                                s1,
                                s2
                            )
                            .as_ptr(),
                        );
                    }

                    // ---- LIVE ZONE ----
                    let live_cards = &view_player.live_card_zone.cards;
                    for i in 0..3.min(live_cards.len()) {
                        let cid = live_cards[i];
                        let tapped = if cid != -1 { is_tapped(cid) } else { false };
                        set_slot(_3ds_board_set_live, i as i32, cid, true, tapped);
                    }
                    for i in live_cards.len()..3 {
                        unsafe {
                            _3ds_board_set_live(i as i32, false, std::ptr::null(), 0, false, false);
                        }
                    }
                    for i in live_cards.len()..3 {
                        unsafe {
                            _3ds_board_set_live(i as i32, false, std::ptr::null(), 0, false, false);
                        }
                    }

                    // ---- ENERGY ----
                    let energy_cards = &view_player.energy_zone.cards;
                    let e_count = energy_cards.len().min(30);
                    unsafe {
                        _3ds_board_set_energy_count(e_count as i32);
                    }
                    for (i, cid) in energy_cards.iter().enumerate().take(30) {
                        let tapped = is_tapped(*cid);
                        set_slot(_3ds_board_set_energy, i as i32, *cid, false, tapped);
                    }

                    // ---- HAND ----
                    let hand_cards = &view_player.hand.cards;
                    let h_count = hand_cards.len().min(15);
                    unsafe {
                        _3ds_board_set_hand_count(h_count as i32);
                    }
                    for (i, cid) in hand_cards.iter().enumerate().take(15) {
                        set_slot(_3ds_board_set_hand, i as i32, *cid, false, false);
                    }

                    // ---- UTILITY COUNTS ----
                    unsafe {
                        _3ds_board_set_utility(
                            view_player.main_deck.cards.len() as i32,
                            view_player.energy_deck.cards.len() as i32,
                            view_player.waitroom.cards.len() as i32,
                            view_player.success_live_card_zone.cards.len() as i32,
                        );
                    }

                    // ---- TOP SCREEN: stats + selected card info ----
                    unsafe {
                        _3ds_text_add_top(
                            format!(
                                "H:{} E:{}/{} D:{} W:{} L:{}\n\0",
                                view_player.hand.cards.len(),
                                view_player.energy_zone.active_count(),
                                view_player.energy_zone.cards.len(),
                                view_player.main_deck.cards.len(),
                                view_player.waitroom.cards.len(),
                                view_player.success_live_card_zone.cards.len()
                            )
                            .as_ptr(),
                        );
                    }

                    // Selected card info (from action cursor)
                    if cur < acts_cache.len() {
                        let act = &acts_cache[cur];
                        let desc = &act.description;
                        if let Some(ref p) = act.parameters {
                            if let Some(cid) = p.card_id {
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    unsafe {
                                        _3ds_text_add_top(
                                            format!(
                                                "[{}] {} | {}\n\0",
                                                card.card_no, card.name, desc
                                            )
                                            .as_ptr(),
                                        );
                                    }
                                } else {
                                    unsafe {
                                        _3ds_text_add_top(format!("{}\n\0", desc).as_ptr());
                                    }
                                }
                            } else {
                                unsafe {
                                    _3ds_text_add_top(format!("{}\n\0", desc).as_ptr());
                                }
                            }
                        } else {
                            unsafe {
                                _3ds_text_add_top(format!("{}\n\0", desc).as_ptr());
                            }
                        }
                    }

                    // Frame timing
                    let gen_ms = ticks_to_ms(unsafe { _3ds_system_tick() });
                    unsafe {
                        _3ds_text_add_top(format!("f:{} {}ms\n\0", frame, gen_ms as u64).as_ptr());
                    }

                    dirty = false;
                    redraw = false;
                }
                Step::Play(gs, cur, acts_cache, dirty, redraw, atlas.clone())
            }
            Step::Done(ref r) => {
                unsafe {
                    _3ds_clear_both();
                }
                match r {
                    Ok(_) => unsafe {
                        _3ds_text_add_bot("Done! Press START.\n\0".as_ptr());
                    },
                    Err(e) => unsafe {
                        let s = format!("ERROR: {}\n\0", e);
                        _3ds_text_add_bot(s.as_ptr());
                    },
                }
                if keys & 0x00000008 != 0 {
                    break;
                }
                Step::Done(match r {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.clone()),
                })
            }
        };
        let tick_end = unsafe { _3ds_system_tick() };
        let frame_ms = ticks_to_ms(tick_end - tick_start);
        if frame_ms > 33.0 {
            tprintln!(
                "[WARN] frame {}: {} ms (step: {})",
                frame,
                frame_ms,
                current_step
            );
        }
        unsafe {
            _3ds_swap_buffers();
        }
    }
    unsafe {
        _3ds_exit();
    }
}

/// 3DS-native settle: same logic as game_setup::settle_single_player_state but
/// calls aptMainLoop() every 10 iterations to keep the OS watchdog happy, and
/// avoids ALL eprintln!/log calls (which can deadlock the GPU console renderer).
#[cfg(feature = "3ds")]
fn settle_3ds(gs: &mut GameState) {
    let mut iters = 0u32;
    loop {
        iters += 1;
        // Yield to OS every 10 iterations to avoid watchdog timeout
        if iters % 10 == 0 {
            if unsafe { _3ds_main_loop() } == 0 {
                return;
            }
        }
        if iters > 500 {
            break;
        }
        if gs.has_pending_choice() {
            break;
        }
        if gs.game_result != GameResult::Ongoing {
            break;
        }
        if game_setup::is_automatic_phase(gs) {
            turn::TurnEngine::advance_phase(gs);
        } else {
            break;
        }
    }
}

#[cfg(feature = "3ds")]
fn step_name(s: &Step) -> &'static str {
    match s {
        Step::ReadCardsBin => "ReadCards",
        Step::ParseCards(_) => "ParseCards",
        Step::LoadDecks(_) => "LoadDecks",
        Step::Play(_, _, _, _, _, _) => "Play",
        Step::Done(_) => "Done",
    }
}

extern "C" {
    fn _3ds_init();
    fn _3ds_main_loop() -> i32;
    fn _3ds_exit();
    fn _3ds_swap_buffers();
    fn _3ds_scan_input();
    fn _3ds_keys_down() -> u32;
    fn _3ds_system_tick() -> u64;
    fn _3ds_debug_print(msg: *const u8);
    fn _3ds_tdbg(msg: *const u8);
    fn _3ds_clear_console();
    fn _3ds_clear_both();
    fn _3ds_clear_top();
    fn _3ds_text_add_top(msg: *const u8);
    fn _3ds_text_add_bot(msg: *const u8);
    fn _3ds_bot_line_height() -> f32;

    // Board API
    fn _3ds_board_enable(on: bool);
    fn _3ds_board_toggle_side();
    fn _3ds_board_is_opponent() -> bool;
    fn _3ds_board_clear_cache();
    fn _3ds_board_set_stage(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_live(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_energy(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_energy_count(count: i32);
    fn _3ds_board_set_hand(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_hand_count(count: i32);
    fn _3ds_board_set_utility(deck: i32, edeck: i32, discard: i32, success: i32);
    fn _3ds_board_set_selection(slot: i32, slot_type: i32);
}

#[cfg(not(feature = "3ds"))]
fn main() {
    println!("Desktop mode - use: cargo run --bin harness");
}
