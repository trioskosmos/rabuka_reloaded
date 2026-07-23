#![allow(unused_unsafe)]
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
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

#[cfg(feature = "3ds")]
use rabuka_3ds::uds;
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
#[allow(unused_macros)]
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
fn cn_or_empty(act: &game_setup::Action) -> String {
    act.parameters
        .as_ref()
        .and_then(|p| p.card_no.clone())
        .unwrap_or_default()
}

/// Render text with inline `{{icon.png|label}}` icon images.
/// Uses _3ds_top_queue_card to render the actual icon T3X files.
fn render_text_with_icons(x: f32, y: f32, text: &str, color: u32, scale: f32, icon_h: f32) {
    let mut cx = x;
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        if start > 0 {
            unsafe {
                _3ds_top_queue_text(
                    cx,
                    y,
                    color,
                    scale,
                    format!("{}\0", &rest[..start]).as_ptr(),
                );
            }
            cx += start as f32 * scale * 8.0;
        }
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let inner = &after[..end];
            if let Some(bar) = inner.find('|') {
                let file = &inner[..bar];
                let iw = icon_h * 0.711;
                let atlas_name = format!("icon_{}.png.t3x", file);
                let c_str = std::ffi::CString::new(atlas_name.as_str()).unwrap_or_default();
                unsafe {
                    _3ds_top_queue_card(c_str.as_ptr(), 0, cx, y, iw, icon_h);
                }
                cx += iw + 2.0;
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    if !rest.is_empty() {
        unsafe {
            _3ds_top_queue_text(cx, y, color, scale, format!("{}\0", rest).as_ptr());
        }
    }
}

/// Wrap ability text — keeps `{{...}}` icon markers for later inline rendering.
fn wrap_ability_text(s: &str, max_chars: usize) -> String {
    wrap_text(s, max_chars)
}

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
    PickMode(usize), // cursor: 0=sandbox, 1=vsAI, 2=AIvsAI, 3=tests, 4=localMP
    PickDeck(usize, bool, bool), // cursor, vs_ai flag, is_multiplayer
    PickDeck2(usize, usize, bool), // cursor, p1_idx, vs_ai
    Loading(usize, usize, bool), // p1_idx, p2_idx, vs_ai
    Testing,         // On-device test suite
    // Multiplayer lobby phases
    MultiplayerDeck(usize), // cursor, selecting deck for multiplayer
    MultiplayerPickRole(usize, usize), // deck_idx, role_cursor (0=Host, 1=Client)
    MultiplayerHostWait(usize), // p1_idx: host waiting for client to connect
    MultiplayerClientScan(usize), // p1_idx: client scanning for host network
    MultiplayerSyncDeck(usize, usize, bool), // p1_idx, p2_idx, is_host
    MultiplayerLoading(usize, usize, bool, Option<Vec<u8>>), // p1_idx, p2_idx, is_host, deck_sync_bytes
    QrScan,                                                  // QR code scanning for deck import
}

#[cfg(feature = "3ds")]
#[derive(Clone)]
enum Step {
    ReadCardsBin,
    ParseCards(Vec<u8>),
    Setup(Arc<Vec<Card>>, Vec<DeckList>, SetupPhase, bool),
    Play(
        GameState,
        usize, // cursor
        Vec<game_setup::Action>,
        bool, // dirty
        bool, // redraw
        CardAtlas,
        bool,                       // vs_ai (human vs AI)
        bool,                       // ai_vs_ai (spectator: both AI)
        bool,                       // cli_mode
        bool,                       // detail_mode
        bool,                       // choice_image_mode
        usize,                      // hand_offset (P1)
        usize,                      // hand_offset_p2
        u32,                        // touch_tap_count
        Option<i16>,                // viewing_card_id
        Option<(String, Vec<i16>)>, // zone_viewer (label, card_ids)
        usize,                      // zone_viewer_offset
        bool,                       // was_touching (edge detect for touch screen)
        bool,                       // is_multiplayer
        bool,                       // is_host (true = P1/host, false = P2/client)
        bool,                       // waiting_for_opponent
    ),
    Done(Result<(), String>),
}

/// On-device test suite — runs QA checks in limited 3DS memory.
/// Accessed via "Run Tests" menu. Each test returns a result line.
#[cfg(feature = "3ds")]
fn run_on_device_tests(cards: Arc<Vec<Card>>, decks: Vec<DeckList>) -> Vec<String> {
    let mut r: Vec<String> = Vec::new();
    let t0 = unsafe { _3ds_system_tick() };
    r.push(format!("CARDS: {}", cards.len()));
    let mut cards_vec = (*cards).clone();
    CardLoader::attach_abilities(&mut cards_vec);
    let wa = cards_vec.iter().filter(|c| !c.abilities.is_empty()).count();
    r.push(if wa > 0 {
        format!("ABILITIES: {} (OK)", wa)
    } else {
        "ABILITIES: NONE (FAIL!)".into()
    });
    r.push(format!("DECKS: {}", decks.len()));
    if let Some(c) = cards.first() {
        let nl = c.name.len();
        r.push(format!("CARD[0]: {} ({}ch) OK", &c.name[..nl.min(20)], nl));
    } else {
        r.push("CARD[0]: NONE (FAIL!)".into());
    }
    let he = cards.iter().any(|c| {
        let cn: &str = &c.card_no;
        cn.contains("LL-E-005")
    });
    r.push(if he {
        "ENERGY: found (OK)".into()
    } else {
        "ENERGY: missing (FAIL!)".into()
    });
    if decks.len() >= 2 {
        match test_ai_vs_ai(&cards_vec, &decks[0], &decks[1], 5) {
            Ok(n) => r.push(format!("AI PLAY: {} actions (OK)", n)),
            Err(e) => r.push(format!("AI PLAY: FAIL {}", e)),
        }
    } else {
        r.push("AI PLAY: skip (need 2 decks)".into());
    }
    let ms = ticks_to_ms(unsafe { _3ds_system_tick() } - t0);
    r.push(format!("TIME: {}ms", ms));
    r.push("=== DONE ===".into());
    r
}

/// Mini AI vs AI test: sets up game, runs 5 turns with random AI.
#[cfg(feature = "3ds")]
fn test_ai_vs_ai(cards: &[Card], d1: &DeckList, d2: &DeckList, mt: u32) -> Result<usize, String> {
    use rabuka_engine::card::CardDatabase;
    use std::sync::Arc;
    let mut db = Arc::new(CardDatabase::load_or_create(cards.to_vec()));
    let n1 = DeckParser::deck_list_to_card_numbers(d1);
    let n2 = DeckParser::deck_list_to_card_numbers(d2);
    let mut pd1 =
        DeckBuilder::build_deck_from_database(&mut db, n1).map_err(|e| format!("D1:{}", e))?;
    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    let mut pd2 =
        DeckBuilder::build_deck_from_database(&mut db, n2).map_err(|e| format!("D2:{}", e))?;
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
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
    let mut c = 0usize;
    let mut tu = 0u32;
    let max_iter = (mt * 40) as usize;
    while gs.game_result == GameResult::Ongoing && tu < mt * 2 && c < max_iter {
        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            break;
        }
        let a = acts[0].clone();
        let p = a.parameters.clone();
        let _ = turn::TurnEngine::execute_main_phase_action(
            &mut gs,
            &a.action_type,
            p.as_ref().and_then(|x| x.card_id),
            p.as_ref().and_then(|x| x.card_indices.clone()),
            p.as_ref()
                .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
            p.as_ref().and_then(|x| x.use_baton_touch),
        );
        gs.reset_loop_detection();
        gs.reset_loop_detection();
        c += 1;
        while gs.game_result == GameResult::Ongoing && game_setup::is_automatic_phase(&gs) {
            turn::TurnEngine::advance_phase(&mut gs);
            tu += 1;
        }
        if gs.current_phase == Phase::Active || gs.current_phase == Phase::Draw {
            tu += 1;
        }
    }
    Ok(c)
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

// Helper to build C2D color values.
// C2D stores colors as 0xAABBGGRR in the u32 literal:
//   bits 31-24 = Alpha
//   bits 23-16 = Blue
//   bits 15-8  = Green
//   bits 7-0   = Red
// The GPU on 3DS reads little-endian memory bytes as RGBA,
// so the u32 literal must be AABBGGRR (A in MSB, R in LSB).
const fn c2d(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | r as u32
}

// Precomputed color constants for game-mode rendering.
// Each uses c2d(R,G,B,A) so the hex matches how GPU reads it.
const COL_TOP_BG: u32 = 0xFF1A0E0A; // c2d(10,14,26,255)   dark navy background
const COL_PANEL: u32 = 0xFF3C2A1A; // c2d(26,42,60,255)   dark blue-gray panel
const COL_GOLD: u32 = 0xFF0B9EF5; // c2d(245,158,11,255) gold text
const COL_LIGHT: u32 = 0xFFDBD5D1; // c2d(209,213,219,255) light gray text
const COL_MED: u32 = 0xFF80726B; // c2d(107,114,128,255) medium gray text
const COL_SEL: u32 = 0xFF5C3A2A; // c2d(42,58,92,255)   selected-item background
const COL_DIM: u32 = 0x66231A33; // c2d(26,35,51,102)   semi-transparent dark
const COL_HIGHLIGHT: u32 = 0x330B9EF5; // c2d(245,158,11,51)  semi-transparent gold
const COL_CARD: u32 = 0x22231A22; // c2d(34,26,35,34)    card detail semi-transparent
const COL_ABILITY: u32 = 0x33231A2A; // c2d(42,26,51,51)    ability queue semi-transparent
const COL_BLUE: u32 = 0xFFFF9E4A; // c2d(74,158,255,255) blue accent text
const COL_PINK: u32 = 0xFFAA55FF; // c2d(255,85,170,255) pink accent text

