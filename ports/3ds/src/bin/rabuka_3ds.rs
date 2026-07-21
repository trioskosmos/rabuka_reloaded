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
    Setup(Arc<Vec<Card>>, Vec<DeckList>, SetupPhase, bool),
    Play(
        GameState,
        usize, // cursor
        Vec<game_setup::Action>,
        bool, // dirty
        bool, // redraw
        CardAtlas,
        bool,        // vs_ai
        bool,        // cli_mode
        bool,        // detail_mode
        usize,       // hand_offset (P1)
        usize,       // hand_offset_p2
        u32,         // touch_tap_count
        Option<i16>, // viewing_card_id
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
        unsafe {
            _3ds_scan_input();
        }
        let keys = unsafe { _3ds_keys_down() };
        let held = unsafe { _3ds_keys_held() };
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
                let new_step = match *phase {
                    SetupPhase::PickMode(cur) => unsafe {
                        if was_dirty {
                            if _3ds_is_cli_mode() {
                                _3ds_clear_top();
                                _3ds_text_add_top("SELECT MODE\n\0".as_ptr());
                                for (i, m) in ["Sandbox (2 players)", "VS AI"].iter().enumerate() {
                                    let arrow = if i == cur { ">" } else { " " };
                                    _3ds_text_add_top(
                                        format!("{} [{}] {}\n\0", arrow, i, m).as_ptr(),
                                    );
                                }
                                _3ds_text_add_top("\nUP/DOWN=select A=confirm\0".as_ptr());
                            } else {
                                // Game-mode: graphical mode selection screen on top LCD.
                                // Colors use c2d() byte order: 0xAABBGGRR.
                                //
                                // Font scaling reference (citro2d normalizes all fonts):
                                //   scale * 30.0 = rendered glyph height in pixels.
                                //   e.g. 0.70 = 21px, 0.85 = 26px, 1.0 = 30px (full).
                                //   Top screen is 400x240, bottom is 320x240.
                                _3ds_top_clear();
                                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                _3ds_top_queue_text(
                                    100.0,
                                    10.0,
                                    COL_GOLD,
                                    0.85f32,
                                    "SELECT MODE\0".as_ptr(),
                                );
                                for (i, m) in ["Sandbox (2 players)", "VS AI"].iter().enumerate() {
                                    let y = 50.0 + i as f32 * 90.0;
                                    let bg = if i == cur { COL_SEL } else { COL_DIM };
                                    unsafe {
                                        _3ds_top_queue_rect(40.0, y, 320.0, 65.0, bg);
                                    }
                                    if i == cur {
                                        unsafe {
                                            _3ds_top_queue_rect(
                                                40.0,
                                                y,
                                                320.0,
                                                65.0,
                                                COL_HIGHLIGHT,
                                            );
                                        }
                                    }
                                    let color = if i == cur { COL_GOLD } else { COL_LIGHT };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            50.0,
                                            y + 15.0,
                                            color,
                                            0.75f32,
                                            format!("{}\0", m).as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        50.0,
                                        225.0,
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
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur),
                                false,
                            )
                        }
                    },
                    SetupPhase::PickDeck(cur, vs_ai) => {
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
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck(cur, vs_ai),
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
                            let mut db = Arc::new(CardDatabase::load_or_create((**cards).clone()));
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
                                    false, // cli_mode (start in game mode)
                                    false, // detail_mode
                                    0,
                                    0,
                                    0,
                                    None,
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
                mut cli_mode,
                mut detail_mode,
                mut hand_offset,
                mut hand_offset_p2,
                mut touch_tap_count,
                mut viewing_card,
            ) => {
                // Build display order from flat action list.
                // display_order[i] = flat index of display item i.
                // This groups: Pass first, then play actions by card, then abilities, then others.
                let display_order: Vec<usize> = {
                    let mut order: Vec<usize> = Vec::new();
                    // Pass first
                    for (i, act) in acts_cache.iter().enumerate() {
                        if act.action_type == game_setup::ActionType::Pass {
                            order.push(i);
                            break;
                        }
                    }
                    // Play actions grouped by card_no
                    let mut seen: Vec<(String, Vec<usize>)> = Vec::new();
                    for (i, act) in acts_cache.iter().enumerate() {
                        if act.action_type == game_setup::ActionType::PlayMemberToStage {
                            let cn = act
                                .parameters
                                .as_ref()
                                .and_then(|p| p.card_no.clone())
                                .unwrap_or_default();
                            if let Some(e) = seen.iter_mut().find(|(c, _)| *c == cn) {
                                e.1.push(i);
                            } else {
                                seen.push((cn, vec![i]));
                            }
                        }
                    }
                    for (_, indices) in &seen {
                        for &ai in indices {
                            order.push(ai);
                        }
                    }
                    // Abilities
                    for (i, act) in acts_cache.iter().enumerate() {
                        if act.action_type == game_setup::ActionType::UseAbility {
                            order.push(i);
                        }
                    }
                    // Others
                    for (i, act) in acts_cache.iter().enumerate() {
                        if act.action_type != game_setup::ActionType::Pass
                            && act.action_type != game_setup::ActionType::PlayMemberToStage
                            && act.action_type != game_setup::ActionType::UseAbility
                        {
                            order.push(i);
                        }
                    }
                    order
                };
                // display_pos: cursor position in display_order (not flat index)
                let mut display_pos = display_order.iter().position(|&fi| fi == cur).unwrap_or(0);

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
                    // Navigate in display space (grouped order)
                    if keys & 0x00000040 != 0 && display_pos > 0 {
                        display_pos -= 1;
                        cur = display_order[display_pos];
                        redraw = true;
                    } else if keys & 0x00000080 != 0 && display_pos + 1 < display_order.len() {
                        display_pos += 1;
                        cur = display_order[display_pos];
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
                    } else if keys & 0x00000010 != 0 && off < max {
                        if is_p1 {
                            hand_offset += 1;
                        } else {
                            hand_offset_p2 += 1;
                        }
                        redraw = true;
                    }
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

                // Y toggles CLI/game mode
                if keys & 0x00000800 != 0 {
                    cli_mode = !cli_mode;
                    unsafe {
                        _3ds_set_cli_mode(cli_mode);
                    }
                    redraw = true;
                }

                // A button executes selected action.
                // cur is always the correct flat action index (mapped from display_pos).
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

                // AI: auto-pick when it's the AI's turn (before human input, covers all phases)
                let is_ai_turn = *vs_ai && gs.active_player().id != gs.player1.id;
                if is_ai_turn {
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
                    cur = 0;
                    dirty = true;
                }

                // Touch: tap board zones to view card details, or overlay to select action
                if unsafe { _3ds_touch_down() } {
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
                            if let Some(cid) = tapped {
                                if Some(cid) == viewing_card {
                                    viewing_card = None;
                                } else {
                                    viewing_card = Some(cid);
                                }
                                // Force redraw so the top-screen card-detail panel
                                // appears (game mode) or text updates (CLI mode).
                                redraw = true;
                            } else {
                                viewing_card = None;
                            }
                        } // end h > 0 guard
                    }
                }

                if dirty || redraw {
                    acts_cache = game_setup::generate_possible_actions(&gs);

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
                            unsafe {
                                $ecount_fn(ecount as i32);
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
                                                let w = wrap_text(&ab.full_text, 40);
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
                                        touch_tap_count
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
                                        let w = wrap_text(&ab.full_text, 34);
                                        unsafe {
                                            _3ds_text_add_top(format!("{}\n\0", w).as_ptr());
                                        }
                                    }
                                    unsafe {
                                        _3ds_text_add_top("(tap slot to dismiss)\n\0".as_ptr());
                                    }
                                }
                            } else if let Some(entry) = gs.ability_queue.current_entry() {
                                let ab_text = wrap_text(&entry.ability.full_text, 36);
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
                            let is_ai_turn = *vs_ai && gs.active_player().id != gs.player1.id;
                            if is_ai_turn {
                                unsafe {
                                    _3ds_text_add_top("AI is thinking...\n\0".as_ptr());
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
                                            let name = act.parameters.as_ref()
                                                .and_then(|p| p.card_name.clone())
                                                .unwrap_or_default();
                                            let cn = act.parameters.as_ref()
                                                .and_then(|p| p.card_no.clone())
                                                .unwrap_or_default();
                                            let cost = act.parameters.as_ref()
                                                .and_then(|p| p.final_cost)
                                                .unwrap_or(0);
                                            let area = act.parameters.as_ref()
                                                .and_then(|p| p.stage_area.clone())
                                                .unwrap_or_default();
                                            let is_db = act.parameters.as_ref()
                                                .and_then(|p| p.card_indices.as_ref())
                                                .map(|ci| ci.len() >= 2)
                                                .unwrap_or(false);
                                            let pfx = if is_db { ">>" } else { " " };
                                            format!("[{}] {} c:{}  {} {}", cn, name, cost, pfx, area)
                                        }
                                        game_setup::ActionType::UseAbility => {
                                            let name = act.parameters.as_ref()
                                                .and_then(|p| p.card_name.clone())
                                                .unwrap_or_default();
                                            let desc = act.description.lines().next().unwrap_or("");
                                            format!("ABIL {} {}", name, desc)
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
                                                _3ds_text_add_top(
                                                    format!("   {}\n\0", l).as_ptr(),
                                                );
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
                            } else {
                                // Build grouped display list for CLI mode too
                                let mut cli_display: Vec<(String, usize)> = Vec::new();
                                // Pass first
                                for (i, act) in acts_cache.iter().enumerate() {
                                    if act.action_type == game_setup::ActionType::Pass {
                                        cli_display.push(("Pass".into(), i));
                                        break;
                                    }
                                }
                                // Play actions grouped by card
                                let mut seen: Vec<(String, Vec<usize>)> = Vec::new();
                                for (i, act) in acts_cache.iter().enumerate() {
                                    if act.action_type == game_setup::ActionType::PlayMemberToStage
                                    {
                                        let cn = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.card_no.clone())
                                            .unwrap_or_default();
                                        if let Some(e) = seen.iter_mut().find(|(c, _)| *c == cn) {
                                            e.1.push(i);
                                        } else {
                                            seen.push((cn, vec![i]));
                                        }
                                    }
                                }
                                for (cn, indices) in &seen {
                                    let first = &acts_cache[indices[0]];
                                    let nm = first
                                        .parameters
                                        .as_ref()
                                        .and_then(|p| p.card_name.clone())
                                        .unwrap_or_default();
                                    let cs = first
                                        .parameters
                                        .as_ref()
                                        .and_then(|p| p.final_cost)
                                        .unwrap_or(0);
                                    cli_display
                                        .push((format!("[{}] {} c:{}", cn, nm, cs), indices[0]));
                                    for &ai in indices {
                                        let act = &acts_cache[ai];
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
                                        cli_display.push((format!("  {} {}", pfx, area), ai));
                                    }
                                }
                                // Abilities
                                for (i, act) in acts_cache.iter().enumerate() {
                                    if act.action_type == game_setup::ActionType::UseAbility {
                                        let nm = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.card_name.clone())
                                            .unwrap_or_default();
                                        let d = act.description.lines().next().unwrap_or("");
                                        cli_display.push((format!("ABIL {} {}", nm, d), i));
                                    }
                                }
                                // Others
                                for (i, act) in acts_cache.iter().enumerate() {
                                    if act.action_type != game_setup::ActionType::Pass
                                        && act.action_type
                                            != game_setup::ActionType::PlayMemberToStage
                                        && act.action_type != game_setup::ActionType::UseAbility
                                    {
                                        let d = act.description.lines().next().unwrap_or("");
                                        cli_display.push((d.to_string(), i));
                                    }
                                }
                                // Render grouped list
                                let n = cli_display.len();
                                let max_vis = 6usize;
                                let half = max_vis / 2;
                                // Find display position of current flat action
                                let display_cur = cli_display
                                    .iter()
                                    .position(|(_, fi)| *fi == cur)
                                    .unwrap_or(0);
                                let start = if n > max_vis {
                                    (display_cur as isize - half as isize)
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
                                    let fi = cli_display[di].1;
                                    let prefix = if fi == cur { ">" } else { " " };
                                    let desc_full = wrap_text(&cli_display[di].0, 36);
                                    for (li, line) in desc_full.lines().enumerate() {
                                        if li == 0 {
                                            unsafe {
                                                _3ds_text_add_top(
                                                    format!("{}{}\n\0", prefix, line).as_ptr(),
                                                );
                                            }
                                        } else {
                                            unsafe {
                                                _3ds_text_add_top(
                                                    format!("   {}\n\0", line).as_ptr(),
                                                );
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
                                        wrap_text(&ab.full_text, 34)
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
                        // FONT SCAPLING REFERENCE (citro2d BCFNT):
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
                        // Top screen in game mode: color-coordinated panels with proper \0 terminators.
                        // Stats panel bar at top showing turn/phase/active-player/HP/deck counts.
                        // Font scaling: scale * 30 = glyph height in px. 0.70 = 21px, 0.80 = 24px.
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
                            _3ds_top_queue_text(
                                4.0,
                                22.0,
                                COL_LIGHT,
                                0.60f32,
                                format!(
                                    "W:{} L:{}  taps:{}  Y=CLI  X=detail\0",
                                    p1.waitroom.cards.len(),
                                    p1.success_live_card_zone.cards.len(),
                                    touch_tap_count,
                                )
                                .as_ptr(),
                            );
                        }

                        // Card detail panel when a board card is tapped
                        if let Some(vcid) = viewing_card {
                            if let Some(card) = gs.card_database.get_card(vcid) {
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 42.0, 400.0, 198.0, COL_CARD);
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
                                    let mut ty = 66.0;
                                    for ab in card.resolved_abilities() {
                                        let w = wrap_text(&ab.full_text, 45);
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
                        // Ability queue entry when an ability is resolving
                        } else if let Some(entry) = gs.ability_queue.current_entry() {
                            unsafe {
                                let ab_text = wrap_text(&entry.ability.full_text, 40);
                                _3ds_top_queue_rect(0.0, 42.0, 400.0, 198.0, COL_ABILITY);
                                _3ds_top_queue_text(
                                    4.0,
                                    44.0,
                                    COL_PINK,
                                    0.75f32,
                                    format!(
                                        "[{}] {}\0",
                                        entry.card_no,
                                        ab_text.lines().next().unwrap_or("")
                                    )
                                    .as_ptr(),
                                );
                                let mut ty = 64.0;
                                for line in ab_text.lines().skip(1) {
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
                            }
                        }

                        // Action overlay on bottom screen: grouped by card for readability.
                        // Uses pre-computed display_order for consistent navigation.
                        let is_ai_turn = *vs_ai && gs.active_player().id != gs.player1.id;
                        if !is_ai_turn && !display_order.is_empty() {
                            // Build display text for each item in display_order
                            let mut display_texts: Vec<String> = Vec::new();
                            for &fi in &display_order {
                                let act = &acts_cache[fi];
                                match act.action_type {
                                    game_setup::ActionType::Pass => {
                                        display_texts.push("Pass".into());
                                    }
                                    game_setup::ActionType::PlayMemberToStage => {
                                        let name = act.parameters.as_ref()
                                            .and_then(|p| p.card_name.clone())
                                            .unwrap_or_default();
                                        let cn = act.parameters.as_ref()
                                            .and_then(|p| p.card_no.clone())
                                            .unwrap_or_default();
                                        let cost = act.parameters.as_ref()
                                            .and_then(|p| p.final_cost)
                                            .unwrap_or(0);
                                        let area = act.parameters.as_ref()
                                            .and_then(|p| p.stage_area.clone())
                                            .unwrap_or_default();
                                        let is_db = act.parameters.as_ref()
                                            .and_then(|p| p.card_indices.as_ref())
                                            .map(|ci| ci.len() >= 2)
                                            .unwrap_or(false);
                                        let area_info = act.parameters.as_ref()
                                            .and_then(|p| p.available_areas.as_ref())
                                            .and_then(|areas| areas.iter().find(|a| a.area == area))
                                            .map(|a| {
                                                if a.is_baton_touch {
                                                    format!("baton from {}", a.existing_member_name.as_deref().unwrap_or("?"))
                                                } else {
                                                    "open".into()
                                                }
                                            })
                                            .unwrap_or_default();
                                        // Check if this is the first action for this card (show header)
                                        let is_first_for_card = act.parameters.as_ref()
                                            .and_then(|p| p.card_no.clone())
                                            .map(|cn| {
                                                acts_cache.iter().enumerate()
                                                    .filter(|(_, a)| a.action_type == game_setup::ActionType::PlayMemberToStage
                                                        && a.parameters.as_ref().map(|p| p.card_no.as_deref()) == Some(Some(&cn)))
                                                    .map(|(i, _)| i)
                                                    .min()
                                                    == Some(fi)
                                            })
                                            .unwrap_or(false);
                                        if is_first_for_card {
                                            // Header line for this card
                                            display_texts.push(format!("[{}] {} c:{}", cn, name, cost));
                                        }
                                        // Area sub-line
                                        let pfx = if is_db { ">>" } else { " " };
                                        display_texts.push(format!("  {} {} ({})", pfx, area, area_info));
                                    }
                                    game_setup::ActionType::UseAbility => {
                                        let name = act.parameters.as_ref()
                                            .and_then(|p| p.card_name.clone())
                                            .unwrap_or_default();
                                        let desc = act.description.lines().next().unwrap_or("");
                                        display_texts.push(format!("ABIL {} {}", name, desc));
                                    }
                                    _ => {
                                        let desc = act.description.lines().next().unwrap_or("");
                                        display_texts.push(desc.to_string());
                                    }
                                }
                            }

                            // Send to C overlay with scrolling centered on display_pos
                            let n = display_order.len();
                            let max_vis = 8usize;
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
                            let has_up = start > 0;
                            let has_down = end < n;
                            let mut oi = 0i32;
                            if has_up {
                                if let Ok(s) = std::ffi::CString::new(format!("\u{25b2} +{}", start)) {
                                    unsafe {
                                        _3ds_board_set_action_overlay_text(oi, s.as_ptr() as *const u8);
                                        _3ds_board_set_overlay_action_idx(oi, -1);
                                        oi += 1;
                                    }
                                }
                            }
                            let mut selected_oi = 0i32;
                            for di in start..end {
                                let flat_idx = display_order[di];
                                let is_selected = di == display_pos;
                                let prefix = if is_selected { ">" } else { " " };
                                let text = format!("{}{}", prefix, display_texts[di]);
                                if is_selected { selected_oi = oi; }
                                if let Ok(s) = std::ffi::CString::new(text) {
                                    unsafe {
                                        _3ds_board_set_action_overlay_text(oi, s.as_ptr() as *const u8);
                                        _3ds_board_set_overlay_action_idx(oi, flat_idx as i32);
                                        oi += 1;
                                    }
                                }
                            }
                            if has_down {
                                if let Ok(s) = std::ffi::CString::new(format!("\u{25bc} +{}", n - end)) {
                                    unsafe {
                                        _3ds_board_set_action_overlay_text(oi, s.as_ptr() as *const u8);
                                        _3ds_board_set_overlay_action_idx(oi, -1);
                                        oi += 1;
                                    }
                                }
                            }

                            unsafe {
                                _3ds_board_set_action_overlay_state(oi, selected_oi);
                            }
                        } else {
                            unsafe {
                                _3ds_board_clear_action_overlay();
                            }
                        }
                            }

                            // 2. Group play actions by card_no
                            let mut seen_cards: Vec<(String, Vec<usize>)> = Vec::new();
                            for (i, act) in acts_cache.iter().enumerate() {
                                if act.action_type == game_setup::ActionType::PlayMemberToStage {
                                    let card_no = act
                                        .parameters
                                        .as_ref()
                                        .and_then(|p| p.card_no.clone())
                                        .unwrap_or_default();
                                    if let Some(entry) =
                                        seen_cards.iter_mut().find(|(c, _)| c == &card_no)
                                    {
                                        entry.1.push(i);
                                    } else {
                                        seen_cards.push((card_no, vec![i]));
                                    }
                                }
                            }
                            for (card_no, indices) in &seen_cards {
                                let first = &acts_cache[indices[0]];
                                let name = first
                                    .parameters
                                    .as_ref()
                                    .and_then(|p| p.card_name.clone())
                                    .unwrap_or_default();
                                let cost = first
                                    .parameters
                                    .as_ref()
                                    .and_then(|p| p.final_cost)
                                    .unwrap_or(0);
                                // Header: "[NO] Name cost:N"
                                display.push((
                                    format!("[{}] {} c:{}", card_no, name, cost),
                                    indices[0],
                                ));
                                // Sub-lines for each area option
                                for &ai in indices {
                                    let act = &acts_cache[ai];
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
                                    let prefix = if is_db { "  >>" } else { "   " };
                                    let area_info = act
                                        .parameters
                                        .as_ref()
                                        .and_then(|p| p.available_areas.as_ref())
                                        .and_then(|areas| areas.iter().find(|a| a.area == area))
                                        .map(|a| {
                                            if a.is_baton_touch {
                                                format!(
                                                    "baton from {}",
                                                    a.existing_member_name
                                                        .as_deref()
                                                        .unwrap_or("?")
                                                )
                                            } else {
                                                "open".into()
                                            }
                                        })
                                        .unwrap_or_default();
                                    display
                                        .push((format!("{} {} ({})", prefix, area, area_info), ai));
                                    // Show double baton pairs inline
                                    if is_db {
                                        if let Some(pairs) = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.double_baton_pairs.as_ref())
                                        {
                                            for pair in pairs {
                                                if pair.placement == area {
                                                    display.push((
                                                        format!(
                                                            "     dbl {}&{} c:{}",
                                                            pair.areas[0], pair.areas[1], pair.cost
                                                        ),
                                                        ai,
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // 3. Ability actions
                            for (i, act) in acts_cache.iter().enumerate() {
                                if act.action_type == game_setup::ActionType::UseAbility {
                                    let name = act
                                        .parameters
                                        .as_ref()
                                        .and_then(|p| p.card_name.clone())
                                        .unwrap_or_default();
                                    let desc = act.description.lines().next().unwrap_or("");
                                    display.push((format!("ABIL {} {}", name, desc), i));
                                }
                            }

                            // 4. Other actions (choices, etc.)
                            for (i, act) in acts_cache.iter().enumerate() {
                                if act.action_type != game_setup::ActionType::Pass
                                    && act.action_type != game_setup::ActionType::PlayMemberToStage
                                    && act.action_type != game_setup::ActionType::UseAbility
                                {
                                    let desc = act.description.lines().next().unwrap_or("");
                                    display.push((desc.to_string(), i));
                                }
                            }

                            // Send to C overlay with scrolling
                            let n = display.len();
                            let max_vis = 8usize;
                            let half = max_vis / 2;
                            let start = if n > max_vis {
                                (cur.min(n - 1) as isize - half as isize)
                                    .max(0)
                                    .min((n - max_vis) as isize)
                                    as usize
                            } else {
                                0
                            };
                            let end = (start + max_vis).min(n);
                            let has_up = start > 0;
                            let has_down = end < n;
                            let mut oi = 0i32;
                            if has_up {
                                if let Ok(s) =
                                    std::ffi::CString::new(format!("\u{25b2} +{}", start))
                                {
                                    unsafe {
                                        _3ds_board_set_action_overlay_text(
                                            oi,
                                            s.as_ptr() as *const u8,
                                        );
                                        _3ds_board_set_overlay_action_idx(oi, -1);
                                        oi += 1;
                                    }
                                }
                            }
                            let mut selected_oi = 0i32;
                            for di in start..end {
                                let flat_idx = display[di].1;
                                let is_selected = flat_idx == cur;
                                let prefix = if is_selected { ">" } else { " " };
                                let text = format!("{}{}", prefix, display[di].0);
                                if is_selected {
                                    selected_oi = oi;
                                }
                                if let Ok(s) = std::ffi::CString::new(text) {
                                    unsafe {
                                        _3ds_board_set_action_overlay_text(
                                            oi,
                                            s.as_ptr() as *const u8,
                                        );
                                        _3ds_board_set_overlay_action_idx(oi, flat_idx as i32);
                                        oi += 1;
                                    }
                                }
                            }
                            if has_down {
                                if let Ok(s) =
                                    std::ffi::CString::new(format!("\u{25bc} +{}", n - end))
                                {
                                    unsafe {
                                        _3ds_board_set_action_overlay_text(
                                            oi,
                                            s.as_ptr() as *const u8,
                                        );
                                        _3ds_board_set_overlay_action_idx(oi, -1);
                                        oi += 1;
                                    }
                                }
                            }

                            unsafe {
                                _3ds_board_set_action_overlay_state(oi, selected_oi);
                            }
                        } else {
                            unsafe {
                                _3ds_board_clear_action_overlay();
                            }
                        }

                        // Clear stale action highlight
                        unsafe {
                            _3ds_board_clear_action_highlight();
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
                    cli_mode,
                    detail_mode,
                    hand_offset,
                    hand_offset_p2,
                    touch_tap_count,
                    viewing_card,
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
fn visible_hand_slots() -> usize {
    let hand_h = 240.0 * 0.42;
    let card_h = hand_h - 4.0;
    let hsw = card_h * 0.711;
    let stride = hsw + 2.0;
    let count = ((314.0 - hsw) / stride) as usize + 1;
    count.max(1).min(15)
}

#[cfg(feature = "3ds")]
fn step_name(s: &Step) -> &'static str {
    match s {
        Step::ReadCardsBin => "ReadCards",
        Step::ParseCards(_) => "ParseCards",
        Step::Setup(_, _, _, _) => "Setup",
        Step::Play(_, _, _, _, _, _, _, _, _, _, _, _, _) => "Play",
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
}

#[cfg(not(feature = "3ds"))]
fn main() {
    println!("Desktop mode - use: cargo run --bin harness");
}
