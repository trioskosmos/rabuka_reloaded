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

// Desktop mode uses none of these; suppress warnings
#![cfg_attr(not(feature = "3ds"), allow(unused_imports, dead_code))]

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_builder::DeckBuilder;
use rabuka_engine::deck_parser::DeckList;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

#[cfg(feature = "3ds")]
const TICK_HZ: u64 = 268_120_000;

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

/// Word-wrap text to fit within `max_chars` per line, breaking at spaces.
/// Inserts `\n` to split long lines. Works with multi-byte UTF-8.
#[cfg(feature = "3ds")]
fn wrap_text(s: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(s.len() + 32);
    for line in s.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let remain = chars.len() - i;
            if remain <= max_chars {
                out.extend(&chars[i..]);
                break;
            }
            let end = (i + max_chars).min(chars.len());
            if let Some(space) = chars[i..end].iter().rposition(|&c| c == ' ') {
                out.extend(&chars[i..i + space]);
                out.push('\n');
                i = i + space + 1;
            } else {
                out.extend(&chars[i..end]);
                out.push('\n');
                i = end;
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(feature = "3ds")]
fn ticks_to_ms(ticks: u64) -> f64 {
    (ticks as f64) / (TICK_HZ as f64) * 1000.0
}

#[cfg(feature = "3ds")]
#[derive(Clone)]
struct CardAtlas {
    /// Map card_no -> (atlas_filename, index)
    map: HashMap<String, (String, usize)>,
}

#[cfg(feature = "3ds")]
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
#[derive(Clone)]
enum SetupPhase {
    PickMode(usize),               // cursor: 0=sandbox, 1=vsAI
    PickDeck(usize, bool),         // cursor, vs_ai flag
    PickDeck2(usize, usize, bool), // cursor, p1_idx, vs_ai
    Loading(usize, usize, bool),   // p1_idx, p2_idx, vs_ai
}

#[cfg(feature = "3ds")]
#[derive(Clone)]
enum Step {
    ReadCardsBin,
    ParseCards(Vec<u8>),
    Setup(Vec<Card>, Vec<DeckList>, SetupPhase, bool), // +dirty flag
    Play(
        GameState,
        usize, // cursor
        Vec<game_setup::Action>,
        bool, // dirty
        bool, // redraw
        CardAtlas,
        bool, // vs_ai
        bool, // detail_mode (top screen shows card detail)
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
                        // Load deck list and go to setup
                        let decks = match DeckParser::parse_all_decks_from_directory(Path::new(
                            "romfs:/decks/",
                        )) {
                            Ok(d) => d,
                            Err(e) => {
                                step = Step::Done(Err(format!("No decks: {}", e)));
                                continue;
                            }
                        };
                        if decks.is_empty() {
                            step = Step::Done(Err("No decks found!".into()));
                            continue;
                        }
                        Step::Setup(cards, decks, SetupPhase::PickMode(0), true)
                    }
                    Err(e) => Step::Done(Err(format!("Parse: {}", e))),
                }
            }
            Step::Setup(ref cards, ref decks, ref phase, ref dirty) => {
                let n = decks.len();
                let was_dirty = *dirty;
                let new_step = match *phase {
                    SetupPhase::PickMode(cur) => unsafe {
                        if was_dirty {
                            _3ds_clear_top();
                            _3ds_text_add_top("SELECT MODE\n\0".as_ptr());
                            for (i, m) in ["Sandbox (2 players)", "VS AI"].iter().enumerate() {
                                let arrow = if i == cur { ">" } else { " " };
                                _3ds_text_add_top(format!("{} [{}] {}\n\0", arrow, i, m).as_ptr());
                            }
                            _3ds_text_add_top("\nUP/DOWN=select A=confirm\0".as_ptr());
                        }
                        if keys & 0x00000040 != 0 && cur > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur - 1),
                                true,
                            )
                        } else if keys & 0x00000080 != 0 && cur + 1 < 2 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur + 1),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            if n == 0 {
                                Step::Done(Err("No decks!".into()))
                            } else {
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::PickDeck(0, cur == 1),
                                    true,
                                )
                            }
                        } else {
                            step.clone()
                        }
                    },
                    SetupPhase::PickDeck(cur, vs_ai) => {
                        if was_dirty {
                            let label = if !vs_ai { "P1 DECK" } else { "YOUR DECK" };
                            unsafe {
                                _3ds_clear_top();
                            }
                            unsafe {
                                _3ds_text_add_top(format!("SELECT {}\n\0", label).as_ptr());
                            }
                            for i in cur.saturating_sub(6).min(n.saturating_sub(12))
                                ..(0usize.min(n) + 12).min(n)
                            {
                                let arrow = if i == cur { ">" } else { " " };
                                unsafe {
                                    _3ds_text_add_top(
                                        format!("{} {}\n\0", arrow, decks[i].name).as_ptr(),
                                    );
                                }
                            }
                            unsafe {
                                _3ds_text_add_top("\nA=select\0".as_ptr());
                            }
                        }
                        if keys & 0x00000040 != 0 && cur > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck(cur - 1, vs_ai),
                                true,
                            )
                        } else if keys & 0x00000080 != 0 && cur + 1 < n {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck(cur + 1, vs_ai),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            if vs_ai {
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::Loading(cur, cur, true),
                                    true,
                                )
                            } else {
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::PickDeck2(0, cur, false),
                                    true,
                                )
                            }
                        } else {
                            step.clone()
                        }
                    }
                    SetupPhase::PickDeck2(cur, p1_idx, vs_ai) => {
                        if was_dirty {
                            unsafe {
                                _3ds_clear_top();
                                _3ds_text_add_top("SELECT P2 DECK\n\0".as_ptr());
                            }
                            for i in cur.saturating_sub(6).min(n.saturating_sub(12))
                                ..(0usize.min(n) + 12).min(n)
                            {
                                let arrow = if i == cur { ">" } else { " " };
                                unsafe {
                                    _3ds_text_add_top(
                                        format!("{} {}\n\0", arrow, decks[i].name).as_ptr(),
                                    );
                                }
                            }
                            unsafe {
                                _3ds_text_add_top("\nA=select B=same\0".as_ptr());
                            }
                        }
                        if keys & 0x00000040 != 0 && cur > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck2(cur - 1, p1_idx, vs_ai),
                                true,
                            )
                        } else if keys & 0x00000080 != 0 && cur + 1 < n {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck2(cur + 1, p1_idx, vs_ai),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::Loading(p1_idx, cur, false),
                                true,
                            )
                        } else if keys & 0x00000002 != 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::Loading(p1_idx, p1_idx, false),
                                true,
                            )
                        } else {
                            step.clone()
                        }
                    }
                    SetupPhase::Loading(p1_idx, p2_idx, vs_ai) => {
                        let r = (|| -> Result<(GameState, CardAtlas), String> {
                            let mut db = Arc::new(CardDatabase::load_or_create(cards.clone()));
                            let nums1 = DeckParser::deck_list_to_card_numbers(&decks[p1_idx]);
                            let nums2 = if p1_idx == p2_idx {
                                nums1.clone()
                            } else {
                                DeckParser::deck_list_to_card_numbers(&decks[p2_idx])
                            };
                            let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums1)
                                .map_err(|e| format!("Deck: {}", e))?;
                            let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums2)
                                .map_err(|e| format!("Deck: {}", e))?;
                            pd1.shuffle_main_deck();
                            pd1.shuffle_energy_deck();
                            pd2.shuffle_main_deck();
                            pd2.shuffle_energy_deck();
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
                            if let Ok(json) = File::open(Path::new("romfs:/abilities.json"))
                                .and_then(|mut f| {
                                    let mut v = String::new();
                                    f.read_to_string(&mut v).map(|_| v)
                                })
                            {
                                if let Ok(a) = CardLoader::load_abilities_from_str(&json) {
                                    let am = CardLoader::build_abilities_map(&a);
                                    for (_, card) in Arc::make_mut(&mut db).cards.iter_mut() {
                                        if deck_nos.contains(&card.card_no) {
                                            if let Some(ab) = am.get(&card.card_no) {
                                                card.abilities = ab.clone();
                                            }
                                        }
                                    }
                                }
                            }
                            DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db)
                                .ok();
                            DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db)
                                .ok();
                            let mut p1 = Player::new("p1".into(), "P1".into(), true);
                            p1.set_main_deck(pd1.main_deck);
                            p1.set_energy_deck(pd1.energy_deck);
                            let mut p2 = Player::new("p2".into(), "P2".into(), false);
                            p2.set_main_deck(pd2.main_deck);
                            p2.set_energy_deck(pd2.energy_deck);
                            let mut gs = GameState::new(p1, p2, db);
                            game_setup::setup_game(&mut gs);
                            Ok((gs, CardAtlas::load()))
                        })();
                        match r {
                            Ok((gs, atlas)) => {
                                unsafe {
                                    _3ds_board_enable(true);
                                }
                                Step::Play(gs, 0, Vec::new(), true, true, atlas, vs_ai, false)
                            }
                            Err(e) => Step::Done(Err(e)),
                        }
                    }
                };
                new_step
            }
            Step::Play(
                mut gs,
                mut cur,
                mut acts_cache,
                mut dirty,
                mut redraw,
                ref atlas,
                ref vs_ai,
                mut detail_mode,
            ) => {
                // Input handling
                if detail_mode {
                    // In detail mode: DPAD scrolls text
                    if keys & 0x00000040 != 0 {
                        let sy = unsafe { _3ds_text_get_scroll_y() };
                        unsafe {
                            _3ds_text_set_scroll_y((sy - 10).max(0));
                        }
                    }
                    if keys & 0x00000080 != 0 {
                        let sy = unsafe { _3ds_text_get_scroll_y() };
                        unsafe {
                            _3ds_text_set_scroll_y(sy + 10);
                        }
                    }
                } else {
                    if keys & 0x00000040 != 0 && cur > 0 {
                        cur -= 1;
                        redraw = true;
                    } else if keys & 0x00000080 != 0 && cur + 1 < acts_cache.len() {
                        cur += 1;
                        redraw = true;
                    }
                }

                // SELECT cycles board view: player / opponent / both
                if keys & 0x00000004 != 0 {
                    unsafe {
                        _3ds_board_cycle_view();
                    }
                    redraw = true;
                }

                // X toggles card detail mode on top screen
                if keys & 0x00000400 != 0 {
                    detail_mode = !detail_mode;
                    if !detail_mode {
                        unsafe {
                            _3ds_text_set_scroll_y(0);
                        }
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

                // AI: auto-pick randomly when it's the AI player's turn with choices
                if *vs_ai && gs.has_pending_choice() {
                    let ap = gs.active_player();
                    let is_human = ap.id == gs.player1.id; // P1 is human
                    if !is_human && acts_cache.len() > 0 {
                        let ai_idx = (unsafe { _3ds_system_tick() } as usize) % acts_cache.len();
                        let action = acts_cache[ai_idx].clone();
                        let p = action.parameters.clone();
                        let _ = turn::TurnEngine::execute_main_phase_action(
                            &mut gs,
                            &action.action_type,
                            p.as_ref().and_then(|x| x.card_id),
                            p.as_ref().and_then(|x| x.card_indices.clone()),
                            p.as_ref()
                                .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
                            p.as_ref().and_then(|x| x.use_baton_touch),
                        );
                        gs.reset_loop_detection();
                        gs.reset_loop_detection();
                        cur = 0;
                        dirty = true;
                        redraw = true;
                    }
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

                    let p1 = &gs.player1;
                    let p2 = &gs.player2;
                    let ap = gs.active_player();
                    unsafe {
                        _3ds_clear_top();
                    }

                    // Helper closures
                    let card_no = |cid: i16| -> Option<String> {
                        gs.card_database.get_card(cid).map(|c| c.card_no.clone())
                    };
                    let is_tapped = |cid: i16| -> bool {
                        gs.mods.orientation_modifiers.get(&cid).map(|s| s.as_str()) == Some("Wait")
                    };
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
                    // Macro to set all slots for one player
                    macro_rules! fill_player_board {
                        ($pb:expr,
                         $stage_fn:ident, $live_fn:ident,
                         $energy_fn:ident, $ecount_fn:ident,
                         $hand_fn:ident, $hcount_fn:ident,
                         $util_fn:ident) => {{
                            let pb = $pb;
                            // Stage
                            let st = &pb.stage.stage;
                            for i in 0..3 {
                                let cid = st[i];
                                let tapped = if cid != -1 { is_tapped(cid) } else { false };
                                set_slot($stage_fn, i as i32, cid, false, tapped);
                            }
                            // Live zone
                            let lc = &pb.live_card_zone.cards;
                            for i in 0..3.min(lc.len()) {
                                let cid = lc[i];
                                let tapped = if cid != -1 { is_tapped(cid) } else { false };
                                set_slot($live_fn, i as i32, cid, true, tapped);
                            }
                            for i in lc.len()..3 {
                                unsafe {
                                    $live_fn(i as i32, false, std::ptr::null(), 0, false, false);
                                }
                            }
                            // Energy
                            let ec = &pb.energy_zone.cards;
                            let ecount = ec.len().min(30);
                            unsafe {
                                $ecount_fn(ecount as i32);
                            }
                            for (i, cid) in ec.iter().enumerate().take(30) {
                                set_slot($energy_fn, i as i32, *cid, false, is_tapped(*cid));
                            }
                            // Hand
                            let hc = &pb.hand.cards;
                            let hcount = hc.len().min(15);
                            unsafe {
                                $hcount_fn(hcount as i32);
                            }
                            for (i, cid) in hc.iter().enumerate().take(15) {
                                set_slot($hand_fn, i as i32, *cid, false, false);
                            }
                            // Utility
                            unsafe {
                                $util_fn(
                                    pb.main_deck.cards.len() as i32,
                                    pb.energy_deck.cards.len() as i32,
                                    pb.waitroom.cards.len() as i32,
                                    pb.success_live_card_zone.cards.len() as i32,
                                );
                            }
                        }};
                    }
                    fill_player_board!(
                        p1,
                        _3ds_board_set_stage,
                        _3ds_board_set_live,
                        _3ds_board_set_energy,
                        _3ds_board_set_energy_count,
                        _3ds_board_set_hand,
                        _3ds_board_set_hand_count,
                        _3ds_board_set_utility
                    );
                    fill_player_board!(
                        p2,
                        _3ds_board_set_opp_stage,
                        _3ds_board_set_opp_live,
                        _3ds_board_set_opp_energy,
                        _3ds_board_set_opp_energy_count,
                        _3ds_board_set_opp_hand,
                        _3ds_board_set_opp_hand_count,
                        _3ds_board_set_opp_utility
                    );

                    // ---- TOP SCREEN (detail mode vs normal mode) ----
                    if detail_mode {
                        unsafe {
                            _3ds_text_set_scroll_y(0);
                        }
                        // Show just the selected card's full info
                        if cur < acts_cache.len() {
                            let act = &acts_cache[cur];
                            if let Some(ref p) = act.parameters {
                                if let Some(cid) = p.card_id {
                                    if let Some(card) = gs.card_database.get_card(cid) {
                                        let desc_w = wrap_text(&act.description, 38);
                                        unsafe {
                                            _3ds_text_add_top(
                                                format!(
                                                    "[{}] {} | {}\n\0",
                                                    card.card_no, card.name, desc_w
                                                )
                                                .as_ptr(),
                                            );
                                        }
                                        for ab in &card.abilities {
                                            let w = wrap_text(&ab.full_text, 40);
                                            unsafe {
                                                _3ds_text_add_top(format!("{}\n\0", w).as_ptr());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        unsafe {
                            _3ds_text_add_top("\n[X]=back\0".as_ptr());
                        }
                    } else {
                        // Normal mode: compact stats + action list
                        let ap_label = if ap.id == p1.id { "P1" } else { "P2" };
                        unsafe {
                            _3ds_text_add_top(
                                format!(
                                    "Turn {} | {:?} | {}\n\0",
                                    gs.turn_number, gs.current_phase, ap_label
                                )
                                .as_ptr(),
                            );
                            _3ds_text_add_top(format!(
                                "P1 H:{} E:{}/{} D:{} W:{} L:{}  P2 H:{} E:{}/{} D:{} W:{} L:{}\n\0",
                                p1.hand.cards.len(), p1.energy_zone.active_count(), p1.energy_zone.cards.len(),
                                p1.main_deck.cards.len(), p1.waitroom.cards.len(), p1.success_live_card_zone.cards.len(),
                                p2.hand.cards.len(), p2.energy_zone.active_count(), p2.energy_zone.cards.len(),
                                p2.main_deck.cards.len(), p2.waitroom.cards.len(), p2.success_live_card_zone.cards.len(),
                            ).as_ptr());
                        }
                        if cur < acts_cache.len() {
                            let desc_w = wrap_text(&acts_cache[cur].description, 36);
                            unsafe {
                                _3ds_text_add_top(format!("> {}\n\0", desc_w).as_ptr());
                            }
                        }
                        // Action list (centered around cursor)
                        let n = acts_cache.len();
                        let max_vis = 7usize;
                        let start = if cur + max_vis >= n {
                            n.saturating_sub(max_vis)
                        } else {
                            cur
                        };
                        let end = (start + max_vis).min(n);
                        if n > max_vis && start > 0 {
                            unsafe {
                                _3ds_text_add_top(format!("\u{25b2} +{}\n\0", start).as_ptr());
                            }
                        }
                        for i in start..end {
                            let arrow = if i == cur { ">" } else { " " };
                            let cp = acts_cache[i]
                                .parameters
                                .as_ref()
                                .and_then(|p| p.card_id)
                                .and_then(|cid| gs.card_database.get_card(cid))
                                .map(|c| format!("[{}] ", c.card_no))
                                .unwrap_or_default();
                            let desc_w = wrap_text(&acts_cache[i].description, 30);
                            unsafe {
                                _3ds_text_add_top(
                                    format!("{}[{:02}] {}{}\n\0", arrow, i, cp, desc_w).as_ptr(),
                                );
                            }
                        }
                        if n > max_vis && end < n {
                            unsafe {
                                _3ds_text_add_top(format!("\u{25bc} +{}\n\0", n - end).as_ptr());
                            }
                        }
                        unsafe {
                            _3ds_text_add_top("[X]=detail\0".as_ptr());
                        }
                    }

                    // ---- CARD DETAILS (from action cursor) ----
                    if cur < acts_cache.len() {
                        let act = &acts_cache[cur];
                        if let Some(ref p) = act.parameters {
                            if let Some(cid) = p.card_id {
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    let desc_w = wrap_text(&act.description, 38);
                                    unsafe {
                                        _3ds_text_add_top(
                                            format!(
                                                "[{}] {} | {}\n\0",
                                                card.card_no, card.name, desc_w
                                            )
                                            .as_ptr(),
                                        );
                                    }
                                    // Show abilities (word-wrapped)
                                    for ab in &card.abilities {
                                        let wrapped = wrap_text(&ab.full_text, 42);
                                        unsafe {
                                            _3ds_text_add_top(format!("{}\n\0", wrapped).as_ptr());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ---- ACTION LIST (scrollable, on top screen) ----
                    let n = acts_cache.len();
                    let line_h = unsafe { _3ds_bot_line_height() } as usize;
                    let max_lines = if line_h > 0 { 210 / line_h } else { 8 };
                    let max_vis = max_lines.saturating_sub(2);
                    let half = max_vis / 2;
                    let start_idx = if n > max_vis {
                        (cur as isize - half as isize)
                            .max(0)
                            .min((n - max_vis) as isize) as usize
                    } else {
                        0
                    };
                    let end_idx = (start_idx + max_vis).min(n);
                    if start_idx > 0 {
                        unsafe {
                            _3ds_text_add_top(format!("\u{25b2} +{}\n\0", start_idx).as_ptr());
                        }
                    }
                    for i in start_idx..end_idx {
                        let arrow = if i == cur { ">" } else { " " };
                        let desc_w = wrap_text(&acts_cache[i].description, 34);
                        let cp = acts_cache[i]
                            .parameters
                            .as_ref()
                            .and_then(|p| p.card_id)
                            .and_then(|cid| gs.card_database.get_card(cid))
                            .map(|c| format!("[{}] ", c.card_no))
                            .unwrap_or_default();
                        unsafe {
                            _3ds_text_add_top(
                                format!("{}[{:02}] {}{}\n\0", arrow, i, cp, desc_w).as_ptr(),
                            );
                        }
                    }
                    if end_idx < n {
                        unsafe {
                            _3ds_text_add_top(format!("\u{25bc} +{}\n\0", n - end_idx).as_ptr());
                        }
                    }

                    dirty = false;
                    redraw = false;
                }
                Step::Play(
                    gs,
                    cur,
                    acts_cache,
                    dirty,
                    redraw,
                    atlas.clone(),
                    *vs_ai,
                    detail_mode,
                )
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
        Step::Setup(_, _, _, _) => "Setup",
        Step::Play(_, _, _, _, _, _, _, _) => "Play",
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
    fn _3ds_text_set_scroll_y(y: i32);
    fn _3ds_text_get_scroll_y() -> i32;
    fn _3ds_bot_line_height() -> f32;

    // Board API
    fn _3ds_board_enable(on: bool);
    fn _3ds_board_cycle_view();
    fn _3ds_board_current_view() -> i32;
    fn _3ds_board_clear_cache();
    // Player slots
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
    // Opponent slots
    fn _3ds_board_set_opp_stage(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_opp_live(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_opp_energy(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_opp_energy_count(count: i32);
    fn _3ds_board_set_opp_hand(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    fn _3ds_board_set_opp_hand_count(count: i32);
    fn _3ds_board_set_opp_utility(deck: i32, edeck: i32, discard: i32, success: i32);
    fn _3ds_board_set_selection(slot: i32, slot_type: i32);
}

#[cfg(not(feature = "3ds"))]
fn main() {
    println!("Desktop mode - use: cargo run --bin harness");
}