/// Phase-aware multiplayer turn check.
/// Returns true if the given player (0=P1, 1=P2) should be able to act.
fn mp_can_act(gs: &GameState, player_id: i32) -> bool {
    gs.can_player_act(player_id)
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

    let mut _frame: u64 = 0;
    let mut step = Step::ReadCardsBin;

    while unsafe { _3ds_main_loop() != 0 } {
        unsafe {
            _3ds_scan_input();
        }
        let keys = unsafe { _3ds_keys_down() };
        let _held = unsafe { _3ds_keys_held() };
        if keys & 0x00000008 != 0 {
            break;
        }

        let _current_step = step_name(&step);
        _frame += 1;

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
                            Ok(d) => {
                                let mut decks = d;
                                decks.sort_by(|a, b| {
                                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                                });
                                decks
                            }
                            Err(e) => {
                                step = Step::Done(Err(format!("No decks: {}", e)));
                                continue;
                            }
                        };
                        if decks.is_empty() {
                            step = Step::Done(Err("No decks found!".into()));
                            continue;
                        }
                        Step::Setup(Arc::new(cards), decks, SetupPhase::PickMode(0), true)
                    }
                    Err(e) => Step::Done(Err(format!("Parse: {}", e))),
                }
            }
            Step::Setup(ref cards, ref decks, ref phase, ref dirty) => {
                let n = decks.len();
                let was_dirty = *dirty;
                let new_step = match phase.clone() {
                    SetupPhase::PickMode(cur) => unsafe {
                        if was_dirty {
                            if _3ds_is_cli_mode() {
                                _3ds_clear_top();
                                _3ds_text_add_top("SELECT MODE\n\0".as_ptr());
                                for (i, m) in [
                                    "VS AI",
                                    "Sandbox (2 players)",
                                    "QR Scan",
                                    "Local Multiplayer",
                                ]
                                .iter()
                                .enumerate()
                                {
                                    let arrow = if i == cur { ">" } else { " " };
                                    _3ds_text_add_top(
                                        format!("{} [{}] {}\n\0", arrow, i, m).as_ptr(),
                                    );
                                }
                                _3ds_text_add_top("\nUP/DOWN=select A=confirm\0".as_ptr());
                            } else {
                                _3ds_top_clear();
                                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                _3ds_top_queue_text(
                                    100.0,
                                    8.0,
                                    COL_GOLD,
                                    0.85f32,
                                    "SELECT MODE\0".as_ptr(),
                                );
                                for (i, m) in ["VS AI", "Sandbox", "QR Scan", "Local MP"]
                                    .iter()
                                    .enumerate()
                                {
                                    let y = 40.0 + i as f32 * 38.0;
                                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                                    unsafe {
                                        _3ds_top_queue_rect(40.0, y, 320.0, 36.0, bg);
                                    }
                                    if i == cur {
                                        unsafe {
                                            _3ds_top_queue_rect(
                                                40.0,
                                                y,
                                                320.0,
                                                36.0,
                                                COL_HIGHLIGHT,
                                            );
                                        }
                                    }
                                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            50.0,
                                            y + 6.0,
                                            color,
                                            0.65f32,
                                            format!("{}\0", m).as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        50.0,
                                        230.0,
                                        COL_MED,
                                        0.60f32,
                                        "UP/DOWN=select  A=confirm\0".as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000040 != 0 && cur > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur - 1),
                                true,
                            )
                        } else if keys & 0x00000080 != 0 && cur + 1 < 4 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur + 1),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            if cur == 2 {
                                // "QR Scan"
                                Step::Setup(cards.clone(), decks.clone(), SetupPhase::QrScan, true)
                            } else if cur == 3 {
                                // "Local Multiplayer" — pick deck then connect
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerDeck(0),
                                    true,
                                )
                            } else if n == 0 {
                                Step::Done(Err("No decks!".into()))
                            } else {
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::PickDeck(0, cur == 0, false),
                                    true,
                                )
                            }
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur),
                                false,
                            )
                        }
                    },
                    SetupPhase::PickDeck(cur, vs_ai, is_multiplayer) => {
                        if was_dirty {
                            let label = if !vs_ai { "P1 DECK" } else { "YOUR DECK" };
                            if unsafe { _3ds_is_cli_mode() } {
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
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        80.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        format!("SELECT {}\0", label).as_ptr(),
                                    );
                                }
                                // Show 6 decks max: 240px screen - 30px title - 20px help = 190px
                                // 190 / 6 = ~32px per row at 0.70 scale (~21px glyph)
                                let start = cur.saturating_sub(3).min(n.saturating_sub(6));
                                let end = (start + 6).min(n);
                                for i in start..end {
                                    let y = 30.0 + (i - start) as f32 * 32.0;
                                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                                    unsafe {
                                        _3ds_top_queue_rect(20.0, y, 360.0, 30.0, bg);
                                    }
                                    if i == cur {
                                        unsafe {
                                            _3ds_top_queue_rect(
                                                20.0,
                                                y,
                                                360.0,
                                                30.0,
                                                COL_HIGHLIGHT,
                                            );
                                        }
                                    }
                                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            24.0,
                                            y + 3.0,
                                            color,
                                            0.70f32,
                                            format!("{}\0", decks[i].name).as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        20.0,
                                        228.0,
                                        COL_MED,
                                        0.65f32,
                                        "UP/DOWN=select  A=confirm\0".as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000040 != 0 && cur > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck(cur - 1, vs_ai, is_multiplayer),
                                true,
                            )
                        } else if keys & 0x00000080 != 0 && cur + 1 < n {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck(cur + 1, vs_ai, is_multiplayer),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            if is_multiplayer {
                                // Local Multiplayer: go to role selection
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerPickRole(cur, 0),
                                    true,
                                )
                            } else if vs_ai {
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
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck(cur, vs_ai, is_multiplayer),
                                false,
                            )
                        }
                    }
                    SetupPhase::MultiplayerDeck(cur) => {
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                }
                                unsafe {
                                    _3ds_text_add_top("SELECT YOUR DECK\n\0".as_ptr());
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
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        80.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        "SELECT YOUR DECK\0".as_ptr(),
                                    );
                                }
                                let start = cur.saturating_sub(3).min(n.saturating_sub(6));
                                let end = (start + 6).min(n);
                                for i in start..end {
                                    let y = 30.0 + (i - start) as f32 * 32.0;
                                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                                    unsafe {
                                        _3ds_top_queue_rect(20.0, y, 360.0, 30.0, bg);
                                    }
                                    if i == cur {
                                        unsafe {
                                            _3ds_top_queue_rect(
                                                20.0,
                                                y,
                                                360.0,
                                                30.0,
                                                COL_HIGHLIGHT,
                                            );
                                        }
                                    }
                                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            24.0,
                                            y + 3.0,
                                            color,
                                            0.70f32,
                                            format!("{}\0", decks[i].name).as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        20.0,
                                        228.0,
                                        COL_MED,
                                        0.65f32,
                                        "UP/DOWN=select  A=confirm\0".as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000040 != 0 && cur > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerDeck(cur - 1),
                                true,
                            )
                        } else if keys & 0x00000080 != 0 && cur + 1 < n {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerDeck(cur + 1),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            // A = select deck, go to role selection with deck index
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerPickRole(cur, 0), // deck_idx=cur, role_cursor=0
                                true,
                            )
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerDeck(cur),
                                false,
                            )
                        }
                    }
                    SetupPhase::PickDeck2(cur, p1_idx, vs_ai) => {
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                }
                                unsafe {
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
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        80.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        "SELECT P2 DECK\0".as_ptr(),
                                    );
                                }
                                let start = cur.saturating_sub(3).min(n.saturating_sub(6));
                                let end = (start + 6).min(n);
                                for i in start..end {
                                    let y = 30.0 + (i - start) as f32 * 32.0;
                                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                                    unsafe {
                                        _3ds_top_queue_rect(20.0, y, 360.0, 30.0, bg);
                                    }
                                    if i == cur {
                                        unsafe {
                                            _3ds_top_queue_rect(
                                                20.0,
                                                y,
                                                360.0,
                                                30.0,
                                                COL_HIGHLIGHT,
                                            );
                                        }
                                    }
                                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            24.0,
                                            y + 3.0,
                                            color,
                                            0.70f32,
                                            format!("{}\0", decks[i].name).as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        20.0,
                                        228.0,
                                        COL_MED,
                                        0.65f32,
                                        "A=select  B=use same\0".as_ptr(),
                                    );
                                }
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
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck2(cur, p1_idx, vs_ai),
                                false,
                            )
                        }
                    }
                    SetupPhase::Loading(p1_idx, p2_idx, vs_ai) => {
                        let r = (|| -> Result<(GameState, CardAtlas), String> {
                            let mut cards_vec = (**cards).clone();
                            CardLoader::attach_abilities(&mut cards_vec);
                            let mut db = Arc::new(CardDatabase::load_or_create(cards_vec));
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
                                    deck_nos.insert(card.card_no.to_string());
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
                                Step::Play(
                                    gs,
                                    0,
                                    Vec::new(),
                                    true,
                                    true,
                                    atlas,
                                    vs_ai,
                                    false, // ai_vs_ai
                                    false, // cli_mode (start in game mode)
                                    false, // detail_mode
                                    true,  // choice_image_mode
                                    0,
                                    0,
                                    0,
                                    None,
                                    None,  // zone_viewer
                                    0,     // zone_viewer_offset
                                    false, // was_touching
                                    false, // is_multiplayer
                                    false, // is_host
                                    false, // waiting_for_opponent
                                )
                            }
                            Err(e) => Step::Done(Err(e)),
                        }
                    }
                    SetupPhase::Testing => {
                        let results = run_on_device_tests(cards.clone(), decks.clone());
                        unsafe {
                            _3ds_clear_both();
                            _3ds_text_add_top("=== ON-DEVICE TESTS ===\n\0".as_ptr());
                            for line in &results {
                                _3ds_text_add_top(format!("{}\n\0", line).as_ptr());
                            }
                            _3ds_text_add_top("\nSTART=exit\0".as_ptr());
                        }
                        if keys & 0x00000008 != 0 {
                            Step::Done(Ok(()))
                        } else {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::Testing, false)
                        }
                    }
                    SetupPhase::QrScan => {
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                    _3ds_text_add_top("QR SCAN\n\0".as_ptr());
                                    _3ds_text_add_top("A=scan  B=back\0".as_ptr());
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        100.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        "QR SCAN\0".as_ptr(),
                                    );
                                    _3ds_top_queue_text(
                                        40.0,
                                        60.0,
                                        COL_LIGHT,
                                        0.70f32,
                                        "Point camera at deck QR code\non your phone screen\0"
                                            .as_ptr(),
                                    );
                                    _3ds_top_queue_text(
                                        40.0,
                                        230.0,
                                        COL_MED,
                                        0.60f32,
                                        "A=scan  B=back\0".as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000002 != 0 {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(5), true)
                        } else if keys & 0x00000001 != 0 {
                            let r_init = unsafe { _3ds_qr_init() };
                            if r_init != 0 {
                                unsafe {
                                    _3ds_clear_both();
                                    _3ds_text_add_top(
                                        format!("Camera init failed: {}\0", r_init).as_ptr(),
                                    );
                                    _3ds_text_add_top("B=back\0".as_ptr());
                                }
                            } else {
                                let mut buf = [0u8; 2048];
                                let r_scan =
                                    unsafe { _3ds_qr_scan(buf.as_mut_ptr(), buf.len() as u32) };
                                unsafe { _3ds_qr_exit() };
                                if r_scan > 0 {
                                    let text = String::from_utf8_lossy(&buf[..r_scan as usize])
                                        .to_string();
                                    let cards_read = DeckParser::parse_deck_content(&text);
                                    unsafe {
                                        _3ds_clear_both();
                                        _3ds_text_add_top(
                                            format!("QR: {} cards read!\n\0", cards_read.len())
                                                .as_ptr(),
                                        );
                                        _3ds_text_add_top("B=back\0".as_ptr());
                                    }
                                } else {
                                    unsafe {
                                        _3ds_clear_both();
                                        _3ds_text_add_top(
                                            format!("Scan failed: {}\0", r_scan).as_ptr(),
                                        );
                                        _3ds_text_add_top("B=back\0".as_ptr());
                                    }
                                }
                            }
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::QrScan, true)
                        } else {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::QrScan, false)
                        }
                    }
                    // Multiplayer: Host or Client?
                    SetupPhase::MultiplayerPickRole(deck_idx, cur) => {
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                    _3ds_text_add_top("MULTIPLAYER\n\0".as_ptr());
                                    _3ds_text_add_top("Host = create network\n\0".as_ptr());
                                    _3ds_text_add_top("Client = join network\n\n\0".as_ptr());
                                    let arrow_h = if cur == 0 { ">" } else { " " };
                                    let arrow_c = if cur == 1 { ">" } else { " " };
                                    _3ds_text_add_top(format!("{} Host\n\0", arrow_h).as_ptr());
                                    _3ds_text_add_top(format!("{} Client\n\0", arrow_c).as_ptr());
                                    _3ds_text_add_top("\nUP/DOWN=select A=confirm\0".as_ptr());
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        100.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        "MULTIPLAYER\0".as_ptr(),
                                    );
                                }
                                let labels = ["HOST (create network)", "CLIENT (join network)"];
                                for (i, m) in labels.iter().enumerate() {
                                    let y = 60.0 + i as f32 * 64.0;
                                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                                    unsafe {
                                        _3ds_top_queue_rect(40.0, y, 320.0, 50.0, bg);
                                    }
                                    if i == cur {
                                        unsafe {
                                            _3ds_top_queue_rect(
                                                40.0,
                                                y,
                                                320.0,
                                                50.0,
                                                COL_HIGHLIGHT,
                                            );
                                        }
                                    }
                                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            50.0,
                                            y + 12.0,
                                            color,
                                            0.70f32,
                                            format!("{}\0", m).as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        50.0,
                                        230.0,
                                        COL_MED,
                                        0.60f32,
                                        "UP/DOWN=select  A=confirm  B=back\0".as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000040 != 0 && cur > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerPickRole(deck_idx, cur - 1),
                                true,
                            )
                        } else if keys & 0x00000080 != 0 && cur + 1 < 2 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerPickRole(deck_idx, cur + 1),
                                true,
                            )
                        } else if keys & 0x00000002 != 0 {
                            // B = back to PickMode
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(4), true)
                        } else if keys & 0x00000001 != 0 {
                            // A = select role
                            // deck_idx is the deck index from MultiplayerDeck
                            // cur is the role cursor (0=Host, 1=Client)
                            if cur == 0 {
                                // Host
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerHostWait(deck_idx),
                                    true,
                                )
                            } else {
                                // Client
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerClientScan(deck_idx),
                                    true,
                                )
                            }
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerPickRole(deck_idx, cur),
                                false,
                            )
                        }
                    }
                    // Multiplayer: Host waiting for client
                    SetupPhase::MultiplayerHostWait(p1_idx) => {
                        // Initialize UDS as host on first entry
                        if was_dirty {
                            let init_result = uds::uds_init(true);
                            match init_result {
                                Ok(()) => {
                                    if unsafe { _3ds_is_cli_mode() } {
                                        unsafe {
                                            _3ds_clear_top();
                                            _3ds_text_add_top(
                                                "HOST: Network created!\n\0".as_ptr(),
                                            );
                                            _3ds_text_add_top("Waiting for client...\n\0".as_ptr());
                                            _3ds_text_add_top("B = cancel\n\0".as_ptr());
                                        }
                                    } else {
                                        unsafe {
                                            _3ds_top_clear();
                                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                            _3ds_top_queue_text(
                                                80.0,
                                                8.0,
                                                COL_GOLD,
                                                0.85f32,
                                                "HOST: Network created!\0".as_ptr(),
                                            );
                                            _3ds_top_queue_text(
                                                50.0,
                                                100.0,
                                                COL_LIGHT,
                                                0.70f32,
                                                "Waiting for client to connect\n\0".as_ptr(),
                                            );
                                            _3ds_top_queue_text(
                                                50.0,
                                                230.0,
                                                COL_MED,
                                                0.60f32,
                                                "B=cancel\0".as_ptr(),
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    if unsafe { _3ds_is_cli_mode() } {
                                        unsafe {
                                            _3ds_clear_top();
                                            _3ds_text_add_top(
                                                format!("UDS INIT FAILED: {}\n\0", e).as_ptr(),
                                            );
                                            _3ds_text_add_top("B = back\n\0".as_ptr());
                                        }
                                    } else {
                                        unsafe {
                                            _3ds_top_clear();
                                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                            _3ds_top_queue_text(
                                                80.0,
                                                8.0,
                                                0xFF0000FF,
                                                0.85f32,
                                                format!("UDS INIT FAILED: {}\0", e).as_ptr(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        // Poll for client connection: try to receive a hello packet
                        let mut hello = [0u8; 4];
                        match uds::uds_recv(&mut hello) {
                            Ok(n) if n > 0 => {
                                // Client connected! Move to deck sync
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerSyncDeck(p1_idx, 0, true),
                                    true,
                                )
                            }
                            _ => {
                                if keys & 0x00000002 != 0 {
                                    // B = cancel
                                    uds::uds_exit();
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::MultiplayerPickRole(p1_idx, 0),
                                        true,
                                    )
                                } else {
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::MultiplayerHostWait(p1_idx),
                                        false,
                                    )
                                }
                            }
                        }
                    }
                    // Multiplayer: Client scanning for host
                    SetupPhase::MultiplayerClientScan(p1_idx) => {
                        // Initialize UDS as client on first entry
                        if was_dirty {
                            let init_result = uds::uds_init(false);
                            match init_result {
                                Ok(()) => {
                                    if unsafe { _3ds_is_cli_mode() } {
                                        unsafe {
                                            _3ds_clear_top();
                                            _3ds_text_add_top("CLIENT: Scanning...\n\0".as_ptr());
                                            _3ds_text_add_top(
                                                "Looking for host network\n\0".as_ptr(),
                                            );
                                            _3ds_text_add_top("B = cancel\n\0".as_ptr());
                                        }
                                    } else {
                                        unsafe {
                                            _3ds_top_clear();
                                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                            _3ds_top_queue_text(
                                                80.0,
                                                8.0,
                                                COL_GOLD,
                                                0.85f32,
                                                "CLIENT: Scanning...\0".as_ptr(),
                                            );
                                            _3ds_top_queue_text(
                                                50.0,
                                                100.0,
                                                COL_LIGHT,
                                                0.70f32,
                                                "Looking for host network\n\0".as_ptr(),
                                            );
                                            _3ds_top_queue_text(
                                                50.0,
                                                230.0,
                                                COL_MED,
                                                0.60f32,
                                                "B=cancel\0".as_ptr(),
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    if unsafe { _3ds_is_cli_mode() } {
                                        unsafe {
                                            _3ds_clear_top();
                                            _3ds_text_add_top(
                                                format!("UDS INIT FAILED: {}\n\0", e).as_ptr(),
                                            );
                                            _3ds_text_add_top("B = back\n\0".as_ptr());
                                        }
                                    } else {
                                        unsafe {
                                            _3ds_top_clear();
                                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                            _3ds_top_queue_text(
                                                80.0,
                                                8.0,
                                                0xFF0000FF,
                                                0.85f32,
                                                format!("UDS INIT FAILED: {}\0", e).as_ptr(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        // Poll for host connection
                        let connected = uds::uds_is_connected();
                        if connected {
                            // Send hello so host knows we're here
                            let hello = [0xAAu8];
                            let _ = uds::uds_send(&hello);
                            // Host found! Move to deck sync
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerSyncDeck(p1_idx, 0, false),
                                true,
                            )
                        } else if keys & 0x00000002 != 0 {
                            // B = cancel
                            uds::uds_exit();
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerPickRole(p1_idx, 0),
                                true,
                            )
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerClientScan(p1_idx),
                                false,
                            )
                        }
                    }
                    // Multiplayer: Syncing deck data
                    SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, is_host) => {
                        if is_host {
                            // Host: Build decks, shuffle, send order to client
                            let r = (|| -> Result<(), String> {
                                use rabuka_engine::card::CardDatabase;
                                let mut cards_vec = (**cards).clone();
                                CardLoader::attach_abilities(&mut cards_vec);
                                let mut db = Arc::new(CardDatabase::load_or_create(cards_vec));
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
                                // Use a random seed for deterministic shuffle
                                let seed = unsafe { _3ds_system_tick() } as u64;
                                pd1.shuffle_main_deck();
                                pd1.shuffle_energy_deck();
                                pd2.shuffle_main_deck();
                                pd2.shuffle_energy_deck();
                                // Build deck sync message
                                let sync = uds::DeckSync {
                                    seed,
                                    p1_main: pd1.main_deck.clone().into(),
                                    p1_energy: pd1.energy_deck.clone().into(),
                                    p2_main: pd2.main_deck.clone().into(),
                                    p2_energy: pd2.energy_deck.clone().into(),
                                };
                                let data = sync.to_bytes();
                                uds::uds_send(&data).map_err(|e| format!("Send: {}", e))?;
                                Ok(())
                            })();
                            match r {
                                Ok(()) => Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerLoading(p1_idx, p2_idx, true, None),
                                    true,
                                ),
                                Err(e) => {
                                    uds::uds_exit();
                                    Step::Done(Err(format!("Deck sync failed: {}", e)))
                                }
                            }
                        } else {
                            // Client: Receive deck order from host
                            if was_dirty {
                                if unsafe { _3ds_is_cli_mode() } {
                                    unsafe {
                                        _3ds_clear_top();
                                        _3ds_text_add_top("Receiving deck data...\n\0".as_ptr());
                                    }
                                } else {
                                    unsafe {
                                        _3ds_top_clear();
                                        _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                        _3ds_top_queue_text(
                                            80.0,
                                            8.0,
                                            COL_GOLD,
                                            0.85f32,
                                            "Receiving deck data...\0".as_ptr(),
                                        );
                                    }
                                }
                            }
                            // Try to receive deck sync
                            let mut recv_buf = [0u8; 4096];
                            match uds::uds_recv(&mut recv_buf) {
                                Ok(n) if n > 0 => {
                                    if uds::DeckSync::from_bytes(&recv_buf[..n]).is_some() {
                                        // Store the raw sync bytes for the loading phase
                                        let sync_bytes = recv_buf[..n].to_vec();
                                        Step::Setup(
                                            cards.clone(),
                                            decks.clone(),
                                            SetupPhase::MultiplayerLoading(
                                                p1_idx,
                                                p2_idx,
                                                false,
                                                Some(sync_bytes),
                                            ),
                                            true,
                                        )
                                    } else {
                                        Step::Setup(
                                            cards.clone(),
                                            decks.clone(),
                                            SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, false),
                                            false,
                                        )
                                    }
                                }
                                _ => {
                                    // No data yet, keep waiting
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, false),
                                        false,
                                    )
                                }
                            }
                        }
                    }
                    // Multiplayer: Loading game with multiplayer flag
                    SetupPhase::MultiplayerLoading(p1_idx, p2_idx, is_host, deck_sync_bytes) => {
                        let r = (|| -> Result<(GameState, CardAtlas), String> {
                            let mut cards_vec = (**cards).clone();
                            CardLoader::attach_abilities(&mut cards_vec);
                            let mut db = Arc::new(CardDatabase::load_or_create(cards_vec));
                            // If we have deck sync data from host, use it directly
                            if let Some(ref sync_bytes) = deck_sync_bytes {
                                let sync = uds::DeckSync::from_bytes(sync_bytes)
                                    .ok_or("Invalid deck sync data")?;
                                // Build players from received card IDs (already shuffled)
                                let mut d1 = rabuka_engine::deck_builder::Deck {
                                    main_deck: std::collections::VecDeque::from(sync.p1_main),
                                    energy_deck: std::collections::VecDeque::from(sync.p1_energy),
                                };
                                let mut d2 = rabuka_engine::deck_builder::Deck {
                                    main_deck: std::collections::VecDeque::from(sync.p2_main),
                                    energy_deck: std::collections::VecDeque::from(sync.p2_energy),
                                };
                                DeckBuilder::add_default_energy_cards_from_database(
                                    &mut d1, &mut db,
                                )
                                .ok();
                                DeckBuilder::add_default_energy_cards_from_database(
                                    &mut d2, &mut db,
                                )
                                .ok();
                                let mut p1 = Player::new("p1".into(), "P1".into(), true);
                                p1.set_main_deck(d1.main_deck);
                                p1.set_energy_deck(d1.energy_deck);
                                let mut p2 = Player::new("p2".into(), "P2".into(), false);
                                p2.set_main_deck(d2.main_deck);
                                p2.set_energy_deck(d2.energy_deck);
                                let mut gs = GameState::new(p1, p2, db);
                                game_setup::setup_game(&mut gs);
                                return Ok((gs, CardAtlas::load()));
                            }
                            // No deck sync: build from local files (host or non-multiplayer)
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
                                    deck_nos.insert(card.card_no.to_string());
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
                                Step::Play(
                                    gs,
                                    0,
                                    Vec::new(),
                                    true,
                                    true,
                                    atlas,
                                    false, // vs_ai (this is multiplayer)
                                    false, // ai_vs_ai
                                    false, // cli_mode (start in game mode)
                                    false, // detail_mode
                                    true,  // choice_image_mode
                                    0,
                                    0,
                                    0,
                                    None,
                                    None,     // zone_viewer
                                    0,        // zone_viewer_offset
                                    false,    // was_touching
                                    true,     // is_multiplayer
                                    is_host,  // is_host
                                    !is_host, // waiting_for_opponent will be recalculated after settle
                                )
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
                ref ai_vs_ai,
                mut cli_mode,
                mut detail_mode,
                mut choice_image_mode,
                mut hand_offset,
                mut hand_offset_p2,
                mut touch_tap_count,
                mut viewing_card,
                mut zone_viewer,
                mut zone_viewer_offset,
                mut was_touching,
                is_multiplayer,
                is_host,
                mut waiting_for_opponent,
            ) => {
                // Build display order from current acts_cache for navigation.
                // This will be rebuilt after acts_cache regeneration if dirty/redraw.
                let mut display_order: Vec<usize> = {
                    let mut order: Vec<usize> = Vec::new();
                    for (i, act) in acts_cache.iter().enumerate() {
                        if act.action_type == game_setup::ActionType::Pass {
                            order.push(i);
                            break;
                        }
                    }
                    for (i, act) in acts_cache.iter().enumerate() {
                        if act.action_type == game_setup::ActionType::UseAbility {
                            order.push(i);
                        }
                    }
                    for (i, act) in acts_cache.iter().enumerate() {
                        if act.action_type != game_setup::ActionType::Pass
                            && act.action_type != game_setup::ActionType::UseAbility
                        {
                            order.push(i);
                        }
                    }
                    order
                };
                let mut display_pos = display_order.iter().position(|&fi| fi == cur).unwrap_or(0);

                // Input handling
                if detail_mode && viewing_card.is_some() {
                    // Detail+card: DPAD navigates the filtered action list
                    let n = display_order.len();
                    if keys & 0x00000040 != 0 && n > 0 {
                        display_pos = if display_pos > 0 {
                            display_pos - 1
                        } else {
                            n - 1
                        };
                        cur = display_order[display_pos];
                        redraw = true;
                    }
                    if keys & 0x00000080 != 0 && n > 0 {
                        display_pos = if display_pos + 1 < n {
                            display_pos + 1
                        } else {
                            0
                        };
                        cur = display_order[display_pos];
                        redraw = true;
                    }
                } else if detail_mode {
                    // Detail alone (no specific card): DPAD scrolls text
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
                    // Navigate in display space with wrap-around
                    let n = display_order.len();
                    if n > 0 {
                        if keys & 0x00000040 != 0 {
                            display_pos = if display_pos > 0 {
                                display_pos - 1
                            } else {
                                n - 1
                            };
                            cur = display_order[display_pos];
                            redraw = true;
                        }
                        if keys & 0x00000080 != 0 {
                            display_pos = if display_pos + 1 < n {
                                display_pos + 1
                            } else {
                                0
                            };
                            cur = display_order[display_pos];
                            redraw = true;
                        }
                    }
                }

                // Image mode: grid navigation with wrap-around (disabled in zone viewer)
                let img_cols = ((400.0 - 8.0) / (80.0 + 4.0)) as usize;
                let is_img_choice = choice_image_mode && gs.has_pending_choice();
                if zone_viewer.is_none() && is_img_choice {
                    let n = display_order.len();
                    if n > 0 {
                        if keys & 0x00000040 != 0 {
                            display_pos = if display_pos >= img_cols {
                                display_pos - img_cols
                            } else {
                                n - 1
                            };
                            cur = display_order[display_pos];
                            redraw = true;
                        }
                        if keys & 0x00000080 != 0 {
                            display_pos = if display_pos + img_cols < n {
                                display_pos + img_cols
                            } else {
                                0
                            };
                            cur = display_order[display_pos];
                            redraw = true;
                        }
                        if keys & 0x00000020 != 0 {
                            display_pos = if display_pos > 0 {
                                display_pos - 1
                            } else {
                                n - 1
                            };
                            cur = display_order[display_pos];
                            redraw = true;
                        }
                        if keys & 0x00000010 != 0 {
                            display_pos = if display_pos + 1 < n {
                                display_pos + 1
                            } else {
                                0
                            };
                            cur = display_order[display_pos];
                            redraw = true;
                        }
                    }
                }

                // B dismisses viewing_card / detail_mode
                if keys & 0x00000002 != 0 {
                    if viewing_card.is_some() {
                        viewing_card = None;
                        detail_mode = false;
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

                // DPAD LEFT/RIGHT: scroll hand view (0x10 = RIGHT, 0x20 = LEFT)
                if !detail_mode {
                    let vis = visible_hand_slots();
                    let is_p1 = gs.active_player().id == gs.player1.id;
                    let (off, max) = if is_p1 {
                        (hand_offset, gs.player1.hand.cards.len().saturating_sub(vis))
                    } else {
                        (
                            hand_offset_p2,
                            gs.player2.hand.cards.len().saturating_sub(vis),
                        )
                    };
                    if keys & 0x00000020 != 0 && off > 0 {
                        if is_p1 {
                            hand_offset -= 1;
                        } else {
                            hand_offset_p2 -= 1;
                        }
                        redraw = true;
                    }
                    if keys & 0x00000010 != 0 && off + vis < max + vis {
                        if is_p1 {
                            hand_offset += 1;
                        } else {
                            hand_offset_p2 += 1;
                        }
                        redraw = true;
                    }
                }

                // X toggles card detail mode + narrows action list to selected card
                if keys & 0x00000400 != 0 {
                    detail_mode = !detail_mode;
                    if detail_mode && cur < acts_cache.len() {
                        if let Some(cid) =
                            acts_cache[cur].parameters.as_ref().and_then(|p| p.card_id)
                        {
                            viewing_card = Some(cid);
                        }
                    } else if !detail_mode {
                        viewing_card = None;
                        unsafe {
                            _3ds_text_set_scroll_y(0);
                        }
                    }
                    redraw = true;
                }

                // Zone viewer controls
                if zone_viewer.is_some() {
                    let cols_vis = 5; // must match rendering
                    let pp = cols_vis * 2;
                    // B: close detail overlay first, then viewer
                    if keys & 0x00000002 != 0 {
                        if viewing_card.is_some() {
                            viewing_card = None;
                        } else {
                            zone_viewer = None;
                        }
                        redraw = true;
                    }
                    // A: show card detail for selected card
                    if keys & 0x00000001 != 0 && viewing_card.is_none() {
                        if let Some((_, ref cards)) = zone_viewer {
                            if zone_viewer_offset < cards.len() {
                                viewing_card = Some(cards[zone_viewer_offset]);
                                redraw = true;
                            }
                        }
                    }
                    // UP/DOWN: move cursor by row
                    if viewing_card.is_none() {
                        if keys & 0x00000040 != 0 {
                            zone_viewer_offset = if zone_viewer_offset >= cols_vis {
                                zone_viewer_offset - cols_vis
                            } else {
                                0
                            };
                            if zone_viewer_offset < zone_viewer_offset / pp * pp {
                                zone_viewer_offset = zone_viewer_offset / pp * pp;
                            }
                            redraw = true;
                        }
                        if keys & 0x00000080 != 0 {
                            let m = zone_viewer
                                .as_ref()
                                .map_or(0, |z| z.1.len().saturating_sub(1));
                            zone_viewer_offset = zone_viewer_offset.saturating_add(cols_vis).min(m);
                            redraw = true;
                        }
                        // LEFT/RIGHT: move cursor by 1
                        if keys & 0x00000020 != 0 {
                            if zone_viewer_offset > 0 {
                                zone_viewer_offset -= 1;
                                if zone_viewer_offset < zone_viewer_offset / pp * pp {
                                    zone_viewer_offset = zone_viewer_offset / pp * pp;
                                }
                            }
                            redraw = true;
                        }
                        if keys & 0x00000010 != 0 {
                            let m = zone_viewer
                                .as_ref()
                                .map_or(0, |z| z.1.len().saturating_sub(1));
                            if zone_viewer_offset < m {
                                zone_viewer_offset += 1;
                            }
                            redraw = true;
                        }
                    }
                }

                // R toggles choice image mode (board highlights vs text action list)
                if keys & 0x00000100 != 0 {
                    choice_image_mode = !choice_image_mode;
                    redraw = true;
                }

                // Y toggles CLI/game mode
                if keys & 0x00000800 != 0 {
                    cli_mode = !cli_mode;
                    unsafe {
                        _3ds_set_cli_mode(cli_mode);
                    }
                    redraw = true;
                }

                // Multiplayer: always try to receive data from opponent (non-blocking)
                if is_multiplayer {
                    let mut recv_buf = [0u8; 256];
                    if let Ok(n) = uds::uds_recv(&mut recv_buf) {
                        if n > 0 {
                            if let Some(sync) = uds::ActionSync::from_bytes(&recv_buf[..n]) {
                                let action_type = match sync.action_tag {
                                    0 => game_setup::ActionType::RockChoice,
                                    1 => game_setup::ActionType::PaperChoice,
                                    2 => game_setup::ActionType::ScissorsChoice,
                                    3 => game_setup::ActionType::ChooseFirstAttacker,
                                    4 => game_setup::ActionType::SelectMulligan,
                                    5 => game_setup::ActionType::SkipMulligan,
                                    6 => game_setup::ActionType::PlayMemberToStage,
                                    7 => game_setup::ActionType::SetLiveCard,
                                    8 => game_setup::ActionType::FinishLiveCardSet,
                                    9 => game_setup::ActionType::EnergyCharge,
                                    10 => game_setup::ActionType::ChoiceDecision,
                                    11 => game_setup::ActionType::ChoiceSelect,
                                    12 => game_setup::ActionType::ChoiceSkip,
                                    13 => game_setup::ActionType::ChoiceOption,
                                    14 => game_setup::ActionType::ChoicePosition,
                                    15 => game_setup::ActionType::UseAbility,
                                    16 => game_setup::ActionType::ChooseSecondAttacker,
                                    17 => game_setup::ActionType::ConfirmMulligan,
                                    18 => game_setup::ActionType::SelectLiveCard,
                                    19 => game_setup::ActionType::ConfirmLiveCardSet,
                                    20 => game_setup::ActionType::SkipLiveCardSet,
                                    21 => game_setup::ActionType::PassRemaining,
                                    22 => game_setup::ActionType::Pass,
                                    _ => game_setup::ActionType::Pass,
                                };
                                let stage_area = match sync.stage_area {
                                    1 => Some(rabuka_engine::zones::MemberArea::LeftSide),
                                    2 => Some(rabuka_engine::zones::MemberArea::Center),
                                    3 => Some(rabuka_engine::zones::MemberArea::RightSide),
                                    _ => None,
                                };
                                let _ = turn::TurnEngine::execute_main_phase_action(
                                    &mut gs,
                                    &action_type,
                                    sync.card_id,
                                    if sync.card_indices.is_empty() {
                                        None
                                    } else {
                                        Some(sync.card_indices.clone())
                                    },
                                    stage_area,
                                    if sync.use_baton_touch {
                                        Some(true)
                                    } else {
                                        None
                                    },
                                );
                                gs.reset_loop_detection();
                                let my_id = if is_host { 0 } else { 1 };
                                waiting_for_opponent = !mp_can_act(&gs, my_id);
                                cur = 0;
                                dirty = true;
                                redraw = true;
                            }
                        }
                    }
                }
                // If waiting for opponent or it's the AI's turn, skip local input
                if (is_multiplayer && waiting_for_opponent) || (*vs_ai && !mp_can_act(&gs, 0)) {
                    // Don't process local input while waiting
                } else
                // A button executes selected action (skip disabled actions, disabled in zone viewer).
                if zone_viewer.is_none() && keys & 0x00000001 != 0 && cur < acts_cache.len()
                {
                    let is_disabled = acts_cache[cur]
                        .parameters
                        .as_ref()
                        .and_then(|p| p.disabled)
                        .unwrap_or(false);
                    if is_disabled {
                        // Do nothing — disabled actions are not selectable
                    } else {
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
                        // In VS AI mode, after human picks RPS for P1, AI auto-picks for P2
                        if *vs_ai
                            && !*ai_vs_ai
                            && gs.current_phase == Phase::RockPaperScissors
                            && gs.player1_rps_choice.is_some()
                            && gs.player2_rps_choice.is_none()
                        {
                            let ai_choice = (unsafe { _3ds_system_tick() } as usize) % 3;
                            let ai_action = match ai_choice {
                                0 => game_setup::ActionType::RockChoice,
                                1 => game_setup::ActionType::PaperChoice,
                                _ => game_setup::ActionType::ScissorsChoice,
                            };
                            let _ = turn::TurnEngine::execute_main_phase_action(
                                &mut gs, &ai_action, None, None, None, None,
                            );
                            gs.reset_loop_detection();
                            // If both choices are None again, it was a draw
                            if gs.player1_rps_choice.is_none() && gs.player2_rps_choice.is_none() {
                                dprintln!("DRAW! Same choice — pick again.\n");
                            }
                        }
                        // Multiplayer: Send action to opponent
                        if is_multiplayer {
                            let my_player_id = if is_host { 0 } else { 1 };
                            let action_tag = match action.action_type {
                                game_setup::ActionType::RockChoice => 0u16,
                                game_setup::ActionType::PaperChoice => 1,
                                game_setup::ActionType::ScissorsChoice => 2,
                                game_setup::ActionType::ChooseFirstAttacker => 3,
                                game_setup::ActionType::ChooseSecondAttacker => 16,
                                game_setup::ActionType::SelectMulligan => 4,
                                game_setup::ActionType::SkipMulligan => 5,
                                game_setup::ActionType::ConfirmMulligan => 17,
                                game_setup::ActionType::PlayMemberToStage => 6,
                                game_setup::ActionType::SetLiveCard => 7,
                                game_setup::ActionType::FinishLiveCardSet => 8,
                                game_setup::ActionType::EnergyCharge => 9,
                                game_setup::ActionType::ChoiceDecision => 10,
                                game_setup::ActionType::ChoiceSelect => 11,
                                game_setup::ActionType::ChoiceSkip => 12,
                                game_setup::ActionType::ChoiceOption => 13,
                                game_setup::ActionType::ChoicePosition => 14,
                                game_setup::ActionType::UseAbility => 15,
                                game_setup::ActionType::SelectLiveCard => 18,
                                game_setup::ActionType::ConfirmLiveCardSet => 19,
                                game_setup::ActionType::SkipLiveCardSet => 20,
                                game_setup::ActionType::PassRemaining => 21,
                                game_setup::ActionType::Pass => 22,
                                _ => 0,
                            };
                            let stage_area = match p
                                .as_ref()
                                .and_then(|x| x.stage_area.as_ref())
                                .map(|s| s.as_str())
                            {
                                Some("left") => 1u8,
                                Some("center") => 2,
                                Some("right") => 3,
                                _ => 0,
                            };
                            let sync = uds::ActionSync {
                                action_tag,
                                card_id: p.as_ref().and_then(|x| x.card_id),
                                card_indices: p
                                    .as_ref()
                                    .and_then(|x| x.card_indices.clone())
                                    .unwrap_or_default(),
                                stage_area,
                                use_baton_touch: p
                                    .as_ref()
                                    .and_then(|x| x.use_baton_touch)
                                    .unwrap_or(false),
                                ability_index: None,
                            };
                            let data = sync.to_bytes();
                            let _ = uds::uds_send(&data);
                            waiting_for_opponent = !mp_can_act(&gs, my_player_id);
                        }
                        cur = 0;
                        dirty = true;
                        redraw = true;
                    } // closes else block (disabled action skip)
                }

                let n2 = acts_cache.len();
                if n2 > 0 && cur >= n2 {
                    cur = n2 - 1;
                }

                // AI: auto-pick when it's the AI's turn (before human input, covers all phases)
                // Skip when dirty=true: acts_cache is stale from a just-executed human action.
                // In multiplayer: opponent's turn is handled via UDS receive, not AI
                // Uses mp_can_act(gs, 0) which correctly handles pending choices (choice_player_id).
                let is_ai_turn = *ai_vs_ai || (*vs_ai && !mp_can_act(&gs, 0));
                if is_ai_turn && !dirty {
                    if acts_cache.len() > 0 {
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
                    }
                    acts_cache.clear();
                    cur = 0;
                    dirty = true;
                    redraw = true;
                }

                let auto = !gs.has_pending_choice()
                    && gs.game_result == GameResult::Ongoing
                    && game_setup::is_automatic_phase(&gs);
                if auto {
                    settle_3ds(&mut gs);
                    if is_multiplayer {
                        let my_id = if is_host { 0 } else { 1 };
                        waiting_for_opponent = !mp_can_act(&gs, my_id);
                    }
                    cur = 0;
                    dirty = true;
                }

                // Touch: tap board zones to view card details, or overlay to select action
                let touching = unsafe { _3ds_touch_down() };
                if touching && !was_touching {
                    touch_tap_count += 1;
                    let mut tx: u32 = 0;
                    let mut ty: u32 = 0;
                    unsafe {
                        _3ds_touch_read(&mut tx, &mut ty);
                    }
                    // Phase 2: tap action overlay to select action
                    if !cli_mode && ty < 240 && !acts_cache.is_empty() {
                        let n = acts_cache.len();
                        let max_vis = 8usize;
                        let half = max_vis / 2;
                        let start = if n > max_vis {
                            (cur as isize - half as isize)
                                .max(0)
                                .min((n - max_vis) as isize) as usize
                        } else {
                            0
                        };
                        let vis = (start + max_vis).min(n) - start;
                        let has_up = start > 0;
                        let has_down = (start + max_vis) < n;
                        let extra = (if has_up { 1 } else { 0 }) + (if has_down { 1 } else { 0 });
                        let oy = 240.0 - ((vis + extra) as f32 * 16.0 + 8.0) - 2.0;
                        let ox = 138.0;
                        if (tx as f32) >= ox
                            && (tx as f32) < (ox + 180.0)
                            && (ty as f32) >= oy
                            && (ty as f32) < (oy + (vis + extra) as f32 * 16.0 + 8.0)
                        {
                            let mut li = ((ty as f32 - oy - 4.0) / 16.0) as usize;
                            if has_up {
                                if li == 0 { /* ▲ marker, skip */
                                } else {
                                    li -= 1;
                                }
                            }
                            if li < vis && (start + li) < n {
                                cur = start + li;
                                redraw = true;
                                viewing_card = None;
                            }
                        }
                    }
                    if ty < 240 {
                        let view = unsafe { _3ds_board_current_view() };
                        let (p1y0, p1h): (i32, i32);
                        let (p2y0, p2h): (i32, i32);
                        if view == 2 {
                            p1y0 = 120;
                            p1h = 120;
                            p2y0 = 2;
                            p2h = 114;
                        } else if view == 1 {
                            p1y0 = 0;
                            p1h = 0;
                            p2y0 = 0;
                            p2h = 240;
                        } else {
                            p1y0 = 0;
                            p1h = 240;
                            p2y0 = 0;
                            p2h = 0;
                        }
                        let (y0, h) = if (ty as i32) >= p1y0 && (ty as i32) < (p1y0 + p1h) {
                            (p1y0, p1h)
                        } else if (ty as i32) >= p2y0 && (ty as i32) < (p2y0 + p2h) {
                            (p2y0, p2h)
                        } else {
                            (0, 0)
                        };
                        if h > 0 {
                            let pb = if y0 == p1y0 { &gs.player1 } else { &gs.player2 };
                            let is_opp = y0 != p1y0;
                            unsafe {
                                _3ds_board_set_section_rect(y0 as f32, h as f32, is_opp);
                            }
                            let hand_y = unsafe { _3ds_board_get_zone_y(3) };
                            let hand_h = unsafe { _3ds_board_get_zone_h(3) };
                            let stage_y = unsafe { _3ds_board_get_zone_y(1) };
                            let stage_h = unsafe { _3ds_board_get_zone_h(1) };
                            let live_y = unsafe { _3ds_board_get_zone_y(0) };
                            let live_h = unsafe { _3ds_board_get_zone_h(0) };
                            let vis = visible_hand_slots();
                            let hand_slot_w = unsafe { _3ds_board_get_slot_w(3) };
                            let st_slot_w = unsafe { _3ds_board_get_slot_w(1) };
                            let live_slot_w = unsafe { _3ds_board_get_slot_w(0) };
                            let tapped = if (ty as i32) >= hand_y && (ty as i32) < (hand_y + hand_h)
                            {
                                let idx = ((tx as f32 - 4.0) / (hand_slot_w + 2.0)) as usize;
                                let hoff = if is_opp { hand_offset_p2 } else { hand_offset };
                                let hand_idx = hoff + idx;
                                if idx < vis && hand_idx < pb.hand.cards.len() {
                                    Some(pb.hand.cards[hand_idx])
                                } else {
                                    None
                                }
                            } else if (ty as i32) >= stage_y && (ty as i32) < (stage_y + stage_h) {
                                let raw_idx = ((tx as f32 - 2.0) / (st_slot_w + 2.0)) as usize;
                                let idx = if is_opp { 2 - raw_idx } else { raw_idx };
                                if idx < 3 && pb.stage.stage[idx] != -1 {
                                    Some(pb.stage.stage[idx])
                                } else {
                                    None
                                }
                            } else if (ty as i32) >= live_y && (ty as i32) < (live_y + live_h) {
                                let idx = ((tx as f32 - 5.0) / (live_slot_w + 2.0)) as usize;
                                if idx < 3 && idx < pb.live_card_zone.cards.len() {
                                    Some(pb.live_card_zone.cards[idx])
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            // Utility zone tap (right of stage slots): open zone viewer
                            if tapped.is_none() {
                                let tx_f = tx as f32;
                                let ux = 2.0 + 3.0 * (st_slot_w + 2.0) + 5.0;
                                let uw = 320.0 - ux - 2.0;
                                let zoned = if (ty as i32) >= stage_y
                                    && (ty as i32) < (stage_y + stage_h)
                                    && tx_f >= ux
                                    && tx_f < ux + uw
                                {
                                    match ((tx_f - ux) / 36.0) as usize {
                                        2 => Some((
                                            "Waitroom".into(),
                                            pb.waitroom.cards.iter().copied().collect::<Vec<i16>>(),
                                        )),
                                        3 => Some((
                                            "Live Success".into(),
                                            pb.success_live_card_zone
                                                .cards
                                                .iter()
                                                .copied()
                                                .collect::<Vec<i16>>(),
                                        )),
                                        _ => None,
                                    }
                                } else {
                                    None
                                };
                                if let Some((zl, zc)) = zoned {
                                    viewing_card = None;
                                    zone_viewer = Some((zl, zc));
                                    zone_viewer_offset = 0;
                                    redraw = true;
                                }
                            }
                            if let Some(cid) = tapped {
                                // Choice image mode: board tap executes the choice directly
                                let handled = if choice_image_mode && gs.has_pending_choice() {
                                    let mut act_idx: Option<usize> =
                                        acts_cache.iter().position(|act| {
                                            act.parameters.as_ref().and_then(|p| p.card_id)
                                                == Some(cid)
                                                && matches!(
                                                    act.action_type,
                                                    game_setup::ActionType::ChoiceSelect
                                                        | game_setup::ActionType::ChoiceDecision
                                                )
                                        });
                                    if act_idx.is_none() {
                                        if let Some(c) = gs.get_pending_choice() {
                                            use rabuka_engine::ability::types::Choice;
                                            if let Choice::SelectAutoAbility { options, .. } = c {
                                                if let Some(opt_idx) = options
                                                    .iter()
                                                    .position(|o| o.card_id == Some(cid))
                                                {
                                                    act_idx = acts_cache.iter().position(|act| {
                                                        act.parameters.as_ref().and_then(|p| p.card_id) == Some(opt_idx as i16)
                                                            && act.action_type == game_setup::ActionType::ChoiceOption
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    if let Some(idx) = act_idx {
                                        let action = acts_cache[idx].clone();
                                        let p = action.parameters.clone();
                                        let result = turn::TurnEngine::execute_main_phase_action(
                                            &mut gs,
                                            &action.action_type,
                                            p.as_ref().and_then(|x| x.card_id),
                                            p.as_ref().and_then(|x| x.card_indices.clone()),
                                            p.as_ref().and_then(|x| {
                                                x.stage_area.as_ref().and_then(|s| s.parse().ok())
                                            }),
                                            p.as_ref().and_then(|x| x.use_baton_touch),
                                        );
                                        if let Err(ref e) = result {
                                            unsafe {
                                                _3ds_debug_print(
                                                    format!("[ERR] {}\n\0", e).as_ptr(),
                                                );
                                            }
                                        }
                                        gs.reset_loop_detection();
                                        cur = 0;
                                        dirty = true;
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };
                                if !handled {
                                    if Some(cid) == viewing_card {
                                        viewing_card = None;
                                        detail_mode = false;
                                    } else {
                                        viewing_card = Some(cid);
                                        detail_mode = true;
                                        if !acts_cache.is_empty() {
                                            if let Some(pos) = acts_cache.iter().position(|act| {
                                                act.parameters.as_ref().and_then(|p| p.card_id)
                                                    == Some(cid)
                                            }) {
                                                cur = pos;
                                            }
                                        }
                                    }
                                }
                                redraw = true;
                            } else {
                                viewing_card = None;
                            }
                        } // end h > 0 guard
                    }
                }
                was_touching = touching;

                if dirty || redraw {
                    acts_cache = game_setup::generate_possible_actions(&gs);

                    // Rebuild display order from freshly generated acts_cache.
                    display_order = {
                        let mut order: Vec<usize> = Vec::new();
                        for (i, act) in acts_cache.iter().enumerate() {
                            if act.action_type == game_setup::ActionType::Pass {
                                order.push(i);
                                break;
                            }
                        }
                        for (i, act) in acts_cache.iter().enumerate() {
                            if act.action_type == game_setup::ActionType::UseAbility {
                                order.push(i);
                            }
                        }
                        for (i, act) in acts_cache.iter().enumerate() {
                            if act.action_type != game_setup::ActionType::Pass
                                && act.action_type != game_setup::ActionType::UseAbility
                            {
                                order.push(i);
                            }
                        }
                        order
                    };
                    // When viewing a specific card, filter to actions linked to that card
                    if let Some(vcid) = viewing_card {
                        display_order.retain(|&fi| {
                            acts_cache[fi].parameters.as_ref().and_then(|p| p.card_id) == Some(vcid)
                        });
                        if !display_order.contains(&cur) {
                            cur = display_order.first().copied().unwrap_or(0);
                        }
                    }
                    display_pos = display_order.iter().position(|&fi| fi == cur).unwrap_or(0);

                    let p1 = &gs.player1;
                    let p2 = &gs.player2;
                    let ap = gs.active_player();

                    // Helper closures (shared by both modes)
                    let card_no = |cid: i16| -> Option<String> {
                        gs.card_database
                            .get_card(cid)
                            .map(|c| c.card_no.to_string())
                    };
                    let is_tapped = |cid: i16| -> bool {
                        gs.mods.orientation_modifiers.get(&cid).map(|o| o.as_str()) == Some("wait")
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
                    macro_rules! fill_player_board {
                        ($pb:expr,
                         $stage_fn:ident, $live_fn:ident,
                         $energy_fn:ident, $ecount_fn:ident,
                         $hand_fn:ident, $hcount_fn:ident,
                         $util_fn:ident,
                         $hoff:expr) => {{
                            let pb = $pb;
                            let st = &pb.stage.stage;
                            for i in 0..3 {
                                let cid = st[i];
                                let tapped = if cid != -1 { is_tapped(cid) } else { false };
                                set_slot($stage_fn, i as i32, cid, false, tapped);
                            }
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
                            let ec = &pb.energy_zone.cards;
                            let ecount = ec.len().min(30);
                            let e_active = pb.energy_zone.active_count();
                            unsafe {
                                $ecount_fn(ecount as i32);
                            }
                            for (i, cid) in ec.iter().enumerate() {
                                // Energy cards: tapped if position >= active_count (front = active)
                                let tapped = i >= e_active;
                                set_slot($energy_fn, i as i32, *cid, false, tapped);
                            }
                            for (i, cid) in ec.iter().enumerate().take(30) {
                                set_slot($energy_fn, i as i32, *cid, false, is_tapped(*cid));
                            }
                            let hc = &pb.hand.cards;
                            let vis = visible_hand_slots();
                            unsafe {
                                $hcount_fn(vis as i32);
                                _3ds_board_set_hand_scroll_info(
                                    vis as i32,
                                    $hoff as i32,
                                    hc.len() as i32,
                                );
                            }
                            for i in 0..vis {
                                let idx = $hoff + i;
                                if idx < hc.len() {
                                    set_slot($hand_fn, i as i32, hc[idx], false, false);
                                } else {
                                    unsafe {
                                        $hand_fn(
                                            i as i32,
                                            false,
                                            std::ptr::null(),
                                            0,
                                            false,
                                            false,
                                        );
                                    }
                                }
                            }
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
                        _3ds_board_set_utility,
                        hand_offset
                    );
                    if is_multiplayer {
                        // Hide opponent's hand in multiplayer
                        unsafe {
                            _3ds_board_set_opp_hand_count(0);
                            for i in 0..visible_hand_slots() as i32 {
                                _3ds_board_set_opp_hand(
                                    i,
                                    false,
                                    std::ptr::null(),
                                    0,
                                    false,
                                    false,
                                );
                            }
                        }
                        // Show opponent stage/live/energy normally
                        fill_player_board!(
                            p2,
                            _3ds_board_set_opp_stage,
                            _3ds_board_set_opp_live,
                            _3ds_board_set_opp_energy,
                            _3ds_board_set_opp_energy_count,
                            _3ds_board_set_opp_hand,
                            _3ds_board_set_opp_hand_count,
                            _3ds_board_set_opp_utility,
                            hand_offset_p2
                        );
                        // Re-clear opp hand after fill
                        unsafe {
                            _3ds_board_set_opp_hand_count(0);
                        }
                    } else {
                        fill_player_board!(
                            p2,
                            _3ds_board_set_opp_stage,
                            _3ds_board_set_opp_live,
                            _3ds_board_set_opp_energy,
                            _3ds_board_set_opp_energy_count,
                            _3ds_board_set_opp_hand,
                            _3ds_board_set_opp_hand_count,
                            _3ds_board_set_opp_utility,
                            hand_offset_p2
                        );
                    }

                    // Set HUD + active player (always, so toggle works smoothly)
                    {
                        let ap_label = if ap.id == p1.id { "P1\0" } else { "P2\0" };
                        let phase_str = format!("{:?}\0", gs.current_phase);
                        unsafe {
                            _3ds_board_set_hud(
                                gs.turn_number as i32,
                                phase_str.as_ptr(),
                                ap_label.as_ptr(),
                            );
                            _3ds_board_set_active_player(ap.id == p1.id);
                        }
                    }

                    // Game over: show winner on top screen
                    if gs.game_result != GameResult::Ongoing {
                        let winner = match gs.game_result {
                            GameResult::FirstAttackerWins => {
                                if gs.player1.is_first_attacker {
                                    "P1"
                                } else {
                                    "P2"
                                }
                            }
                            GameResult::SecondAttackerWins => {
                                if gs.player1.is_first_attacker {
                                    "P2"
                                } else {
                                    "P1"
                                }
                            }
                            _ => "Draw",
                        };
                        unsafe {
                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                            _3ds_top_queue_text(
                                4.0,
                                100.0,
                                COL_GOLD,
                                1.2f32,
                                format!("{} wins!\0", winner).as_ptr(),
                            );
                            _3ds_top_queue_text(
                                4.0,
                                140.0,
                                COL_LIGHT,
                                0.65f32,
                                format!(
                                    "Score: {} vs {}\0",
                                    gs.player1.success_live_card_zone.cards.len(),
                                    gs.player2.success_live_card_zone.cards.len()
                                )
                                .as_ptr(),
                            );
                            _3ds_top_queue_text(
                                4.0,
                                170.0,
                                COL_MED,
                                0.55f32,
                                "Press START to exit\0".as_ptr(),
                            );
                        }
                    }

                    if cli_mode {
                        // ===== CLI MODE: existing text-based rendering =====
                        unsafe {
                            _3ds_clear_top();
                        }
                        if detail_mode {
                            unsafe {
                                _3ds_text_set_scroll_y(0);
                            }
                            if cur < acts_cache.len() {
                                let act = &acts_cache[cur];
                                if let Some(ref p) = act.parameters {
                                    if let Some(cid) = p.card_id {
                                        if let Some(card) = gs.card_database.get_card(cid) {
                                            unsafe {
                                                _3ds_text_add_top(
                                                    format!("[{}] {}\n\0", card.card_no, card.name)
                                                        .as_ptr(),
                                                );
                                            }
                                            for ab in card.resolved_abilities() {
                                                let w = wrap_ability_text(&ab.full_text, 40);
                                                unsafe {
                                                    _3ds_text_add_top(
                                                        format!("{}\n\0", w).as_ptr(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            unsafe {
                                _3ds_text_add_top("[X]=back Y=game\0".as_ptr());
                            }
                        } else {
                            let ap_label = if ap.id == p1.id { "P1" } else { "P2" };
                            let touch_indicator =
                                if viewing_card.is_some() { "[T]" } else { "   " };
                            unsafe {
                                _3ds_text_add_top(
                                    format!(
                                        "Turn {} | {:?} | {}{} | taps:{}\n\0",
                                        gs.turn_number,
                                        gs.current_phase,
                                        ap_label,
                                        touch_indicator,
                                        touch_tap_count,
                                    )
                                    .as_ptr(),
                                );
                                _3ds_text_add_top(format!("P1 H:{} E:{}/{} D:{} W:{} L:{}  P2 H:{} E:{}/{} D:{} W:{} L:{}\n\0",
                                    p1.hand.cards.len(), p1.energy_zone.active_count(), p1.energy_zone.cards.len(),
                                    p1.main_deck.cards.len(), p1.waitroom.cards.len(), p1.success_live_card_zone.cards.len(),
                                    p2.hand.cards.len(), p2.energy_zone.active_count(), p2.energy_zone.cards.len(),
                                    p2.main_deck.cards.len(), p2.waitroom.cards.len(), p2.success_live_card_zone.cards.len(),
                                ).as_ptr());
                            }
                            if let Some(vcid) = viewing_card {
                                if let Some(card) = gs.card_database.get_card(vcid) {
                                    unsafe {
                                        _3ds_text_add_top(
                                            format!(
                                                "[{}] {}\n\0",
                                                card.card_no,
                                                wrap_text(&card.name, 30)
                                            )
                                            .as_ptr(),
                                        );
                                    }
                                    for ab in card.resolved_abilities() {
                                        let w = wrap_ability_text(&ab.full_text, 34);
                                        unsafe {
                                            _3ds_text_add_top(format!("{}\n\0", w).as_ptr());
                                        }
                                    }
                                    unsafe {
                                        _3ds_text_add_top("(tap slot to dismiss)\n\0".as_ptr());
                                    }
                                }
                            } else if let Some(entry) = gs.ability_queue.current_entry() {
                                let ab_text = wrap_ability_text(&entry.ability.full_text, 36);
                                unsafe {
                                    _3ds_text_add_top(
                                        format!(
                                            "[{}] {}\n\0",
                                            entry.card_no,
                                            ab_text.lines().next().unwrap_or("")
                                        )
                                        .as_ptr(),
                                    );
                                }
                                for line in ab_text.lines().skip(1) {
                                    unsafe {
                                        _3ds_text_add_top(format!("   {}\n\0", line).as_ptr());
                                    }
                                }
                            }
                            let is_ai_turn =
                                *ai_vs_ai || (*vs_ai && gs.active_player().id != gs.player1.id);
                            let is_opponent_turn_mp =
                                is_multiplayer && !mp_can_act(&gs, if is_host { 0 } else { 1 });
                            if is_ai_turn {
                                unsafe {
                                    _3ds_text_add_top("AI is thinking...\n\0".as_ptr());
                                }
                            } else if is_opponent_turn_mp {
                                unsafe {
                                    _3ds_text_add_top("Waiting for opponent...\n\0".as_ptr());
                                }
                            } else {
                                // Render grouped list using display_order
                                let n = display_order.len();
                                let max_vis = 6usize;
                                let half = max_vis / 2;
                                let start = if n > max_vis {
                                    (display_pos as isize - half as isize)
                                        .max(0)
                                        .min((n - max_vis) as isize)
                                        as usize
                                } else {
                                    0
                                };
                                let end = (start + max_vis).min(n);
                                if start > 0 {
                                    unsafe {
                                        _3ds_text_add_top(
                                            format!("\u{25b2} +{}\n\0", start).as_ptr(),
                                        );
                                    }
                                }
                                for di in start..end {
                                    let fi = display_order[di];
                                    let act = &acts_cache[fi];
                                    let prefix = if fi == cur { ">" } else { " " };
                                    let line = match act.action_type {
                                        game_setup::ActionType::Pass => "Pass".to_string(),
                                        game_setup::ActionType::PlayMemberToStage => {
                                            let name = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.card_name.clone())
                                                .unwrap_or_default();
                                            let cn = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.card_no.clone())
                                                .unwrap_or_default();
                                            let cost = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.base_cost)
                                                .unwrap_or(0);
                                            let area = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.stage_area.clone())
                                                .unwrap_or_default();
                                            let is_db = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.card_indices.as_ref())
                                                .map(|ci| ci.len() >= 2)
                                                .unwrap_or(false);
                                            let pfx = if is_db { ">>" } else { " " };
                                            format!(
                                                "[{}] {} c:{}  {} {}",
                                                cn, name, cost, pfx, area
                                            )
                                        }
                                        game_setup::ActionType::UseAbility => {
                                            let name = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.card_name.clone())
                                                .unwrap_or_default();
                                            let cost = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.base_cost)
                                                .unwrap_or(0);
                                            let area = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.stage_area.clone())
                                                .unwrap_or_default();
                                            let abil = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.source_ability.clone())
                                                .unwrap_or_default();
                                            let abil_short: String =
                                                abil.chars().take(36).collect();
                                            if cost > 0 {
                                                format!(
                                                    "[{}] {} {} c:{} {}",
                                                    cn_or_empty(act),
                                                    name,
                                                    area,
                                                    cost,
                                                    abil_short
                                                )
                                            } else {
                                                format!(
                                                    "[{}] {} {} {}",
                                                    cn_or_empty(act),
                                                    name,
                                                    area,
                                                    abil_short
                                                )
                                            }
                                        }
                                        _ => {
                                            act.description.lines().next().unwrap_or("").to_string()
                                        }
                                    };
                                    let desc_full = wrap_text(&line, 36);
                                    for (li, l) in desc_full.lines().enumerate() {
                                        if li == 0 {
                                            unsafe {
                                                _3ds_text_add_top(
                                                    format!("{}{}\n\0", prefix, l).as_ptr(),
                                                );
                                            }
                                        } else {
                                            unsafe {
                                                _3ds_text_add_top(format!("   {}\n\0", l).as_ptr());
                                            }
                                        }
                                    }
                                }
                                if end < n {
                                    unsafe {
                                        _3ds_text_add_top(
                                            format!("\u{25bc} +{}\n\0", n - end).as_ptr(),
                                        );
                                    }
                                }
                            }
                            let detail_hint = if cur < acts_cache.len() {
                                acts_cache[cur]
                                    .parameters
                                    .as_ref()
                                    .and_then(|p| p.card_id)
                                    .and_then(|cid| gs.card_database.get_card(cid))
                                    .and_then(|card| card.resolved_abilities().next())
                                    .map(|ab| {
                                        wrap_ability_text(&ab.full_text, 34)
                                            .lines()
                                            .next()
                                            .unwrap_or("")
                                            .to_string()
                                    })
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            unsafe {
                                _3ds_text_add_top(
                                    format!("[X]=detail Y=game {}\0", detail_hint).as_ptr(),
                                );
                            }
                        }
                    } else {
                        // ===== GAME MODE: graphical rendering =====
                        //
                        // FONT SCALING REFERENCE (citro2d BCFNT):
                        // The BCFNT font has native cellHeight=42px. citro2d normalizes
                        // this so that scale 1.0 always renders at 30px glyph height:
                        //   rendered_height = user_scale * (30.0 / cellHeight) * cellHeight
                        //                    = user_scale * 30.0
                        //
                        // Scale-to-pixel cheat sheet:
                        //   0.50 = 15px  (too small, was our old default)
                        //   0.60 = 18px  (barely readable)
                        //   0.65 = 20px  (minimum for body text)
                        //   0.70 = 21px  (good for deck list items)
                        //   0.75 = 23px  (menu items)
                        //   0.80 = 24px  (card names)
                        //   0.85 = 26px  (titles, CLI mode)
                        //   1.00 = 30px  (full size)
                        //
                        // Top screen: 400x240. Bottom screen: 320x240.
                        // Line advance ≈ ceil(scale * 0.714 * 31) pixels per line.
                        unsafe {
                            _3ds_top_clear();
                        }
                        // Top screen: stats bar (0-40px) + content panel (42-240px).
                        unsafe {
                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 40.0, COL_PANEL);
                            _3ds_top_queue_text(
                                4.0,
                                2.0,
                                COL_GOLD,
                                0.70f32,
                                format!(
                                    "T{} {:?} [{}]  P1 H:{} E:{}/{} D:{}  P2 H:{} E:{}/{} D:{}\0",
                                    gs.turn_number,
                                    gs.current_phase,
                                    if ap.id == p1.id { "P1" } else { "P2" },
                                    p1.hand.cards.len(),
                                    p1.energy_zone.active_count(),
                                    p1.energy_zone.cards.len(),
                                    p1.main_deck.cards.len(),
                                    p2.hand.cards.len(),
                                    p2.energy_zone.active_count(),
                                    p2.energy_zone.cards.len(),
                                    p2.main_deck.cards.len(),
                                )
                                .as_ptr(),
                            );
                            let p1_blade: u32 = gs
                                .player1
                                .stage
                                .stage
                                .iter()
                                .filter_map(|&cid| {
                                    if cid == -1 {
                                        return None;
                                    }
                                    let card = gs.card_database.get_card(cid)?;
                                    let is_wait = gs
                                        .mods
                                        .orientation_modifiers
                                        .get(&cid)
                                        .map(|o| o.as_str() == "wait")
                                        .unwrap_or(false);
                                    if is_wait {
                                        return Some(0u32);
                                    }
                                    let m = gs.mods.blade_modifiers.get(&cid);
                                    let total = if let Some(e) = m {
                                        if e.set != 0 {
                                            e.total().max(0) as u32
                                        } else {
                                            (card.blade as i32 + e.total()).max(0) as u32
                                        }
                                    } else {
                                        card.blade
                                    };
                                    Some(total)
                                })
                                .sum::<u32>();
                            _3ds_top_queue_text(
                                4.0,
                                22.0,
                                COL_LIGHT,
                                0.60f32,
                                format!(
                                    "W:{} L:{}  taps:{}  BL:{}\0",
                                    p1.waitroom.cards.len(),
                                    p1.success_live_card_zone.cards.len(),
                                    touch_tap_count,
                                    p1_blade,
                                )
                                .as_ptr(),
                            );
                        }

                        // Content panel:
                        //   detail_mode = full-screen card detail (blocks action list)
                        //   viewing_card = compact card info overlay + action list below
                        //   ability_queue = compact queue overlay + action list below
                        //   otherwise = action list only
                        let mut content_y: f32 = 42.0;

                        if let Some((ref zlabel, ref zcards)) = zone_viewer {
                            if viewing_card.is_none() {
                                let gap = 4.0f32;
                                let cols = 5usize;
                                let cw = ((400.0 - 8.0) / cols as f32) - gap;
                                let ch = cw / 0.711;
                                let rows = 1usize;
                                let pp = cols * rows;
                                let page = zone_viewer_offset / pp * pp;
                                let n = zcards.len();
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        4.0,
                                        4.0,
                                        COL_GOLD,
                                        0.70f32,
                                        format!("{}  (B=close, A=detail)\0", zlabel).as_ptr(),
                                    );
                                }
                                for i in page..n.min(page + pp) {
                                    let col = (i - page) % cols;
                                    let row = (i - page) / cols;
                                    let ix = 4.0 + col as f32 * (cw + gap);
                                    let iy = 28.0 + row as f32 * (ch + 14.0 + gap);
                                    let cid = zcards[i];
                                    let border = if i == zone_viewer_offset {
                                        COL_GOLD
                                    } else {
                                        COL_CARD
                                    };
                                    unsafe {
                                        _3ds_top_queue_rect(ix, iy, cw, ch + 14.0, border);
                                    }
                                    let cn = gs
                                        .card_database
                                        .get_card(cid)
                                        .map(|c| c.card_no.as_ref())
                                        .unwrap_or("?");
                                    if let Some((atl, idx)) = atlas.lookup(cn) {
                                        let c_str = std::ffi::CString::new(atl.as_bytes())
                                            .unwrap_or_default();
                                        unsafe {
                                            _3ds_top_queue_card(
                                                c_str.as_ptr(),
                                                *idx as i32,
                                                ix + 1.0,
                                                iy + 1.0,
                                                cw - 2.0,
                                                ch,
                                            );
                                            _3ds_top_queue_text(
                                                ix + 1.0,
                                                iy + ch + 1.0,
                                                COL_LIGHT,
                                                0.45f32,
                                                format!("{}\0", cn).as_ptr(),
                                            );
                                        }
                                    }
                                }
                                if n > pp {
                                    let page_n = page / pp + 1;
                                    let total_p = (n + pp - 1) / pp;
                                    unsafe {
                                        _3ds_top_queue_text(
                                            300.0,
                                            4.0,
                                            COL_MED,
                                            0.50f32,
                                            format!("{}/{}\0", page_n, total_p).as_ptr(),
                                        );
                                    }
                                }
                            } else {
                                // Active card detail overlay within zone viewer
                                let vcid = viewing_card.unwrap();
                                if let Some(card) = gs.card_database.get_card(vcid) {
                                    let is_tapped = gs
                                        .mods
                                        .orientation_modifiers
                                        .get(&vcid)
                                        .map(|o| o.as_str() == "wait")
                                        .unwrap_or(false);
                                    let bt = card.blade;
                                    let bm = gs
                                        .mods
                                        .blade_modifiers
                                        .get(&vcid)
                                        .map(|m| m.total())
                                        .unwrap_or(0);
                                    let tb = if is_tapped {
                                        0
                                    } else {
                                        (bt as i32 + bm).max(0)
                                    };
                                    let sc = card.score.unwrap_or(0) as i32
                                        + gs.mods
                                            .score_modifiers
                                            .get(&vcid)
                                            .map(|m| m.total())
                                            .unwrap_or(0);
                                    let co = card.cost.unwrap_or(0);
                                    let hr = card
                                        .base_heart
                                        .as_ref()
                                        .map(|bh| {
                                            bh.hearts
                                                .iter()
                                                .map(|(c, v)| {
                                                    let code = c.short_label();
                                                    let bonus = gs
                                                        .mods
                                                        .heart_modifiers
                                                        .get(&vcid)
                                                        .and_then(|hm| hm.get(c))
                                                        .map(|m| m.total())
                                                        .unwrap_or(0);
                                                    if bonus != 0 {
                                                        format!("{}{}+{}", code, v, bonus)
                                                    } else {
                                                        format!("{}{}", code, v)
                                                    }
                                                })
                                                .collect::<Vec<_>>()
                                                .join(" ")
                                        })
                                        .unwrap_or_default();
                                    unsafe {
                                        _3ds_top_queue_rect(40.0, 40.0, 320.0, 160.0, COL_CARD);
                                        _3ds_top_queue_text(
                                            44.0,
                                            44.0,
                                            COL_BLUE,
                                            0.70f32,
                                            format!("[{}] {}\0", card.card_no, card.name).as_ptr(),
                                        );
                                        _3ds_top_queue_text(
                                            44.0,
                                            66.0,
                                            COL_LIGHT,
                                            0.60f32,
                                            format!(
                                                "B:{} H:{} S:{} C:{}{}\0",
                                                tb,
                                                hr,
                                                sc,
                                                co,
                                                if is_tapped { " [TAP]" } else { "" }
                                            )
                                            .as_ptr(),
                                        );
                                        let mut ty = 86.0;
                                        for ab in card.resolved_abilities() {
                                            let w = wrap_ability_text(&ab.full_text, 38);
                                            for line in w.lines() {
                                                if ty < 190.0 {
                                                    _3ds_top_queue_text(
                                                        44.0,
                                                        ty,
                                                        COL_LIGHT,
                                                        0.55f32,
                                                        format!("{}\0", line).as_ptr(),
                                                    );
                                                    ty += 16.0;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else if detail_mode {
                            let (rect_h, cont_y) = if viewing_card.is_some() {
                                (110.0, 158.0)
                            } else {
                                (198.0, 300.0)
                            };
                            let detail_cid = viewing_card.or_else(|| {
                                acts_cache
                                    .get(cur)
                                    .and_then(|a| a.parameters.as_ref().and_then(|p| p.card_id))
                            });
                            if let Some(cid) = detail_cid {
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 42.0, 400.0, rect_h, COL_CARD);
                                        _3ds_top_queue_text(
                                            4.0,
                                            44.0,
                                            COL_BLUE,
                                            0.80f32,
                                            format!(
                                                "[{}] {}\0",
                                                card.card_no,
                                                wrap_text(&card.name, 25)
                                            )
                                            .as_ptr(),
                                        );
                                        let is_tapped = gs
                                            .mods
                                            .orientation_modifiers
                                            .get(&cid)
                                            .map(|o| o.as_str() == "wait")
                                            .unwrap_or(false);
                                        let base_blade = card.blade;
                                        let blade_mod = gs
                                            .mods
                                            .blade_modifiers
                                            .get(&cid)
                                            .map(|m| m.total())
                                            .unwrap_or(0);
                                        let total_blade = if is_tapped {
                                            0
                                        } else {
                                            (base_blade as i32 + blade_mod).max(0)
                                        };
                                        let score = card.score.unwrap_or(0) as i32
                                            + gs.mods
                                                .score_modifiers
                                                .get(&cid)
                                                .map(|m| m.total())
                                                .unwrap_or(0);
                                        let cost = card.cost.unwrap_or(0);
                                        let heart_str = card
                                            .base_heart
                                            .as_ref()
                                            .map(|bh| {
                                                let parts: Vec<String> = bh
                                                    .hearts
                                                    .iter()
                                                    .map(|(c, v)| {
                                                        let code = c.short_label();
                                                        let bonus = gs
                                                            .mods
                                                            .heart_modifiers
                                                            .get(&cid)
                                                            .and_then(|hm| hm.get(c))
                                                            .map(|m| m.total())
                                                            .unwrap_or(0);
                                                        if bonus != 0 {
                                                            format!("{}{}+{}", code, v, bonus)
                                                        } else {
                                                            format!("{}{}", code, v)
                                                        }
                                                    })
                                                    .collect();
                                                parts.join(" ")
                                            })
                                            .unwrap_or_default();
                                        let tap_str = if is_tapped { " [TAPPED]" } else { "" };
                                        _3ds_top_queue_text(
                                            4.0,
                                            66.0,
                                            COL_LIGHT,
                                            0.65f32,
                                            format!(
                                                "B:{}  H:{}  S:{}  C:{}{}\0",
                                                total_blade, heart_str, score, cost, tap_str
                                            )
                                            .as_ptr(),
                                        );
                                        let mut ty = 86.0;
                                        for ab in card.resolved_abilities() {
                                            let w = wrap_ability_text(&ab.full_text, 45);
                                            for line in w.lines() {
                                                if ty < 232.0 {
                                                    _3ds_top_queue_text(
                                                        4.0,
                                                        ty,
                                                        COL_LIGHT,
                                                        0.65f32,
                                                        format!("{}\0", line).as_ptr(),
                                                    );
                                                    ty += 18.0;
                                                }
                                            }
                                            ty += 3.0;
                                        }
                                    }
                                }
                            }
                            content_y = cont_y;
                        } else {
                            if let Some(vcid) = viewing_card {
                                // Compact card info overlay with stats
                                if let Some(card) = gs.card_database.get_card(vcid) {
                                    let is_tapped = gs
                                        .mods
                                        .orientation_modifiers
                                        .get(&vcid)
                                        .map(|o| o.as_str() == "wait")
                                        .unwrap_or(false);
                                    let base_blade = card.blade;
                                    let blade_mod = gs
                                        .mods
                                        .blade_modifiers
                                        .get(&vcid)
                                        .map(|m| m.total())
                                        .unwrap_or(0);
                                    let total_blade = if is_tapped {
                                        0
                                    } else {
                                        (base_blade as i32 + blade_mod).max(0)
                                    };
                                    let score = card.score.unwrap_or(0) as i32
                                        + gs.mods
                                            .score_modifiers
                                            .get(&vcid)
                                            .map(|m| m.total())
                                            .unwrap_or(0);
                                    let cost = card.cost.unwrap_or(0);
                                    let heart_str = card
                                        .base_heart
                                        .as_ref()
                                        .map(|bh| {
                                            let parts: Vec<String> = bh
                                                .hearts
                                                .iter()
                                                .map(|(c, v)| {
                                                    let code = c.short_label();
                                                    let bonus = gs
                                                        .mods
                                                        .heart_modifiers
                                                        .get(&vcid)
                                                        .and_then(|hm| hm.get(c))
                                                        .map(|m| m.total())
                                                        .unwrap_or(0);
                                                    if bonus != 0 {
                                                        format!("{}{}+{}", code, v, bonus)
                                                    } else {
                                                        format!("{}{}", code, v)
                                                    }
                                                })
                                                .collect();
                                            parts.join(" ")
                                        })
                                        .unwrap_or_default();
                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 42.0, 400.0, 76.0, COL_CARD);
                                        _3ds_top_queue_text(
                                            4.0,
                                            44.0,
                                            COL_BLUE,
                                            0.75f32,
                                            format!(
                                                "[{}] {}\0",
                                                card.card_no,
                                                wrap_text(&card.name, 30)
                                            )
                                            .as_ptr(),
                                        );
                                        let tap_str = if is_tapped { " TAP" } else { "" };
                                        _3ds_top_queue_text(
                                            4.0,
                                            64.0,
                                            COL_LIGHT,
                                            0.65f32,
                                            format!(
                                                "B:{}  H:{}  S:{}  C:{}{}\0",
                                                total_blade, heart_str, score, cost, tap_str
                                            )
                                            .as_ptr(),
                                        );
                                        if let Some(ab) = card.resolved_abilities().next() {
                                            let first_line = wrap_ability_text(&ab.full_text, 50)
                                                .lines()
                                                .next()
                                                .unwrap_or("")
                                                .to_string();
                                            _3ds_top_queue_text(
                                                4.0,
                                                82.0,
                                                COL_LIGHT,
                                                0.60f32,
                                                format!("{}\0", first_line).as_ptr(),
                                            );
                                        }
                                    }
                                }
                                content_y = 126.0;
                            } else if let Some(entry) = gs.ability_queue.current_entry() {
                                // Ability queue overlay with full text
                                let ab_lines: Vec<String> =
                                    wrap_ability_text(&entry.ability.full_text, 50)
                                        .lines()
                                        .take(4)
                                        .map(|l| l.to_string())
                                        .collect();
                                let n_lines = ab_lines.len();
                                let h = 22.0 + n_lines as f32 * 14.0;
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 42.0, 400.0, h, COL_ABILITY);
                                    _3ds_top_queue_text(
                                        4.0,
                                        44.0,
                                        COL_LIGHT,
                                        0.65f32,
                                        format!("[{}] {}\0", entry.card_no, ab_lines[0]).as_ptr(),
                                    );
                                    for (li, line) in ab_lines.iter().enumerate().skip(1) {
                                        _3ds_top_queue_text(
                                            8.0,
                                            44.0 + li as f32 * 16.0,
                                            COL_LIGHT,
                                            0.65f32,
                                            format!("{}\0", line).as_ptr(),
                                        );
                                    }
                                }
                                content_y = 42.0 + h + 6.0;
                            }
                        }

                        // Choice image mode: show card images where possible, text for rest
                        let has_card_imgs = choice_image_mode
                            && gs.has_pending_choice()
                            && display_order.iter().any(|&fi| {
                                let act = &acts_cache[fi];
                                let cid = act.parameters.as_ref().and_then(|p| p.card_id);
                                cid.and_then(card_no)
                                    .and_then(|cn| atlas.lookup(cn.as_str()))
                                    .is_some()
                            });
                        let is_image_choice =
                            choice_image_mode && gs.has_pending_choice() && has_card_imgs;
                        {
                            // Action list on top screen (was previously on bottom overlay).
                            let is_ai_turn =
                                *ai_vs_ai || (*vs_ai && gs.active_player().id != gs.player1.id);
                            let is_opponent_turn_mp =
                                is_multiplayer && !mp_can_act(&gs, if is_host { 0 } else { 1 });
                            if zone_viewer.is_none() {
                                if is_image_choice {
                                    let opt_map: std::collections::HashMap<i16, i16> = {
                                        let mut m = std::collections::HashMap::new();
                                        if let Some(c) = gs.get_pending_choice() {
                                            use rabuka_engine::ability::types::Choice;
                                            if let Choice::SelectAutoAbility { options, .. } = c {
                                                for (i, opt) in options.iter().enumerate() {
                                                    if let Some(cid) = opt.card_id {
                                                        m.insert(i as i16, cid);
                                                    }
                                                }
                                            }
                                        }
                                        m
                                    };
                                    let cw = 80.0f32;
                                    let ch = cw / 0.711;
                                    let gap = 4.0f32;
                                    let cols = ((400.0 - 8.0) / (cw + gap)) as usize;
                                    let row_h = ch + 20.0 + gap;
                                    let base_y = content_y.max(42.0);
                                    let rows_vis = ((230.0 - base_y) / row_h) as usize + 1;
                                    let total_rows = (display_order.len() + cols - 1) / cols;
                                    let sr = if cols > 0 && rows_vis > 0 {
                                        let c = display_pos / cols;
                                        let half = rows_vis / 2;
                                        if total_rows > rows_vis {
                                            c.saturating_sub(half).min(total_rows - rows_vis)
                                        } else {
                                            0
                                        }
                                    } else {
                                        0
                                    };
                                    let iy = base_y - sr as f32 * row_h;
                                    let mut ty = iy;
                                    for (di, &fi) in display_order.iter().enumerate() {
                                        let act = &acts_cache[fi];
                                        let is_disabled = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.disabled)
                                            .unwrap_or(false);
                                        let col = di % cols;
                                        let row = di / cols;
                                        let ix = 4.0 + col as f32 * (cw + gap);
                                        let iy_card = iy + row as f32 * (ch + 20.0 + gap);
                                        if iy_card + ch + 20.0 > 230.0 {
                                            break;
                                        }
                                        let real_cid = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.card_id)
                                            .and_then(|idx| opt_map.get(&idx).copied())
                                            .or_else(|| {
                                                act.parameters.as_ref().and_then(|p| p.card_id)
                                            });
                                        if let Some(cid) = real_cid {
                                            if let Some(cn) = card_no(cid) {
                                                if let Some((atl, idx)) = atlas.lookup(cn.as_str())
                                                {
                                                    let c_str =
                                                        std::ffi::CString::new(atl.as_bytes())
                                                            .unwrap_or_default();
                                                    let border = if di == display_pos {
                                                        COL_GOLD
                                                    } else {
                                                        COL_CARD
                                                    };
                                                    unsafe {
                                                        _3ds_top_queue_rect(
                                                            ix,
                                                            iy_card,
                                                            cw,
                                                            ch + 20.0,
                                                            border,
                                                        );
                                                        _3ds_top_queue_card(
                                                            c_str.as_ptr(),
                                                            *idx as i32,
                                                            ix + 1.0,
                                                            iy_card + 1.0,
                                                            cw - 2.0,
                                                            ch,
                                                        );
                                                        if is_disabled {
                                                            _3ds_top_queue_rect(
                                                                ix + 1.0,
                                                                iy_card + 1.0,
                                                                cw - 2.0,
                                                                ch,
                                                                0xAA000000,
                                                            );
                                                        }
                                                        _3ds_top_queue_text(
                                                            ix + 1.0,
                                                            iy_card + ch + 1.0,
                                                            COL_LIGHT,
                                                            0.50f32,
                                                            format!("{}\0", cn).as_ptr(),
                                                        );
                                                    }
                                                    ty = iy_card + ch + 20.0 + gap;
                                                    continue;
                                                }
                                            }
                                        }
                                        // Non-card actions (skip etc.): render as text
                                        let desc = act
                                            .description
                                            .lines()
                                            .next()
                                            .unwrap_or("")
                                            .to_string();
                                        if !desc.is_empty() {
                                            let c = if is_disabled { COL_MED } else { COL_LIGHT };
                                            unsafe {
                                                _3ds_top_queue_text(
                                                    4.0,
                                                    ty,
                                                    c,
                                                    0.65f32,
                                                    format!("{}\0", desc).as_ptr(),
                                                );
                                            }
                                            ty += 18.0;
                                        }
                                    }
                                } else if !is_ai_turn
                                    && !is_opponent_turn_mp
                                    && !display_order.is_empty()
                                    && content_y < 240.0
                                {
                                    let mut ty = content_y;
                                    let max_vis = ((230.0 - content_y) / 20.0) as usize + 1;
                                    let half = max_vis / 2;
                                    let n = display_order.len();
                                    let start = if n > max_vis {
                                        (display_pos as isize - half as isize)
                                            .max(0)
                                            .min((n - max_vis) as isize)
                                            as usize
                                    } else {
                                        0
                                    };
                                    let end = (start + max_vis).min(n);
                                    if start > 0 {
                                        unsafe {
                                            _3ds_top_queue_text(
                                                4.0,
                                                ty,
                                                COL_MED,
                                                0.60f32,
                                                format!("\u{25b2} +{}\0", start).as_ptr(),
                                            );
                                            ty += 18.0;
                                        }
                                    }
                                    let mut di = start;
                                    while di < end {
                                        let fi = display_order[di];
                                        let act = &acts_cache[fi];
                                        let is_sel = di == display_pos;
                                        let is_disabled = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.disabled)
                                            .unwrap_or(false);
                                        let this_cid = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.card_id)
                                            .unwrap_or(-1);
                                        let is_pmts = act.action_type
                                            == game_setup::ActionType::PlayMemberToStage;
                                        let mut ge = di + 1;
                                        if is_pmts && this_cid != -1 {
                                            while ge < end {
                                                let n = &acts_cache[display_order[ge]];
                                                if n.action_type
                                                    == game_setup::ActionType::PlayMemberToStage
                                                    && n.parameters.as_ref().and_then(|p| p.card_id)
                                                        == Some(this_cid)
                                                {
                                                    ge += 1;
                                                } else {
                                                    break;
                                                }
                                            }
                                        }
                                        let is_group = ge > di + 1;
                                        let group_sel =
                                            is_group && (di..ge).any(|i| i == display_pos);
                                        let line_color = if group_sel || is_sel {
                                            COL_GOLD
                                        } else if is_disabled {
                                            COL_MED
                                        } else {
                                            COL_LIGHT
                                        };
                                        let line_scale: f32 =
                                            if group_sel || is_sel { 0.70 } else { 0.65 };
                                        if ty > 230.0 {
                                            break;
                                        }
                                        if is_group {
                                            let cn = cn_or_empty(act);
                                            let name = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.card_name.clone())
                                                .unwrap_or_default();
                                            let base_cost = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.base_cost)
                                                .unwrap_or(0);
                                            let hdr = if !cn.is_empty() {
                                                format!("[{}] {} c:{}", cn, name, base_cost)
                                            } else {
                                                format!("{} c:{}", name, base_cost)
                                            };
                                            let mut areas = String::new();
                                            for i in di..ge {
                                                let gact = &acts_cache[display_order[i]];
                                                let area = gact
                                                    .parameters
                                                    .as_ref()
                                                    .and_then(|p| p.stage_area.clone())
                                                    .unwrap_or("?".into());
                                                let cost = gact
                                                    .parameters
                                                    .as_ref()
                                                    .and_then(|p| p.available_areas.as_ref())
                                                    .and_then(|aa| {
                                                        aa.iter()
                                                            .find(|x| x.area == area)
                                                            .map(|x| x.cost)
                                                    })
                                                    .or_else(|| {
                                                        gact.parameters
                                                            .as_ref()
                                                            .and_then(|p| p.base_cost)
                                                    })
                                                    .unwrap_or(0);
                                                if i == display_pos {
                                                    areas
                                                        .push_str(&format!("[{}:{}] ", area, cost));
                                                } else {
                                                    areas.push_str(&format!("{}:{} ", area, cost));
                                                }
                                            }
                                            for (li, l) in wrap_text(&hdr, 55).lines().enumerate() {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                unsafe {
                                                    _3ds_top_queue_text(
                                                        4.0,
                                                        ty,
                                                        line_color,
                                                        line_scale,
                                                        format!(
                                                            "{}{}\0",
                                                            if li == 0 {
                                                                if is_sel {
                                                                    "> "
                                                                } else {
                                                                    "  "
                                                                }
                                                            } else {
                                                                "   "
                                                            },
                                                            l
                                                        )
                                                        .as_ptr(),
                                                    );
                                                }
                                                ty += 20.0;
                                            }
                                            for (li, l) in wrap_text(&areas, 55).lines().enumerate()
                                            {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                unsafe {
                                                    _3ds_top_queue_text(
                                                        4.0,
                                                        ty,
                                                        line_color,
                                                        line_scale,
                                                        format!(
                                                            "{}{}\0",
                                                            if li == 0 {
                                                                if group_sel {
                                                                    "> "
                                                                } else {
                                                                    "  "
                                                                }
                                                            } else {
                                                                "   "
                                                            },
                                                            l
                                                        )
                                                        .as_ptr(),
                                                    );
                                                }
                                                ty += 20.0;
                                            }
                                            di = ge;
                                        } else {
                                            let prefix = if is_sel {
                                                "> "
                                            } else if is_disabled {
                                                "· "
                                            } else {
                                                "  "
                                            };
                                            let line = match act.action_type {
                                                game_setup::ActionType::Pass => "Pass".into(),
                                                game_setup::ActionType::PlayMemberToStage => {
                                                    let cn = cn_or_empty(act);
                                                    let name = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.card_name.clone())
                                                        .unwrap_or_default();
                                                    let base_cost = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.base_cost)
                                                        .unwrap_or(0);
                                                    if !cn.is_empty() {
                                                        format!("[{}] {} c:{}", cn, name, base_cost)
                                                    } else {
                                                        format!("{} c:{}", name, base_cost)
                                                    }
                                                }
                                                game_setup::ActionType::UseAbility => {
                                                    let name = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.card_name.clone())
                                                        .unwrap_or_default();
                                                    let cost = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.final_cost.or(p.base_cost))
                                                        .unwrap_or(0);
                                                    let area = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.stage_area.clone())
                                                        .unwrap_or_default();
                                                    let abil = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.source_ability.clone())
                                                        .unwrap_or_default();
                                                    let abil_short: String =
                                                        abil.chars().take(28).collect();
                                                    let cn = cn_or_empty(act);
                                                    if !cn.is_empty() {
                                                        if cost > 0 {
                                                            format!(
                                                                "[{}] {} {} c:{} {}",
                                                                cn, name, area, cost, abil_short
                                                            )
                                                        } else {
                                                            format!(
                                                                "[{}] {} {} {}",
                                                                cn, name, area, abil_short
                                                            )
                                                        }
                                                    } else {
                                                        if cost > 0 {
                                                            format!(
                                                                "{} {} c:{} {}",
                                                                name, area, cost, abil_short
                                                            )
                                                        } else {
                                                            format!(
                                                                "{} {} {}",
                                                                name, area, abil_short
                                                            )
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    let cn = cn_or_empty(act);
                                                    let name = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.card_name.clone())
                                                        .unwrap_or_default();
                                                    let desc = act
                                                        .description
                                                        .lines()
                                                        .next()
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let ability_text = if act.action_type
                                                        == game_setup::ActionType::ChoiceOption
                                                    {
                                                        gs.get_pending_choice().and_then(|c| {
                                                            use rabuka_engine::ability::types::Choice;
                                                            if let Choice::SelectAutoAbility { options, .. } = c {
                                                                act.parameters.as_ref().and_then(|p| p.card_id)
                                                                    .and_then(|idx| options.get(idx as usize))
                                                                    .map(|o| o.ability_text.clone())
                                                            } else { None }
                                                        }).unwrap_or_default()
                                                    } else {
                                                        String::new()
                                                    };
                                                    let display = if !ability_text.is_empty() {
                                                        &ability_text
                                                    } else {
                                                        &desc
                                                    };
                                                    if !cn.is_empty() && !name.is_empty() {
                                                        format!("[{}] {} {}", cn, name, display)
                                                    } else if !cn.is_empty() {
                                                        format!("[{}] {}", cn, display)
                                                    } else {
                                                        display.to_string()
                                                    }
                                                }
                                            };
                                            let color = if is_disabled {
                                                COL_MED
                                            } else if is_sel {
                                                COL_GOLD
                                            } else {
                                                COL_LIGHT
                                            };
                                            let scale: f32 = if is_sel { 0.70 } else { 0.65 };
                                            for (li, l) in wrap_text(&line, 55).lines().enumerate()
                                            {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                let pfx = if li == 0 { prefix } else { "   " };
                                                let txt = format!("{}{}", pfx, l);
                                                if txt.contains("{{") {
                                                    render_text_with_icons(
                                                        4.0, ty, &txt, color, scale, 14.0,
                                                    );
                                                } else {
                                                    unsafe {
                                                        _3ds_top_queue_text(
                                                            4.0,
                                                            ty,
                                                            color,
                                                            scale,
                                                            format!("{}\0", txt).as_ptr(),
                                                        );
                                                    }
                                                }
                                                ty += 20.0;
                                            }
                                            di += 1;
                                        }
                                    }
                                    if end < n {
                                        unsafe {
                                            _3ds_top_queue_text(
                                                4.0,
                                                ty,
                                                COL_MED,
                                                0.60f32,
                                                format!("\u{25bc} +{}\0", n - end).as_ptr(),
                                            );
                                        }
                                    }
                                }
                            } // closes if zone_viewer.is_none()
                        }

                        // Clear stale action highlight on bottom board
                        unsafe {
                            _3ds_board_clear_action_highlight();
                        }

                        // Choice image mode: highlight board cards for the pending choice
                        if choice_image_mode && gs.has_pending_choice() {
                            let mut card_ids: Vec<i16> = Vec::new();
                            let opt_map: Vec<(i16, i16)> = {
                                use rabuka_engine::ability::types::Choice;
                                let mut m = Vec::new();
                                if let Some(c) = gs.get_pending_choice() {
                                    if let Choice::SelectAutoAbility { options, .. } = c {
                                        for (i, opt) in options.iter().enumerate() {
                                            if let Some(cid) = opt.card_id {
                                                m.push((i as i16, cid));
                                            }
                                        }
                                    }
                                }
                                m
                            };
                            for act in &acts_cache {
                                if matches!(
                                    act.action_type,
                                    game_setup::ActionType::ChoiceSelect
                                        | game_setup::ActionType::ChoiceDecision
                                ) {
                                    if let Some(cid) =
                                        act.parameters.as_ref().and_then(|p| p.card_id)
                                    {
                                        if !card_ids.contains(&cid) {
                                            card_ids.push(cid);
                                        }
                                    }
                                }
                            }
                            if !opt_map.is_empty() {
                                for &(_, cid) in &opt_map {
                                    if !card_ids.contains(&cid) {
                                        card_ids.push(cid);
                                    }
                                }
                            }
                            for &cid in &card_ids {
                                if let Some((zone, slot)) = find_card_zone_slot(&gs, cid) {
                                    unsafe {
                                        _3ds_board_set_action_highlight(zone, slot);
                                    }
                                }
                            }
                        }
                    }

                    // Multiplayer debug overlay (last thing drawn, never cleared)
                    if zone_viewer.is_none() && is_multiplayer {
                        let my_id = if is_host { 0 } else { 1 };
                        let can_act = mp_can_act(&gs, my_id);
                        unsafe {
                            _3ds_top_queue_text(
                                4.0,
                                215.0,
                                0xFFFFFF00,
                                0.65f32,
                                format!(
                                    "MP|ap={} my={} can={} wait={} phase={:?} acts={}\0",
                                    gs.active_player().id.as_str(),
                                    if is_host { "HST" } else { "CLT" },
                                    if can_act { "Y" } else { "N" },
                                    if waiting_for_opponent { "W" } else { "A" },
                                    gs.current_phase,
                                    acts_cache.len(),
                                )
                                .as_ptr(),
                            );
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
                    *ai_vs_ai,
                    cli_mode,
                    detail_mode,
                    choice_image_mode,
                    hand_offset,
                    hand_offset_p2,
                    touch_tap_count,
                    viewing_card,
                    zone_viewer,
                    zone_viewer_offset,
                    was_touching,
                    is_multiplayer,
                    is_host,
                    waiting_for_opponent,
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
fn find_card_zone_slot(gs: &GameState, cid: i16) -> Option<(i32, i32)> {
    for p in [&gs.player1, &gs.player2] {
        if let Some(idx) = p.stage.stage.iter().position(|&id| id == cid) {
            return Some((1, idx as i32));
        }
        if let Some(idx) = p.hand.cards.iter().position(|&id| id == cid) {
            return Some((3, idx as i32));
        }
        if let Some(idx) = p
            .success_live_card_zone
            .cards
            .iter()
            .position(|&id| id == cid)
        {
            return Some((0, idx as i32));
        }
        if let Some(idx) = p.energy_zone.cards.iter().position(|&id| id == cid) {
            return Some((2, idx as i32));
        }
    }
    None
}

#[cfg(feature = "3ds")]
fn visible_hand_slots() -> usize {
    let hand_h = unsafe { _3ds_board_get_zone_h(3) as f32 };
    let card_h = (hand_h - 4.0).max(1.0);
    let hsw = card_h * 0.711;
    let stride = hsw + 1.0;
    let count = ((316.0 - hsw) / stride) as usize + 1;
    count.max(1).min(15)
}

#[cfg(feature = "3ds")]
fn step_name(s: &Step) -> &'static str {
    match s {
        Step::ReadCardsBin => "ReadCards",
        Step::ParseCards(_) => "ParseCards",
        Step::Setup(_, _, _, _) => "Setup",
        Step::Play(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => "Play",
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
    fn _3ds_keys_held() -> u32;
    fn _3ds_touch_read(px: *mut u32, py: *mut u32);
    fn _3ds_touch_down() -> bool;
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
    fn _3ds_board_set_hand_scroll_info(visible: i32, offset: i32, total: i32);
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
    fn _3ds_board_set_section_rect(y0: f32, h: f32, opponent: bool);
    fn _3ds_board_get_zone_y(zone_type: i32) -> i32;
    fn _3ds_board_get_zone_h(zone_type: i32) -> i32;
    fn _3ds_board_get_slot_w(zone_type: i32) -> f32;

    // Game-mode / CLI mode toggle
    fn _3ds_set_cli_mode(cli: bool);
    fn _3ds_is_cli_mode() -> bool;

    // Top screen graphical drawing (game mode)
    fn _3ds_top_clear();
    fn _3ds_top_queue_rect(x: f32, y: f32, w: f32, h: f32, color: u32);
    fn _3ds_top_queue_text(x: f32, y: f32, color: u32, scale: f32, text: *const u8);
    fn _3ds_top_queue_card(atlas: *const u8, idx: i32, x: f32, y: f32, w: f32, h: f32);

    // Board HUD
    fn _3ds_board_set_hud(turn: i32, phase: *const u8, player: *const u8);
    fn _3ds_board_set_active_player(is_p1: bool);

    // Action highlight on board slots
    fn _3ds_board_set_action_highlight(zone: i32, slot: i32);
    fn _3ds_board_clear_action_highlight();

    // Action overlay (Phase 2: actions on bottom screen, safe per-line copy)
    fn _3ds_board_set_action_overlay_state(count: i32, selected: i32);
    fn _3ds_board_set_action_overlay_text(index: i32, text: *const u8);
    fn _3ds_board_set_overlay_action_idx(display_line: i32, action_index: i32);
    fn _3ds_board_get_overlay_action_idx(display_line: i32) -> i32;
    fn _3ds_board_get_overlay_selected() -> i32;
    fn _3ds_board_clear_action_overlay();
    // QR code scanning (camera + quirc, same tech used by FBI installer)
    fn _3ds_qr_init() -> i32;
    fn _3ds_qr_exit();
    fn _3ds_qr_scan(out_text: *mut u8, out_max: u32) -> i32;
}

#[cfg(not(feature = "3ds"))]
fn main() {
    println!("Desktop mode - use: cargo run --bin harness");
}
