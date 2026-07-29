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
use rabuka_engine::deck_parser::DeckEntry;
use rabuka_engine::deck_parser::DeckList;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

#[cfg(feature = "3ds")]
use rabuka_3ds::i18n;
#[cfg(feature = "3ds")]
use rabuka_3ds::i18n::Lang;
#[cfg(feature = "3ds")]
use rabuka_3ds::uds;
const TICK_HZ: u64 = 268_120_000;

/// Current UI language. Default: Japanese. Toggled via START menu.
#[cfg(feature = "3ds")]
static mut CURRENT_LANG: Lang = Lang::Japanese;

#[cfg(feature = "3ds")]
fn current_lang() -> Lang {
    unsafe { CURRENT_LANG }
}

#[cfg(feature = "3ds")]
fn set_lang(lang: Lang) {
    unsafe {
        CURRENT_LANG = lang;
    }
}

/// Shorthand for translating a key in the current language.
#[cfg(feature = "3ds")]
fn tl(key: &str) -> String {
    i18n::t(key, current_lang())
}

/// Shorthand for formatting a translated key with params.
#[cfg(feature = "3ds")]
fn tl_fmt(key: &str, params: &[(&str, &str)]) -> String {
    i18n::t_fmt(key, current_lang(), params)
}

/// Translate stage area labels for display.
#[cfg(feature = "3ds")]
fn tl_area(area: &str) -> &str {
    if current_lang() == Lang::Japanese {
        match area {
            "left" => "左",
            "center" => "センター",
            "right" => "右",
            other => other,
        }
    } else {
        match area {
            "left" => "Left",
            "center" => "Center",
            "right" => "Right",
            other => other,
        }
    }
}

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

/// Word-wrap text to fit within `max_px` pixels per line at the given font scale.
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
/// Uses _3ds_measure_text_width (citro2d C2D_TextGetDimensions) for exact pixel positioning.
fn render_text_with_icons(x: f32, y: f32, text: &str, color: u32, scale: f32) {
    let text_h = scale * 30.0;
    let icon_h = (scale * 16.0).max(11.0);
    let icon_y = y + (text_h - icon_h) / 2.0;
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
            let text_seg = std::ffi::CString::new(&rest[..start]).unwrap_or_default();
            cx += unsafe { _3ds_measure_text_width(text_seg.as_ptr() as *const u8, scale) };
        }
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let inner = &after[..end];
            if let Some(bar) = inner.find('|') {
                let file = &inner[..bar];
                let iw = icon_width_for(file, icon_h);
                let icon_name = file.strip_suffix(".png").unwrap_or(file);
                let atlas_name = format!("icon_{}.png.t3x", icon_name);
                let c_str = std::ffi::CString::new(atlas_name.as_str()).unwrap_or_default();
                unsafe {
                    _3ds_top_queue_card(c_str.as_ptr() as *const u8, 0, cx, icon_y, iw, icon_h);
                }
                cx += iw + scale * 6.0;
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

/// Look up the display width for an icon at the given height, maintaining aspect ratio.
fn icon_width_for(file: &str, h: f32) -> f32 {
    // Known icon dimensions (from docs/img/texticon/*.png)
    // Square icons (160x160): hearts, blades, energy, score, etc.
    // Rectangular: triggers and position icons
    let (w, ih) = match file {
        // Triggers: kidou=378x160, jidou=427x160, jyouji=378x160, toujyou=378x160
        f if f.starts_with("kidou") || f.starts_with("jyouji") || f.starts_with("toujyou") => {
            (378.0, 160.0)
        }
        f if f.starts_with("jidou") => (427.0, 160.0),
        // Live start/success: 833x160
        f if f.starts_with("live_start") || f.starts_with("live_success") => (833.0, 160.0),
        // Position: 667x160 for center, 833x204 for leftside, 833x202 for rightside
        f if f.starts_with("center") => (667.0, 160.0),
        f if f.starts_with("leftside") => (833.0, 204.0),
        f if f.starts_with("rightside") => (833.0, 202.0),
        // Turn1: 677x160, Turn2: 300x60
        f if f.starts_with("turn1") => (677.0, 160.0),
        f if f.starts_with("turn2") => (300.0, 60.0),
        // Everything else (hearts, resources): 160x160 square
        _ => (160.0, 160.0),
    };
    h * w / ih
}

/// Convert heart label like "h061+1" to icon + count: "{{heart_06.png|h06}} 1+1"
fn heart_label_to_icon(s: &str) -> String {
    if s.len() >= 3 && s.as_bytes()[0] == b'h' {
        if let Ok(n) = s[1..3].parse::<u8>() {
            if n <= 6 {
                let rest = &s[3..];
                let count_str = if rest.is_empty() {
                    String::new()
                } else {
                    format!(" {}", rest)
                };
                return format!("{{{{heart_{:02}.png|{}}}}}{}", n, &s[..3], count_str);
            }
        }
    }
    match s {
        "b_all" => "{{icon_b_all.png|ALL}}".into(),
        "draw" => "{{icon_draw.png|DRAW}}".into(),
        "score" => "{{icon_score.png|SCORE}}".into(),
        "all" => "{{icon_all.png|ALL}}".into(),
        _ => s.to_string(),
    }
}

/// Build heart string from a HeartMap, sorted by heart index (h00 first).
fn build_heart_str(
    hearts: &rabuka_engine::card::HeartMap,
    card_id: i16,
    mods: &rabuka_engine::core::game_modifiers::GameModifiers,
    is_need: bool,
) -> String {
    let mut entries: Vec<(u8, String)> = hearts
        .iter()
        .map(|(c, v)| {
            let code = c.short_label();
            let num = if code.len() >= 3 && code.as_bytes()[0] == b'h' {
                code[1..3].parse::<u8>().unwrap_or(99)
            } else {
                99
            };
            let bonus = if is_need {
                mods.need_heart_modifiers
                    .get(&card_id)
                    .and_then(|hm| hm.get(c))
                    .map(|m| m.total())
                    .unwrap_or(0)
            } else {
                mods.heart_modifiers
                    .get(&card_id)
                    .and_then(|hm| hm.get(c))
                    .map(|m| m.total())
                    .unwrap_or(0)
            };
            let label = if bonus != 0 {
                format!("{}{}+{}", code, v, bonus)
            } else {
                format!("{}{}", code, v)
            };
            (num, label)
        })
        .collect();
    entries.sort_by_key(|e| e.0);
    entries
        .into_iter()
        .map(|e| e.1)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build stat line with texticons for card detail.
/// Member: blade, hearts, energy. Live: score, need_heart, energy.
fn card_stat_line(
    blade: i32,
    heart_str: &str,
    score: i32,
    cost: u32,
    tapped: bool,
    variant: &str,
    need_heart_str: &str,
) -> String {
    let mut s = String::new();
    if variant == "member_card" {
        // Member: energy → hearts → blade
        if cost > 0 {
            s.push_str(&format!("{{{{icon_energy.png|E}}}}{}  ", cost));
        }
        for part in heart_str.split_whitespace() {
            let converted = heart_label_to_icon(part);
            if !converted.starts_with("{{") && heart_str.len() <= 3 {
                continue;
            }
            s.push_str(&converted);
            s.push(' ');
        }
        if !heart_str.is_empty() {
            s = s.trim_end().to_string();
            s.push_str("  ");
        }
        if blade > 0 {
            s.push_str(&format!("{{{{icon_blade.png|BLADE}}}}{}", blade));
        }
    } else if variant == "live_card" {
        // Live: score → need_heart
        if score > 0 {
            s.push_str(&format!("{{{{icon_score.png|SCORE}}}}{}  ", score));
        }
        for part in need_heart_str.split_whitespace() {
            let converted = heart_label_to_icon(part);
            if !converted.starts_with("{{") && need_heart_str.len() <= 3 {
                continue;
            }
            s.push_str(&converted);
            s.push(' ');
        }
        if !need_heart_str.is_empty() {
            s = s.trim_end().to_string();
        }
    }
    if tapped {
        s.push_str(" [TAPPED]");
    }
    s
}

/// Measure text pixel width using the 3DS system font (exact, proportional).
fn measure_text_width(s: &str, scale: f32) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    let c_str = std::ffi::CString::new(s).unwrap_or_default();
    unsafe { _3ds_measure_text_width(c_str.as_ptr() as *const u8, scale) }
}

/// Pre-computed card display stats for detail rendering.
struct CardDisplayStats {
    is_tapped: bool,
    total_blade: i32,
    score: i32,
    cost: u32,
    heart_str: String,
    need_heart_str: String,
}

fn compute_card_stats(
    card: &rabuka_engine::card::Card,
    cid: i16,
    gs: &rabuka_engine::game_state::GameState,
) -> CardDisplayStats {
    let is_tapped = gs
        .mods
        .orientation_modifiers
        .get(&cid)
        .map(|o| o.as_str() == "wait")
        .unwrap_or(false);
    let bm = gs
        .mods
        .blade_modifiers
        .get(&cid)
        .map(|m| m.total())
        .unwrap_or(0);
    let total_blade = if is_tapped {
        0
    } else {
        (card.blade as i32 + bm).max(0)
    };
    let score = card.score.unwrap_or(0) as i32
        + gs.mods
            .score_modifiers
            .get(&cid)
            .map(|m| m.total())
            .unwrap_or(0);
    let cost = card.cost.unwrap_or(0);
    let heart_str = build_heart_str(
        &card
            .base_heart
            .as_ref()
            .map(|bh| bh.hearts.clone())
            .unwrap_or_default(),
        cid,
        &gs.mods,
        false,
    );
    let need_heart_str = build_heart_str(
        &card
            .need_heart
            .as_ref()
            .map(|bh| bh.hearts.clone())
            .unwrap_or_default(),
        cid,
        &gs.mods,
        true,
    );
    CardDisplayStats {
        is_tapped,
        total_blade,
        score,
        cost,
        heart_str,
        need_heart_str,
    }
}

/// Find a break point in a string so the prefix fits within max_px pixels.
/// Tries space first, then binary-searches exact character boundary.
fn split_at_px(s: &str, max_px: f32, scale: f32) -> (String, String) {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return (String::new(), String::new());
    }
    // Estimate chars that fit (avg CJK ~12px at scale 1.0), search backward for space
    let est = (max_px / (scale * 12.0)).ceil() as usize;
    let search_end = est.min(chars.len());
    if search_end > 0 {
        let sub: String = chars[..search_end].iter().collect();
        if let Some(space) = sub.rfind(' ') {
            let w = measure_text_width(&sub[..space], scale);
            if w <= max_px {
                let rest = s[space + 1..].trim_start();
                return (sub[..space].to_string(), rest.to_string());
            }
        }
    }
    // Binary search for exact pixel break point
    let mut lo = 1usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let sub: String = chars[..mid].iter().collect();
        let w = measure_text_width(&sub, scale);
        if w <= max_px {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let split_byte: usize = chars[..lo].iter().map(|c| c.len_utf8()).sum();
    let (part, rest) = s.split_at(split_byte);
    (part.to_string(), rest.to_string())
}

/// Wrap ability text — keeps `{{...}}` icon markers for later inline rendering.
fn wrap_ability_text(s: &str, max_px: f32, scale: f32) -> String {
    wrap_text(s, max_px, scale)
}

/// Truncate text at segment boundaries, keeping `{{...}}` icon markers intact.
/// Only plain text characters count toward the character limit.
fn truncate_aware_segments(s: &str, max_chars: usize) -> String {
    let segs = segment_text(s);
    let mut result = String::new();
    let mut text_count = 0usize;
    for seg in &segs {
        match seg {
            TextSeg::Icon(icon) => {
                result.push_str(&format!("{{{{{}}}}}", icon));
            }
            TextSeg::Text(text) => {
                let chars: Vec<char> = text.chars().collect();
                let remaining = max_chars.saturating_sub(text_count);
                if remaining == 0 {
                    break;
                }
                if chars.len() <= remaining {
                    result.push_str(text);
                    text_count += chars.len();
                } else {
                    let truncated: String = chars.into_iter().take(remaining).collect();
                    result.push_str(&truncated);
                    text_count += remaining;
                    break;
                }
            }
        }
    }
    result
}

/// Segments of text: either a plain text span or an icon marker like {{icon.png|alt}}
#[derive(Debug)]
enum TextSeg {
    Text(String),
    Icon(String),
}

/// Split text into segments, treating `{{...}}` markers as atomic units.
fn segment_text(s: &str) -> Vec<TextSeg> {
    let mut segs = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find("{{") {
        if start > 0 {
            segs.push(TextSeg::Text(rest[..start].to_string()));
        }
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            segs.push(TextSeg::Icon(after[..end].to_string()));
            rest = &after[end + 2..];
        } else {
            segs.push(TextSeg::Text(rest[start..].to_string()));
            break;
        }
    }
    if !rest.is_empty() {
        segs.push(TextSeg::Text(rest.to_string()));
    }
    segs
}

fn wrap_text(s: &str, max_px: f32, scale: f32) -> String {
    if max_px <= 0.0 {
        return s.to_string();
    }
    let icon_h = (scale * 16.0).max(11.0);
    let mut out = String::with_capacity(s.len() + 32);
    for line in s.lines() {
        let segs = segment_text(line);
        let mut line_out = String::new();
        let mut line_w: f32 = 0.0;
        for seg in &segs {
            match seg {
                TextSeg::Text(t) => {
                    let mut remaining: String = t.clone();
                    while !remaining.is_empty() {
                        let seg_w = measure_text_width(&remaining, scale);
                        if line_w + seg_w <= max_px {
                            line_out.push_str(&remaining);
                            line_w += seg_w;
                            break;
                        } else if line_w == 0.0 {
                            let (part, rest) = split_at_px(&remaining, max_px, scale);
                            line_out.push_str(&part);
                            out.push_str(line_out.trim_end());
                            out.push('\n');
                            line_out.clear();
                            line_w = 0.0;
                            remaining = rest.trim_start().to_string();
                        } else {
                            out.push_str(line_out.trim_end());
                            out.push('\n');
                            line_out.clear();
                            line_w = 0.0;
                        }
                    }
                }
                TextSeg::Icon(icon) => {
                    let iw = if let Some(bar) = icon.find('|') {
                        icon_width_for(&icon[..bar], icon_h) + scale * 6.0
                    } else {
                        2.0 * scale * 9.0
                    };
                    if line_w + iw > max_px && line_w > 0.0 {
                        out.push_str(line_out.trim_end());
                        out.push('\n');
                        line_out.clear();
                        line_w = 0.0;
                    }
                    line_out.push_str("{{");
                    line_out.push_str(icon);
                    line_out.push_str("}}");
                    line_w += iw;
                }
            }
        }
        if !line_out.is_empty() {
            out.push_str(line_out.trim_end());
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
    #[allow(dead_code)]
    Testing, // On-device test suite
    // Multiplayer lobby phases
    MultiplayerDeck(usize), // cursor, selecting deck for multiplayer
    MultiplayerPickRole(usize, usize), // deck_idx, role_cursor (0=Host, 1=Client)
    MultiplayerHostWait(usize), // p1_idx: host waiting for client to connect
    MultiplayerClientScan(usize), // p1_idx: client scanning for host network
    MultiplayerClientHostSelect(usize, Vec<u16>, usize), // p1_idx, host_node_ids, cursor
    MultiplayerSyncDeck(usize, usize, bool), // p1_idx, p2_idx, is_host
    MultiplayerLoading(usize, usize, bool, Option<Vec<u8>>), // p1_idx, p2_idx, is_host, deck_sync_bytes
    QrScan,                                                  // QR code scanning for deck import
    QrResult(Vec<String>),                                   // QR scan result, user can confirm
}

#[cfg(feature = "3ds")]
#[derive(Clone, Copy, PartialEq)]
enum Overlay {
    None,
    StartMenu(usize),
    GameLog(usize),
    PerfStats(Option<usize>),
    RevealedCards(bool, usize),
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
        bool,                       // choice_subview (false=choices grid, true=text overlay)
        usize,                      // text_page (current page index in text subview)
        usize,                      // choice_grid_offset (scroll offset for choice image grid)
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
        Overlay,                    // overlay (start menu, game log, perf stats, revealed)
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
        match rabuka_engine::game_setup::test_ai_vs_ai(&cards_vec, &decks[0], &decks[1], 5) {
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
#[allow(dead_code)]
const fn c2d(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
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
#[allow(dead_code)]
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
    unsafe {
        _3ds_audio_init();
        _3ds_audio_play_ogg(b"romfs:/next_card.ogg\0".as_ptr());
    }
    i18n::init();

    let mut _frame: u64 = 0;
    let mut step = Step::ReadCardsBin;

    while unsafe { _3ds_main_loop() != 0 } {
        unsafe {
            _3ds_scan_input();
        }
        let keys = unsafe { _3ds_keys_down() };
        let _held = unsafe { _3ds_keys_held() };
        // START exits everywhere except during gameplay (where it opens the menu)
        if keys & 0x00000008 != 0 && !matches!(step, Step::Play(..)) {
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
                                _3ds_text_add_top(format!("{}\n\0", tl("SELECT MODE")).as_ptr());
                                for (i, m) in
                                    ["VS AI", "Sandbox (2 players)", "QR Scan", "Local MP"]
                                        .iter()
                                        .enumerate()
                                {
                                    let arrow = if i == cur { ">" } else { " " };
                                    _3ds_text_add_top(
                                        format!("{} [{}] {}\n\0", arrow, i, tl(m)).as_ptr(),
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
                                    format!("{}\0", tl("SELECT MODE")).as_ptr(),
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
                                    let label = tl(m);
                                    unsafe {
                                        _3ds_top_queue_text(
                                            50.0,
                                            y + 6.0,
                                            color,
                                            0.65f32,
                                            format!("{}\0", label).as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        50.0,
                                        230.0,
                                        COL_MED,
                                        0.60f32,
                                        format!("{}\0", tl("UP/DOWN=select  A=confirm")).as_ptr(),
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
                        } else if keys & 0x00000100 != 0 {
                            set_lang(current_lang().toggle());
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur),
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
                            let label = if !vs_ai {
                                tl("P1 DECK")
                            } else {
                                tl("YOUR DECK")
                            };
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                }
                                unsafe {
                                    _3ds_text_add_top(format!("{}\n\0", label).as_ptr());
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
                                        format!("{}\0", tl("UP/DOWN=select  A=confirm")).as_ptr(),
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
                                let deck_hdr = tl("SELECT YOUR DECK");
                                unsafe {
                                    _3ds_text_add_top(format!("{}\n\0", deck_hdr).as_ptr());
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
                                    let deck_hdr = tl("SELECT YOUR DECK");
                                    _3ds_top_queue_text(
                                        80.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        format!("{}\0", deck_hdr).as_ptr(),
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
                                        format!("{}\0", tl("UP/DOWN=select  A=confirm")).as_ptr(),
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
                                let deck_hdr = tl("SELECT P2 DECK");
                                unsafe {
                                    _3ds_text_add_top(format!("{}\n\0", deck_hdr).as_ptr());
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
                                    let deck_hdr = tl("SELECT P2 DECK");
                                    _3ds_top_queue_text(
                                        80.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        format!("{}\0", deck_hdr).as_ptr(),
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
                                        format!("{}\0", tl("A=select  B=use same")).as_ptr(),
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
                                    false, // choice_subview (false=choices grid)
                                    0,     // text_page
                                    0,     // choice_grid_offset
                                    0,     // hand_offset
                                    0,     // hand_offset_p2
                                    0,     // touch_tap_count
                                    None,  // viewing_card
                                    None,  // zone_viewer
                                    0,     // zone_viewer_offset
                                    false, // was_touching
                                    false, // is_multiplayer
                                    false, // is_host
                                    false, // waiting_for_opponent
                                    Overlay::None,
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
                            unsafe {
                                _3ds_qr_start();
                            }
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                    _3ds_text_add_top(format!("{}\n\0", tl("QR SCAN")).as_ptr());
                                    _3ds_text_add_top(
                                        format!(
                                            "{}\n{}\0",
                                            tl("Point camera at QR code"),
                                            tl("B=cancel")
                                        )
                                        .as_ptr(),
                                    );
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    let qr_hdr = tl("QR SCAN");
                                    _3ds_top_queue_text(
                                        120.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        format!("{}\0", qr_hdr).as_ptr(),
                                    );
                                    let qr_msg = tl("Point camera at deck QR code");
                                    _3ds_top_queue_text(
                                        40.0,
                                        60.0,
                                        COL_LIGHT,
                                        0.70f32,
                                        format!("{}\0", qr_msg).as_ptr(),
                                    );
                                    let qr_auto = tl("Auto-detects when QR is visible");
                                    _3ds_top_queue_text(
                                        40.0,
                                        85.0,
                                        COL_MED,
                                        0.65f32,
                                        format!("{}\0", qr_auto).as_ptr(),
                                    );
                                    let qr_cancel = tl("B=cancel");
                                    _3ds_top_queue_text(
                                        40.0,
                                        230.0,
                                        COL_MED,
                                        0.60f32,
                                        format!("{}\0", qr_cancel).as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000002 != 0 {
                            unsafe {
                                _3ds_qr_stop();
                            }
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(5), true)
                        } else {
                            let mut buf = [0u8; 2048];
                            let r = unsafe { _3ds_qr_poll(buf.as_mut_ptr(), buf.len() as u32) };
                            if r > 0 {
                                unsafe {
                                    _3ds_qr_stop();
                                }
                                let text = String::from_utf8_lossy(&buf[..r as usize]).to_string();
                                let cards_read = DeckParser::parse_deck_content(&text);
                                if cards_read.is_empty() {
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::QrScan,
                                        true,
                                    )
                                } else {
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::QrResult(cards_read),
                                        true,
                                    )
                                }
                            } else if r < -10 {
                                // Fatal error (alloc failed, quirc failed, re-arm failed)
                                unsafe {
                                    _3ds_clear_both();
                                    _3ds_text_add_top(
                                        format!(
                                            "{}\n\0",
                                            tl_fmt("Camera error", &[("e", &r.to_string())])
                                        )
                                        .as_ptr(),
                                    );
                                    _3ds_text_add_top(format!("{}\0", tl("B=back")).as_ptr());
                                }
                                unsafe {
                                    _3ds_qr_stop();
                                }
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::PickMode(5),
                                    true,
                                )
                            } else {
                                Step::Setup(cards.clone(), decks.clone(), SetupPhase::QrScan, false)
                            }
                        }
                    }
                    SetupPhase::QrResult(cards_read) => {
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                    _3ds_text_add_top(format!("{}\n\0", tl("QR DECK")).as_ptr());
                                    for c in cards_read.iter().take(20) {
                                        _3ds_text_add_top(format!("  {}\n\0", c).as_ptr());
                                    }
                                    _3ds_text_add_top(
                                        format!("\n{} cards\nA=use  B=discard\0", cards_read.len())
                                            .as_ptr(),
                                    );
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        120.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        format!("{}\0", tl("QR DECK")).as_ptr(),
                                    );
                                    _3ds_top_queue_text(
                                        200.0,
                                        32.0,
                                        COL_LIGHT,
                                        0.70f32,
                                        format!("{} cards imported\0", cards_read.len()).as_ptr(),
                                    );
                                    // Count unique cards
                                    let mut counts: HashMap<String, u32> = HashMap::new();
                                    for c in cards_read.iter() {
                                        *counts.entry(c.clone()).or_insert(0) += 1;
                                    }
                                    let mut sorted: Vec<_> = counts.into_iter().collect();
                                    sorted.sort_by(|a, b| a.0.cmp(&b.0));
                                    let mut y = 55.0f32;
                                    for (card_no, qty) in sorted.iter().take(15) {
                                        _3ds_top_queue_text(
                                            40.0,
                                            y,
                                            COL_LIGHT,
                                            0.60f32,
                                            format!("{} x {}\0", card_no, qty).as_ptr(),
                                        );
                                        y += 11.0;
                                    }
                                    _3ds_top_queue_text(
                                        40.0,
                                        230.0,
                                        COL_MED,
                                        0.60f32,
                                        format!("{}\0", tl("A=use deck  B=discard")).as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000002 != 0 {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(5), true)
                        } else if keys & 0x00000001 != 0 {
                            // Build a DeckList from the scanned cards and add to decks list
                            let entry_map =
                                cards_read
                                    .iter()
                                    .fold(HashMap::<&str, u32>::new(), |mut m, c| {
                                        *m.entry(c.as_str()).or_insert(0) += 1;
                                        m
                                    });
                            let entries: Vec<_> = entry_map
                                .into_iter()
                                .map(|(card_no, qty)| DeckEntry {
                                    card_no: card_no.to_string(),
                                    quantity: qty,
                                })
                                .collect();
                            let qr_deck = DeckList {
                                name: "QR Scanned".to_string(),
                                entries,
                            };
                            let mut new_decks = decks.clone();
                            let dlen = new_decks.len();
                            new_decks.push(qr_deck);
                            Step::Setup(
                                cards.clone(),
                                new_decks,
                                SetupPhase::PickDeck(dlen, true, false),
                                true,
                            )
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::QrResult(cards_read.clone()),
                                false,
                            )
                        }
                    }
                    // Multiplayer: Host or Client?
                    SetupPhase::MultiplayerPickRole(deck_idx, cur) => {
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                    let mp_hdr = tl("MULTIPLAYER");
                                    _3ds_text_add_top(format!("{}\n\0", mp_hdr).as_ptr());
                                    let host_net = tl("Host = create network");
                                    _3ds_text_add_top(format!("{}\n\0", host_net).as_ptr());
                                    let client_net = tl("Client = join network");
                                    _3ds_text_add_top(format!("{}\n\n\0", client_net).as_ptr());
                                    let arrow_h = if cur == 0 { ">" } else { " " };
                                    let arrow_c = if cur == 1 { ">" } else { " " };
                                    let host_label = tl("Host");
                                    let client_label = tl("Client");
                                    _3ds_text_add_top(
                                        format!("{} {}\n\0", arrow_h, host_label).as_ptr(),
                                    );
                                    _3ds_text_add_top(
                                        format!("{} {}\n\0", arrow_c, client_label).as_ptr(),
                                    );
                                    _3ds_text_add_top("\nUP/DOWN=select A=confirm\0".as_ptr());
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    let mp_hdr = tl("MULTIPLAYER");
                                    _3ds_top_queue_text(
                                        100.0,
                                        8.0,
                                        COL_GOLD,
                                        0.85f32,
                                        format!("{}\0", mp_hdr).as_ptr(),
                                    );
                                }
                                let host_label = tl("Host");
                                let client_label = tl("Client");
                                let labels = [
                                    format!("{} ({})", host_label, tl("Host = create network")),
                                    format!("{} ({})", client_label, tl("Client = join network")),
                                ];
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
                                        format!("{}\0", tl("UP/DOWN=select  A=confirm  B=back"))
                                            .as_ptr(),
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
                                                format!("{}\n\0", tl("HOST: Network created!"))
                                                    .as_ptr(),
                                            );
                                            let wait_msg = tl("Waiting for client...");
                                            _3ds_text_add_top(format!("{}\n\0", wait_msg).as_ptr());
                                            let b_cancel = tl("B = cancel");
                                            _3ds_text_add_top(format!("{}\n\0", b_cancel).as_ptr());
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
                                                format!("{}\0", tl("HOST: Network created!"))
                                                    .as_ptr(),
                                            );
                                            let wait_msg = tl("Waiting for client...");
                                            _3ds_top_queue_text(
                                                50.0,
                                                100.0,
                                                COL_LIGHT,
                                                0.70f32,
                                                format!("{}\0", wait_msg).as_ptr(),
                                            );
                                            _3ds_top_queue_text(
                                                50.0,
                                                230.0,
                                                COL_MED,
                                                0.60f32,
                                                format!("{}\0", tl("B=cancel")).as_ptr(),
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    if unsafe { _3ds_is_cli_mode() } {
                                        unsafe {
                                            _3ds_clear_top();
                                            _3ds_text_add_top(
                                                format!(
                                                    "{}\n\0",
                                                    tl_fmt(
                                                        "UDS INIT FAILED",
                                                        &[("e", &e.to_string())]
                                                    )
                                                )
                                                .as_ptr(),
                                            );
                                            _3ds_text_add_top(
                                                format!("{}\n\0", tl("B = back")).as_ptr(),
                                            );
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
                                                format!(
                                                    "{}\0",
                                                    tl_fmt(
                                                        "UDS INIT FAILED",
                                                        &[("e", &e.to_string())]
                                                    )
                                                )
                                                .as_ptr(),
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
                        // Initialize UDS and scan for hosts on first entry
                        if was_dirty {
                            let init_result = uds::uds_init(false);
                            match init_result {
                                Ok(()) => {
                                    let hosts = uds::uds_scan_networks();
                                    if hosts.is_empty() {
                                        // No hosts found
                                        if unsafe { _3ds_is_cli_mode() } {
                                            unsafe {
                                                _3ds_clear_top();
                                                _3ds_text_add_top(
                                                    format!("{}\n\0", tl("No hosts found"))
                                                        .as_ptr(),
                                                );
                                                _3ds_text_add_top(
                                                    format!("{}\n\0", tl("B = back")).as_ptr(),
                                                );
                                            }
                                        } else {
                                            unsafe {
                                                _3ds_top_clear();
                                                _3ds_top_queue_rect(
                                                    0.0, 0.0, 400.0, 240.0, COL_TOP_BG,
                                                );
                                                _3ds_top_queue_text(
                                                    80.0,
                                                    100.0,
                                                    COL_MED,
                                                    0.75f32,
                                                    format!("{}\0", tl("No hosts found")).as_ptr(),
                                                );
                                                _3ds_top_queue_text(
                                                    80.0,
                                                    230.0,
                                                    COL_MED,
                                                    0.60f32,
                                                    format!("{}\0", tl("B=back")).as_ptr(),
                                                );
                                            }
                                        }
                                        Step::Setup(
                                            cards.clone(),
                                            decks.clone(),
                                            SetupPhase::MultiplayerClientScan(p1_idx),
                                            false,
                                        )
                                    } else {
                                        // Hosts found — go to selection
                                        Step::Setup(
                                            cards.clone(),
                                            decks.clone(),
                                            SetupPhase::MultiplayerClientHostSelect(
                                                p1_idx, hosts, 0,
                                            ),
                                            true,
                                        )
                                    }
                                }
                                Err(e) => {
                                    if unsafe { _3ds_is_cli_mode() } {
                                        unsafe {
                                            _3ds_clear_top();
                                            _3ds_text_add_top(
                                                format!(
                                                    "{}\n\0",
                                                    tl_fmt(
                                                        "UDS INIT FAILED",
                                                        &[("e", &e.to_string())]
                                                    )
                                                )
                                                .as_ptr(),
                                            );
                                            _3ds_text_add_top(
                                                format!("{}\n\0", tl("B = back")).as_ptr(),
                                            );
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
                                                format!(
                                                    "{}\0",
                                                    tl_fmt(
                                                        "UDS INIT FAILED",
                                                        &[("e", &e.to_string())]
                                                    )
                                                )
                                                .as_ptr(),
                                            );
                                        }
                                    }
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::MultiplayerPickRole(p1_idx, 0),
                                        true,
                                    )
                                }
                            }
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerClientScan(p1_idx),
                                false,
                            )
                        }
                    }
                    // Multiplayer: Client selecting which host to connect to
                    SetupPhase::MultiplayerClientHostSelect(p1_idx, ref hosts, cursor) => {
                        let n = hosts.len();
                        if n == 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerClientScan(p1_idx),
                                true,
                            )
                        } else if keys & 0x00000002 != 0 {
                            // B = back to scan
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerClientScan(p1_idx),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            // A = connect to selected host
                            let selected = hosts[cursor];
                            match uds::uds_connect_network(selected) {
                                Ok(()) => {
                                    let hello = [0xAAu8];
                                    let _ = uds::uds_send(&hello);
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::MultiplayerSyncDeck(p1_idx, 0, false),
                                        true,
                                    )
                                }
                                Err(_) => Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerClientHostSelect(
                                        p1_idx,
                                        hosts.clone(),
                                        cursor,
                                    ),
                                    true,
                                ),
                            }
                        } else {
                            // UP/DOWN to navigate
                            let mut new_cursor = cursor;
                            if keys & 0x00000040 != 0 && cursor > 0 {
                                new_cursor = cursor - 1;
                            }
                            if keys & 0x00000080 != 0 && cursor + 1 < n {
                                new_cursor = cursor + 1;
                            }
                            // Draw host list
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                }
                                let hdr = tl("SELECT HOST");
                                unsafe {
                                    _3ds_text_add_top(format!("{}\n\0", hdr).as_ptr());
                                }
                                for (i, _) in hosts.iter().enumerate() {
                                    let prefix = if i == new_cursor { "> " } else { "  " };
                                    let label =
                                        format!("{}{}\0", prefix, format!("Host {}", i + 1));
                                    unsafe {
                                        _3ds_text_add_top(format!("{}\n\0", label).as_ptr());
                                    }
                                }
                                unsafe {
                                    _3ds_text_add_top(
                                        format!("{}\n\0", tl("A=connect B=back")).as_ptr(),
                                    );
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
                                        format!("{}\0", tl("SELECT HOST")).as_ptr(),
                                    );
                                }
                                for i in 0..n {
                                    let y = 50.0 + i as f32 * 32.0;
                                    let col = if i == new_cursor { COL_SEL } else { COL_LIGHT };
                                    let prefix = if i == new_cursor { "> " } else { "  " };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            40.0,
                                            y,
                                            col,
                                            0.70f32,
                                            format!("{}{}\0", prefix, format!("Host {}", i + 1))
                                                .as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        40.0,
                                        230.0,
                                        COL_MED,
                                        0.60f32,
                                        format!("{}\0", tl("A=connect B=back")).as_ptr(),
                                    );
                                }
                            }
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerClientHostSelect(
                                    p1_idx,
                                    hosts.clone(),
                                    new_cursor,
                                ),
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
                                        _3ds_text_add_top(
                                            format!("{}\n\0", tl("Receiving deck data..."))
                                                .as_ptr(),
                                        );
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
                                            format!("{}\0", tl("Receiving deck data...")).as_ptr(),
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
                                    false,    // vs_ai (this is multiplayer)
                                    false,    // ai_vs_ai
                                    false,    // cli_mode (start in game mode)
                                    false,    // detail_mode
                                    true,     // choice_image_mode
                                    false,    // choice_subview (false=choices grid)
                                    0,        // text_page
                                    0,        // choice_grid_offset
                                    0,        // hand_offset
                                    0,        // hand_offset_p2
                                    0,        // touch_tap_count
                                    None,     // viewing_card
                                    None,     // zone_viewer
                                    0,        // zone_viewer_offset
                                    false,    // was_touching
                                    true,     // is_multiplayer
                                    is_host,  // is_host
                                    !is_host, // waiting_for_opponent will be recalculated after settle
                                    Overlay::None,
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
                mut choice_subview,
                mut text_page,
                mut choice_grid_offset,
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
                mut overlay,
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

                // Input handling (suppressed when overlay is active)
                if overlay != Overlay::None {
                    // Overlay-specific input
                    match overlay {
                        Overlay::StartMenu(ref mut sel) => {
                            if keys & 0x00000040 != 0 {
                                *sel = sel.saturating_sub(1);
                                redraw = true;
                            }
                            if keys & 0x00000080 != 0 {
                                *sel = sel.saturating_add(1).min(3);
                                redraw = true;
                            }
                            if keys & 0x00000001 != 0 {
                                overlay = match *sel {
                                    0 => Overlay::PerfStats(None),
                                    1 => Overlay::GameLog(gs.rule_log.len().saturating_sub(16)),
                                    2 => Overlay::RevealedCards(true, 0),
                                    3 => {
                                        // Toggle language
                                        set_lang(current_lang().toggle());
                                        i18n::init();
                                        Overlay::StartMenu(*sel)
                                    }
                                    _ => Overlay::None,
                                };
                                redraw = true;
                            }
                            if keys & 0x00000002 != 0 {
                                overlay = Overlay::None;
                                redraw = true;
                            }
                        }
                        Overlay::GameLog(ref mut offset) => {
                            if keys & 0x00000040 != 0 {
                                *offset = offset.saturating_sub(1);
                                redraw = true;
                            }
                            if keys & 0x00000080 != 0 {
                                *offset = offset.saturating_add(1);
                                redraw = true;
                            }
                        }
                        Overlay::PerfStats(ref mut detail) => {
                            if keys & 0x00000001 != 0 {
                                // A toggles detail view for the latest snapshot
                                let snapshots = &gs.performance_snapshots;
                                *detail = match *detail {
                                    None => {
                                        if !snapshots.is_empty() {
                                            Some(snapshots.len() - 1)
                                        } else {
                                            None
                                        }
                                    }
                                    Some(_) => None,
                                };
                                redraw = true;
                            }
                        }
                        Overlay::RevealedCards(ref mut show_self, ref mut rev_scroll) => {
                            if keys & 0x00000400 != 0 {
                                *show_self = !*show_self;
                                *rev_scroll = 0;
                                redraw = true;
                            }
                            if keys & 0x00000040 != 0 {
                                *rev_scroll = rev_scroll.saturating_sub(1);
                                redraw = true;
                            }
                            if keys & 0x00000080 != 0 {
                                *rev_scroll = rev_scroll.saturating_add(1);
                                redraw = true;
                            }
                        }
                        Overlay::None => {}
                    }
                } else if detail_mode && viewing_card.is_some() {
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

                // Image mode: choices are the primary view; L shows ability text overlay
                // L toggles overlay; UP/DOWN/LEFT/RIGHT navigate choices; A confirms
                // Overlay shown: L/B dismiss, UP/DOWN scroll text pages
                if choice_image_mode
                    && gs.has_pending_choice()
                    && zone_viewer.is_none()
                    && overlay == Overlay::None
                {
                    if choice_subview {
                        // === Text overlay: L/B dismiss, UP/DOWN page through text ===
                        if keys & 0x00000200 != 0 || keys & 0x00000002 != 0 {
                            choice_subview = false;
                            redraw = true;
                        }
                        if let Some(entry) = gs.ability_queue.current_entry() {
                            if keys & 0x00000040 != 0 || keys & 0x00000080 != 0 {
                                let ab_text = i18n::translate_ability(
                                    &entry.ability.full_text,
                                    current_lang(),
                                );
                                let ab_lines: Vec<String> =
                                    wrap_ability_text(&ab_text, 384.0, 0.65)
                                        .lines()
                                        .map(|l| l.to_string())
                                        .collect();
                                let lpp = 7usize;
                                let total_pages = ((ab_lines.len() + lpp - 1) / lpp).max(1);
                                if keys & 0x00000040 != 0 {
                                    if text_page > 0 {
                                        text_page -= 1;
                                        redraw = true;
                                    }
                                } else {
                                    if text_page + 1 < total_pages {
                                        text_page += 1;
                                        redraw = true;
                                    }
                                }
                            }
                        }
                    } else {
                        // === Choices grid: L opens text overlay, DPAD navigates items ===
                        if keys & 0x00000200 != 0 {
                            let has_ab_entry = gs.ability_queue.current_entry().is_some();
                            if has_ab_entry {
                                choice_subview = true;
                                text_page = 0;
                                redraw = true;
                            }
                        }
                        let n = display_order.len();
                        if n > 0 {
                            if keys & 0x00000040 != 0 {
                                // UP: move back by one row (5 items)
                                let row = 5usize;
                                if display_pos >= row {
                                    display_pos -= row;
                                } else {
                                    display_pos = 0;
                                }
                                if display_pos < choice_grid_offset {
                                    choice_grid_offset = display_pos / row * row;
                                }
                                cur = display_order[display_pos];
                                redraw = true;
                            }
                            if keys & 0x00000080 != 0 {
                                // DOWN: move forward by one row (5 items)
                                let row = 5usize;
                                display_pos = (display_pos + row).min(n - 1);
                                let visible_end = choice_grid_offset + 10;
                                if display_pos >= visible_end {
                                    choice_grid_offset = (display_pos / row) * row;
                                    choice_grid_offset = choice_grid_offset.saturating_sub(row);
                                }
                                cur = display_order[display_pos];
                                redraw = true;
                            }
                            if keys & 0x00000020 != 0 {
                                // LEFT: move back by 1, don't cross row boundary
                                let row_start = choice_grid_offset;
                                if display_pos > row_start {
                                    display_pos -= 1;
                                }
                                cur = display_order[display_pos];
                                redraw = true;
                            }
                            if keys & 0x00000010 != 0 {
                                // RIGHT: move forward by 1, don't cross row boundary
                                let row_end = (choice_grid_offset + 5).min(n);
                                if display_pos + 1 < row_end {
                                    display_pos += 1;
                                }
                                cur = display_order[display_pos];
                                redraw = true;
                            }
                        }
                    }
                }

                // B: close menus / overlays / card detail
                if keys & 0x00000002 != 0 {
                    if viewing_card.is_some() {
                        viewing_card = None;
                        detail_mode = false;
                        redraw = true;
                    } else if zone_viewer.is_some() {
                        zone_viewer = None;
                        redraw = true;
                    } else if overlay != Overlay::None {
                        overlay = Overlay::None;
                        redraw = true;
                    }
                }

                // SELECT cycles board view: player / opponent / both
                if overlay == Overlay::None && keys & 0x00000004 != 0 {
                    unsafe {
                        _3ds_board_cycle_view();
                    }
                    redraw = true;
                }

                // DPAD LEFT/RIGHT: scroll hand view (0x10 = RIGHT, 0x20 = LEFT)
                if overlay == Overlay::None && !detail_mode {
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
                if overlay == Overlay::None && keys & 0x00000400 != 0 {
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
                    // X: show card detail for selected card
                    if keys & 0x00000400 != 0 && viewing_card.is_none() {
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
                if overlay == Overlay::None && keys & 0x00000100 != 0 {
                    choice_image_mode = !choice_image_mode;
                    redraw = true;
                }

                // Y toggles CLI/game mode
                if overlay == Overlay::None && keys & 0x00000800 != 0 {
                    cli_mode = !cli_mode;
                    unsafe {
                        _3ds_set_cli_mode(cli_mode);
                    }
                    redraw = true;
                }

                // START opens the in-game menu (perf stats / game log / revealed cards)
                if keys & 0x00000008 != 0 {
                    overlay = if overlay == Overlay::None {
                        Overlay::StartMenu(0)
                    } else {
                        Overlay::None
                    };
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
                                let _ =
                                    turn::TurnEngine::execute_main_phase_action_with_ability_index(
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
                                        sync.ability_index.map(|x| x as usize),
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
                if overlay == Overlay::None
                    && zone_viewer.is_none()
                    && keys & 0x00000001 != 0
                    && cur < acts_cache.len()
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
                        let result = turn::TurnEngine::execute_main_phase_action_with_ability_index(
                            &mut gs,
                            &action.action_type,
                            p.as_ref().and_then(|x| x.card_id),
                            p.as_ref().and_then(|x| x.card_indices.clone()),
                            p.as_ref()
                                .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
                            p.as_ref().and_then(|x| x.use_baton_touch),
                            p.as_ref().and_then(|x| x.ability_index),
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
                                ability_index: p
                                    .as_ref()
                                    .and_then(|x| x.ability_index)
                                    .map(|x| x as u16),
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
                if is_ai_turn && !dirty && (_frame % 10 == 0) {
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
                    game_setup::settle_single_player_state(&mut gs);
                    if is_multiplayer {
                        let my_id = if is_host { 0 } else { 1 };
                        waiting_for_opponent = !mp_can_act(&gs, my_id);
                    }
                    cur = 0;
                    dirty = true;
                }
                if gs.has_pending_choice()
                    || game_setup::is_automatic_phase(&gs)
                    || gs.game_result != GameResult::Ongoing
                {
                    cur = 0;
                    dirty = true;
                    redraw = true;
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
                        // Compute zone coordinates for each player's section separately
                        // because zone positions depend on section_rect (which differs per player in split view)
                        let (p1_stage_y, p1_stage_h, p1_st_slot_w) = unsafe {
                            _3ds_board_set_section_rect(p1y0 as f32, p1h as f32, false);
                            (
                                _3ds_board_get_zone_y(1),
                                _3ds_board_get_zone_h(1),
                                _3ds_board_get_slot_w(1),
                            )
                        };
                        let (p2_stage_y, p2_stage_h, p2_st_slot_w) = if p2h > 0 {
                            unsafe {
                                _3ds_board_set_section_rect(p2y0 as f32, p2h as f32, true);
                                (
                                    _3ds_board_get_zone_y(1),
                                    _3ds_board_get_zone_h(1),
                                    _3ds_board_get_slot_w(1),
                                )
                            }
                        } else {
                            (0, 0, 0.0f32)
                        };
                        // Use the correct coordinates for the tapped player's section
                        let (stage_y, stage_h, st_slot_w) = if y0 == p1y0 {
                            (p1_stage_y, p1_stage_h, p1_st_slot_w)
                        } else {
                            (p2_stage_y, p2_stage_h, p2_st_slot_w)
                        };
                        let mut stage_tap: Option<String> = None;
                        let mut tapped_card: Option<i16> = None;
                        let mut tapped_hand_idx: Option<usize> = None;
                        let mut tap_active_side: bool = false;
                        if h > 0 {
                            let pb = if y0 == p1y0 { &gs.player1 } else { &gs.player2 };
                            let ap_id = &gs.active_player().id;
                            let is_ap_p1 = ap_id == &gs.player1.id;
                            tap_active_side = if is_ap_p1 { y0 == p1y0 } else { y0 != p1y0 };
                            if detail_mode && viewing_card.is_some() && tap_active_side {
                                let raw = ((tx as f32 - 2.0) / (st_slot_w + 2.0)) as usize;
                                if (ty as i32) >= stage_y && (ty as i32) < (stage_y + stage_h) {
                                    let idx = if y0 != p1y0 { 2 - raw } else { raw };
                                    stage_tap = match idx {
                                        0 => Some("left".into()),
                                        1 => Some("center".into()),
                                        2 => Some("right".into()),
                                        _ => None,
                                    };
                                }
                            }
                            // Also detect stage tap for choice position (any side, any mode)
                            if stage_tap.is_none()
                                && choice_image_mode
                                && gs.has_pending_choice()
                                && (ty as i32) >= stage_y
                                && (ty as i32) < (stage_y + stage_h)
                            {
                                let raw = ((tx as f32 - 2.0) / (st_slot_w + 2.0)) as usize;
                                let idx = if y0 != p1y0 { 2 - raw } else { raw };
                                stage_tap = match idx {
                                    0 => Some("left".into()),
                                    1 => Some("center".into()),
                                    2 => Some("right".into()),
                                    _ => None,
                                };
                            }
                            unsafe {
                                _3ds_board_set_section_rect(y0 as f32, h as f32, y0 != p1y0);
                            }
                            let hand_y = unsafe { _3ds_board_get_zone_y(3) };
                            let hand_h = unsafe { _3ds_board_get_zone_h(3) };
                            let live_y = unsafe { _3ds_board_get_zone_y(0) };
                            let live_h = unsafe { _3ds_board_get_zone_h(0) };
                            let vis = visible_hand_slots();
                            let hand_slot_w = unsafe { _3ds_board_get_slot_w(3) };
                            let live_slot_w = unsafe { _3ds_board_get_slot_w(0) };
                            tapped_card = if tap_active_side
                                && (ty as i32) >= hand_y
                                && (ty as i32) < (hand_y + hand_h)
                            {
                                let idx = ((tx as f32 - 4.0) / (hand_slot_w + 2.0)) as usize;
                                let hoff = if y0 != p1y0 {
                                    hand_offset_p2
                                } else {
                                    hand_offset
                                };
                                let hand_idx = hoff + idx;
                                if idx < vis && hand_idx < pb.hand.cards.len() {
                                    tapped_hand_idx = Some(hand_idx);
                                    Some(pb.hand.cards[hand_idx])
                                } else {
                                    None
                                }
                            } else if (ty as i32) >= stage_y && (ty as i32) < (stage_y + stage_h) {
                                let raw_idx = ((tx as f32 - 2.0) / (st_slot_w + 2.0)) as usize;
                                let idx = if y0 != p1y0 { 2 - raw_idx } else { raw_idx };
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
                            // Utility zone tap: left half = waitroom, right half = live success
                            if tapped_card.is_none() {
                                let tx_f = tx as f32;
                                let ux = 2.0 + 3.0 * (st_slot_w + 2.0) + 5.0;
                                let uw = 320.0 - ux - 2.0;
                                let zoned = if (ty as i32) >= stage_y
                                    && (ty as i32) < (stage_y + stage_h)
                                    && tx_f >= ux
                                    && tx_f < ux + uw
                                {
                                    if tx_f < ux + uw * 0.5 {
                                        Some((
                                            tl("Waitroom").into(),
                                            pb.waitroom.cards.iter().copied().collect::<Vec<i16>>(),
                                        ))
                                    } else {
                                        Some((
                                            tl("Live Success").into(),
                                            pb.success_live_card_zone
                                                .cards
                                                .iter()
                                                .copied()
                                                .collect::<Vec<i16>>(),
                                        ))
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
                        } // end h > 0 guard

                        // ===== TAP-TO-DEPLOY (pb dropped, can &mut gs) =====

                        // Phase-specific: mulligan hand tap toggles selection
                        if let Some(cid) = tapped_card {
                            if tap_active_side
                                && matches!(
                                    gs.current_phase,
                                    Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
                                )
                            {
                                if let Some(hidx) = tapped_hand_idx {
                                    for (ai, act) in acts_cache.iter().enumerate() {
                                        if act.action_type != game_setup::ActionType::SelectMulligan
                                        {
                                            continue;
                                        }
                                        let p = match &act.parameters {
                                            Some(x) => x,
                                            None => continue,
                                        };
                                        if p.card_indices.as_ref().and_then(|v| v.first().copied())
                                            != Some(hidx)
                                        {
                                            continue;
                                        }
                                        let action = acts_cache[ai].clone();
                                        let pp = action.parameters.clone();
                                        let _ = turn::TurnEngine::execute_main_phase_action(
                                            &mut gs,
                                            &action.action_type,
                                            pp.as_ref().and_then(|x| x.card_id),
                                            pp.as_ref().and_then(|x| x.card_indices.clone()),
                                            None,
                                            None,
                                        );
                                        gs.reset_loop_detection();
                                        if is_multiplayer {
                                            let my_id = if is_host { 0 } else { 1 };
                                            let sync = uds::ActionSync {
                                                action_tag: 4,
                                                card_id: pp.as_ref().and_then(|x| x.card_id),
                                                card_indices: pp
                                                    .as_ref()
                                                    .and_then(|x| x.card_indices.clone())
                                                    .unwrap_or_default(),
                                                stage_area: 0,
                                                use_baton_touch: false,
                                                ability_index: None,
                                            };
                                            let _ = uds::uds_send(&sync.to_bytes());
                                            waiting_for_opponent = !mp_can_act(&gs, my_id);
                                        }
                                        cur = 0;
                                        dirty = true;
                                        redraw = true;
                                        break;
                                    }
                                }
                            // Live card phase: hand tap toggles selection
                            } else if tap_active_side
                                && matches!(
                                    gs.current_phase,
                                    Phase::LiveCardSetFirstAttacker
                                        | Phase::LiveCardSetSecondAttacker
                                )
                            {
                                if let Some(hidx) = tapped_hand_idx {
                                    for (ai, act) in acts_cache.iter().enumerate() {
                                        if act.action_type != game_setup::ActionType::SelectLiveCard
                                        {
                                            continue;
                                        }
                                        let p = match &act.parameters {
                                            Some(x) => x,
                                            None => continue,
                                        };
                                        if p.card_indices.as_ref().and_then(|v| v.first().copied())
                                            != Some(hidx)
                                        {
                                            continue;
                                        }
                                        let action = acts_cache[ai].clone();
                                        let pp = action.parameters.clone();
                                        let _ = turn::TurnEngine::execute_main_phase_action(
                                            &mut gs,
                                            &action.action_type,
                                            pp.as_ref().and_then(|x| x.card_id),
                                            pp.as_ref().and_then(|x| x.card_indices.clone()),
                                            None,
                                            None,
                                        );
                                        gs.reset_loop_detection();
                                        if is_multiplayer {
                                            let my_id = if is_host { 0 } else { 1 };
                                            let sync = uds::ActionSync {
                                                action_tag: 18,
                                                card_id: pp.as_ref().and_then(|x| x.card_id),
                                                card_indices: pp
                                                    .as_ref()
                                                    .and_then(|x| x.card_indices.clone())
                                                    .unwrap_or_default(),
                                                stage_area: 0,
                                                use_baton_touch: false,
                                                ability_index: None,
                                            };
                                            let _ = uds::uds_send(&sync.to_bytes());
                                            waiting_for_opponent = !mp_can_act(&gs, my_id);
                                        }
                                        cur = 0;
                                        dirty = true;
                                        redraw = true;
                                        break;
                                    }
                                }
                            // Choice image mode: board tap executes the choice directly
                            } else if choice_image_mode && gs.has_pending_choice() {
                                let mut act_idx: Option<usize> =
                                    acts_cache.iter().position(|act| {
                                        act.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
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
                                            if let Some(opt_idx) =
                                                options.iter().position(|o| o.card_id == Some(cid))
                                            {
                                                act_idx = acts_cache.iter().position(|act| {
                                                    act.parameters.as_ref().and_then(|p| p.card_id)
                                                        == Some(opt_idx as i16)
                                                        && act.action_type
                                                            == game_setup::ActionType::ChoiceOption
                                                });
                                            }
                                        }
                                    }
                                }
                                if let Some(idx) = act_idx {
                                    let action = acts_cache[idx].clone();
                                    let p = action.parameters.clone();
                                    let result = turn::TurnEngine::execute_main_phase_action_with_ability_index(
                                        &mut gs,
                                        &action.action_type,
                                        p.as_ref().and_then(|x| x.card_id),
                                        p.as_ref().and_then(|x| x.card_indices.clone()),
                                        p.as_ref().and_then(|x| {
                                            x.stage_area.as_ref().and_then(|s| s.parse().ok())
                                        }),
                                        p.as_ref().and_then(|x| x.use_baton_touch),
                                        p.as_ref().and_then(|x| x.ability_index),
                                    );
                                    if let Err(ref e) = result {
                                        unsafe {
                                            _3ds_debug_print(format!("[ERR] {}\n\0", e).as_ptr());
                                        }
                                    }
                                    gs.reset_loop_detection();
                                    cur = 0;
                                    dirty = true;
                                } else {
                                    // Unhandled choice tap: fall through to detail toggle
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
                            // Default: toggle card detail view (skip if user tapped a stage zone)
                            } else if stage_tap.is_none() {
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
                                redraw = true;
                            }
                        } else if stage_tap.is_none() {
                            viewing_card = None;
                        }

                        // Stage zone tap actions (PlayMemberToStage, UseAbility, ChoicePosition)
                        let mut stage_handled = false;
                        if let Some(sa) = &stage_tap {
                            let slot_idx = match sa.as_str() {
                                "left" => 0usize,
                                "center" => 1,
                                "right" => 2,
                                _ => 3,
                            };
                            // Detail mode: PlayMemberToStage (empty slot) or UseAbility (filled slot)
                            if detail_mode && viewing_card.is_some() && tap_active_side {
                                let player = if y0 == p1y0 { &gs.player1 } else { &gs.player2 };
                                let card_at_slot = if slot_idx < 3 {
                                    let cid = player.stage.stage[slot_idx];
                                    if cid != -1 {
                                        Some(cid)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                // PlayMemberToStage (empty slot = normal play, filled slot = baton touch)
                                for (ai, act) in acts_cache.iter().enumerate() {
                                    if act.action_type != game_setup::ActionType::PlayMemberToStage
                                    {
                                        continue;
                                    }
                                    let p = match &act.parameters {
                                        Some(x) => x,
                                        None => continue,
                                    };
                                    if p.disabled.unwrap_or(false) {
                                        continue;
                                    }
                                    if p.stage_area.as_ref().map(|s| s.as_str())
                                        != Some(sa.as_str())
                                    {
                                        continue;
                                    }
                                    if p.card_id != viewing_card {
                                        continue;
                                    }
                                    cur = ai;
                                    let act2 = acts_cache[cur].clone();
                                    let pp = act2.parameters.clone();
                                    let _ = turn::TurnEngine::execute_main_phase_action_with_ability_index(
                                        &mut gs,
                                        &act2.action_type,
                                        pp.as_ref().and_then(|x| x.card_id),
                                        pp.as_ref().and_then(|x| x.card_indices.clone()),
                                        pp.as_ref().and_then(|x| {
                                            x.stage_area.as_ref().and_then(|s| s.parse().ok())
                                        }),
                                        pp.as_ref().and_then(|x| x.use_baton_touch),
                                        pp.as_ref().and_then(|x| x.ability_index),
                                    );
                                    if is_multiplayer {
                                        let my_id = if is_host { 0 } else { 1 };
                                        let sc = match sa.as_str() {
                                            "left" => 1u8,
                                            "center" => 2,
                                            "right" => 3,
                                            _ => 0,
                                        };
                                        let sync = uds::ActionSync {
                                            action_tag: 6,
                                            card_id: pp.as_ref().and_then(|x| x.card_id),
                                            card_indices: pp
                                                .as_ref()
                                                .and_then(|x| x.card_indices.clone())
                                                .unwrap_or_default(),
                                            stage_area: sc,
                                            use_baton_touch: pp
                                                .as_ref()
                                                .and_then(|x| x.use_baton_touch)
                                                .unwrap_or(false),
                                            ability_index: None,
                                        };
                                        let _ = uds::uds_send(&sync.to_bytes());
                                        waiting_for_opponent = !mp_can_act(&gs, my_id);
                                    }
                                    detail_mode = false;
                                    viewing_card = None;
                                    cur = 0;
                                    redraw = true;
                                    stage_handled = true;
                                    break;
                                }

                                // Filled stage slot: activate the selected Kidou ability.
                                // This was documented as supported here, but the old code
                                // only searched PlayMemberToStage actions, so tapping a
                                // staged Kidou card could leave the stale Pass-only view.
                                if !stage_handled {
                                    for (ai, act) in acts_cache.iter().enumerate() {
                                        if act.action_type != game_setup::ActionType::UseAbility {
                                            continue;
                                        }
                                        let p = match &act.parameters {
                                            Some(x) => x,
                                            None => continue,
                                        };
                                        if p.card_id != viewing_card
                                            || p.stage_area.as_deref() != Some(sa.as_str())
                                        {
                                            continue;
                                        }
                                        if p.disabled.unwrap_or(false) {
                                            continue;
                                        }
                                        let action = act.clone();
                                        let params = action.parameters.clone();
                                        let _ = turn::TurnEngine::execute_main_phase_action_with_ability_index(
                                            &mut gs,
                                            &action.action_type,
                                            params.as_ref().and_then(|x| x.card_id),
                                            params.as_ref().and_then(|x| x.card_indices.clone()),
                                            params.as_ref().and_then(|x| {
                                                x.stage_area.as_ref().and_then(|s| s.parse().ok())
                                            }),
                                            params.as_ref().and_then(|x| x.use_baton_touch),
                                            params.as_ref().and_then(|x| x.ability_index),
                                        );
                                        if is_multiplayer {
                                            let sync = uds::ActionSync {
                                                action_tag: 15,
                                                card_id: params.as_ref().and_then(|x| x.card_id),
                                                card_indices: params
                                                    .as_ref()
                                                    .and_then(|x| x.card_indices.clone())
                                                    .unwrap_or_default(),
                                                stage_area: match sa.as_str() {
                                                    "left" => 1,
                                                    "center" => 2,
                                                    "right" => 3,
                                                    _ => 0,
                                                },
                                                use_baton_touch: false,
                                                ability_index: params
                                                    .as_ref()
                                                    .and_then(|x| x.ability_index)
                                                    .map(|x| x as u16),
                                            };
                                            let _ = uds::uds_send(&sync.to_bytes());
                                        }
                                        gs.reset_loop_detection();
                                        detail_mode = false;
                                        viewing_card = None;
                                        cur = 0;
                                        redraw = true;
                                        stage_handled = true;
                                        break;
                                    }
                                }
                            }
                            // ChoicePosition: select stage position during choice prompt
                            if !stage_handled && choice_image_mode && gs.has_pending_choice() {
                                for (ai, act) in acts_cache.iter().enumerate() {
                                    if act.action_type != game_setup::ActionType::ChoicePosition {
                                        continue;
                                    }
                                    let p = match &act.parameters {
                                        Some(x) => x,
                                        None => continue,
                                    };
                                    if p.disabled.unwrap_or(false) {
                                        continue;
                                    }
                                    if p.stage_area.as_ref().map(|s| s.as_str())
                                        != Some(sa.as_str())
                                    {
                                        continue;
                                    }
                                    cur = ai;
                                    let act2 = acts_cache[cur].clone();
                                    let pp = act2.parameters.clone();
                                    let result = turn::TurnEngine::execute_main_phase_action(
                                        &mut gs,
                                        &act2.action_type,
                                        pp.as_ref().and_then(|x| x.card_id),
                                        pp.as_ref().and_then(|x| x.card_indices.clone()),
                                        pp.as_ref().and_then(|x| {
                                            x.stage_area.as_ref().and_then(|s| s.parse().ok())
                                        }),
                                        pp.as_ref().and_then(|x| x.use_baton_touch),
                                    );
                                    if let Err(ref e) = result {
                                        unsafe {
                                            _3ds_debug_print(format!("[ERR] {}\n\0", e).as_ptr());
                                        }
                                    }
                                    if is_multiplayer {
                                        let my_id = if is_host { 0 } else { 1 };
                                        let sc = match sa.as_str() {
                                            "left" => 1u8,
                                            "center" => 2,
                                            "right" => 3,
                                            _ => 0,
                                        };
                                        let sync = uds::ActionSync {
                                            action_tag: 14,
                                            card_id: pp.as_ref().and_then(|x| x.card_id),
                                            card_indices: pp
                                                .as_ref()
                                                .and_then(|x| x.card_indices.clone())
                                                .unwrap_or_default(),
                                            stage_area: sc,
                                            use_baton_touch: false,
                                            ability_index: None,
                                        };
                                        let _ = uds::uds_send(&sync.to_bytes());
                                        waiting_for_opponent = !mp_can_act(&gs, my_id);
                                    }
                                    gs.reset_loop_detection();
                                    viewing_card = None;
                                    cur = 0;
                                    redraw = true;
                                    detail_mode = false;
                                    break;
                                }
                            }
                        }
                    }
                }
                was_touching = touching;

                if dirty || redraw {
                    acts_cache = game_setup::generate_possible_actions(&gs);
                    choice_grid_offset = 0;

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
                            for (i, cid) in ec.iter().enumerate().take(30) {
                                // Energy cards: tapped if position >= active_count (front = active)
                                let tapped = i >= e_active;
                                set_slot($energy_fn, i as i32, *cid, false, tapped);
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
                        // Hide AI opponent's hand — shouldn't be visible to player
                        unsafe {
                            _3ds_board_set_opp_hand_count(0);
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
                            let wins_text = tl("Score");
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
                                    "{}: {} vs {}\0",
                                    wins_text,
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
                                format!("{}\0", tl("Press START to exit")).as_ptr(),
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
                                            let display_name =
                                                i18n::card_display_name(&card.name, current_lang());
                                            unsafe {
                                                _3ds_text_add_top(
                                                    format!(
                                                        "[{}] {}\n\0",
                                                        card.card_no, display_name
                                                    )
                                                    .as_ptr(),
                                                );
                                            }
                                            for ab in card.resolved_abilities() {
                                                let ab_text = i18n::translate_ability(
                                                    &ab.full_text,
                                                    current_lang(),
                                                );
                                                let w = wrap_ability_text(&ab_text, 390.0, 0.85);
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
                                        "{} {} | {:?} | {}{} | taps:{}\n\0",
                                        tl("Turn").trim_end_matches(':'),
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
                                    let display_name =
                                        i18n::card_display_name(&card.name, current_lang());
                                    unsafe {
                                        _3ds_text_add_top(
                                            format!(
                                                "[{}] {}\n\0",
                                                card.card_no,
                                                wrap_text(&display_name, 390.0, 0.85)
                                            )
                                            .as_ptr(),
                                        );
                                    }
                                    for ab in card.resolved_abilities() {
                                        let ab_text =
                                            i18n::translate_ability(&ab.full_text, current_lang());
                                        let w = wrap_ability_text(&ab_text, 390.0, 0.85);
                                        unsafe {
                                            _3ds_text_add_top(format!("{}\n\0", w).as_ptr());
                                        }
                                    }
                                    unsafe {
                                        _3ds_text_add_top("(tap slot to dismiss)\n\0".as_ptr());
                                    }
                                }
                            } else if let Some(entry) = gs.ability_queue.current_entry() {
                                let ab_text = wrap_ability_text(
                                    &i18n::translate_ability(
                                        &entry.ability.full_text,
                                        current_lang(),
                                    ),
                                    390.0,
                                    0.85,
                                );
                                for line in ab_text.lines() {
                                    unsafe {
                                        _3ds_text_add_top(format!("{}\n\0", line).as_ptr());
                                    }
                                }
                            }
                            let is_ai_turn =
                                *ai_vs_ai || (*vs_ai && gs.active_player().id != gs.player1.id);
                            let is_opponent_turn_mp =
                                is_multiplayer && !mp_can_act(&gs, if is_host { 0 } else { 1 });
                            if is_ai_turn {
                                let msg = tl("AI is thinking...");
                                unsafe {
                                    _3ds_text_add_top(format!("{}\n\0", msg).as_ptr());
                                }
                            } else if is_opponent_turn_mp {
                                let msg = tl("Waiting for opponent...");
                                unsafe {
                                    _3ds_text_add_top(format!("{}\n\0", msg).as_ptr());
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
                                        game_setup::ActionType::Pass => tl("Pass"),
                                        game_setup::ActionType::PlayMemberToStage => {
                                            let name = i18n::card_display_name(
                                                &act.parameters
                                                    .as_ref()
                                                    .and_then(|p| p.card_name.clone())
                                                    .unwrap_or_default(),
                                                current_lang(),
                                            );
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
                                            let area_label = tl_area(&area);
                                            let card_indices = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.card_indices.clone())
                                                .unwrap_or_default();
                                            let is_db = card_indices.len() >= 2;
                                            if is_db {
                                                let src_labels: Vec<&str> = card_indices
                                                    .iter()
                                                    .map(|&idx| match idx {
                                                        0 => tl_area("left"),
                                                        1 => tl_area("center"),
                                                        2 => tl_area("right"),
                                                        _ => "?",
                                                    })
                                                    .collect();
                                                format!(
                                                    "[{}] {} c:{} {}→{}",
                                                    cn,
                                                    name,
                                                    cost,
                                                    src_labels.join("+"),
                                                    area_label
                                                )
                                            } else {
                                                format!(
                                                    "[{}] {} c:{}   {}",
                                                    cn, name, cost, area_label
                                                )
                                            }
                                        }
                                        game_setup::ActionType::UseAbility => {
                                            let name = i18n::card_display_name(
                                                &act.parameters
                                                    .as_ref()
                                                    .and_then(|p| p.card_name.clone())
                                                    .unwrap_or_default(),
                                                current_lang(),
                                            );
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
                                            let area_label = tl_area(&area);
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
                                                    area_label,
                                                    cost,
                                                    abil_short
                                                )
                                            } else {
                                                format!(
                                                    "[{}] {} {} {}",
                                                    cn_or_empty(act),
                                                    name,
                                                    area_label,
                                                    abil_short
                                                )
                                            }
                                        }
                                        _ => act
                                            .display_desc(current_lang() == Lang::Japanese)
                                            .lines()
                                            .next()
                                            .unwrap_or("")
                                            .to_string(),
                                    };
                                    let desc_full = wrap_text(&line, 390.0, 0.85);
                                    for (li, l) in desc_full.lines().enumerate() {
                                        if li == 0 {
                                            unsafe {
                                                _3ds_text_add_top(
                                                    format!("{}{}\n\0", prefix, l).as_ptr(),
                                                );
                                            }
                                        } else {
                                            unsafe {
                                                _3ds_text_add_top(format!("{}\n\0", l).as_ptr());
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
                                        let ab_text =
                                            i18n::translate_ability(&ab.full_text, current_lang());
                                        wrap_ability_text(&ab_text, 390.0, 0.85)
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
                        // Top screen: stats bar (0-40px) + content panel (42-240px).
                        // Clear the top screen so old menu content doesn't overlap
                        unsafe {
                            _3ds_top_clear();
                        }
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

                        // Content panel — rendering stack (bottom to top):
                        //   1. zone_viewer       — zone card grid (own/opponent stage)
                        //   2. detail_mode        — full-screen card detail overlay
                        //   3. ability_queue      — compact ability banner (CLI/text only)
                        //   4. choice_image_mode  — ability banner + card choice grid
                        //   5. action list        — text action list (bottom text area)
                        let mut content_y: f32 = 42.0;

                        if let Some((ref zlabel, ref zcards)) = zone_viewer {
                            if viewing_card.is_none() {
                                let gap = 4.0f32;
                                let cols = 5usize;
                                let rows = 2usize;
                                let pp = cols * rows;
                                let max_ch = ((240.0 - 28.0 - gap) / rows as f32) - 14.0;
                                let cw = (max_ch * 0.711)
                                    .min((400.0 - 8.0 - (cols as f32 - 1.0) * gap) / cols as f32);
                                let ch = cw / 0.711;
                                let page = zone_viewer_offset / pp * pp;
                                let n = zcards.len();
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        4.0,
                                        4.0,
                                        COL_GOLD,
                                        0.70f32,
                                        format!("{}  (B=close, X=detail)\0", zlabel).as_ptr(),
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
                                                c_str.as_ptr() as *const u8,
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
                                    let stats = compute_card_stats(card, vcid, &gs);
                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                        _3ds_top_queue_rect(0.0, 0.0, 400.0, 232.0, COL_CARD);
                                        let display_name =
                                            i18n::card_display_name(&card.name, current_lang());
                                        _3ds_top_queue_text(
                                            4.0,
                                            4.0,
                                            COL_BLUE,
                                            0.80f32,
                                            format!("[{}] {}\0", card.card_no, display_name)
                                                .as_ptr(),
                                        );
                                        render_text_with_icons(
                                            4.0,
                                            24.0,
                                            &card_stat_line(
                                                stats.total_blade,
                                                &stats.heart_str,
                                                stats.score,
                                                stats.cost,
                                                stats.is_tapped,
                                                card.card_type.as_card_str(),
                                                &stats.need_heart_str,
                                            ),
                                            COL_LIGHT,
                                            0.65f32,
                                        );
                                        let mut ty = 44.0;
                                        for ab in card.resolved_abilities() {
                                            let ab_text = i18n::translate_ability(
                                                &ab.full_text,
                                                current_lang(),
                                            );
                                            let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                                            for line in w.lines() {
                                                if ty < 230.0 {
                                                    render_text_with_icons(
                                                        4.0, ty, line, COL_LIGHT, 0.65,
                                                    );
                                                    ty += 18.0;
                                                }
                                            }
                                            ty += 3.0;
                                        }
                                    }
                                }
                            }
                        } else if detail_mode {
                            let detail_cid = viewing_card.or_else(|| {
                                acts_cache
                                    .get(cur)
                                    .and_then(|a| a.parameters.as_ref().and_then(|p| p.card_id))
                            });
                            let mut ability_end = 0.0;
                            if let Some(cid) = detail_cid {
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    // Pre-count ability text lines so we can size the panel
                                    let mut line_count = 0usize;
                                    for ab in card.resolved_abilities() {
                                        let ab_text =
                                            i18n::translate_ability(&ab.full_text, current_lang());
                                        let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                                        line_count += w.lines().count();
                                    }
                                    // If no abilities, use minimal height; otherwise expand panel
                                    let text_h = 86.0
                                        + line_count as f32 * 18.0
                                        + (card.resolved_abilities().count().saturating_sub(1)
                                            as f32)
                                            * 3.0;
                                    let min_h = 86.0 + 18.0; // at least one line
                                    let panel_end = (text_h.max(min_h) + 8.0).min(232.0);
                                    let rect_h = panel_end - 42.0;

                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 42.0, 400.0, rect_h, COL_CARD);
                                        let display_name =
                                            i18n::card_display_name(&card.name, current_lang());
                                        _3ds_top_queue_text(
                                            4.0,
                                            44.0,
                                            COL_BLUE,
                                            0.80f32,
                                            format!(
                                                "[{}] {}\0",
                                                card.card_no,
                                                wrap_text(&display_name, 392.0, 0.80)
                                            )
                                            .as_ptr(),
                                        );
                                        let stats = compute_card_stats(card, cid, &gs);
                                        render_text_with_icons(
                                            4.0,
                                            66.0,
                                            &card_stat_line(
                                                stats.total_blade,
                                                &stats.heart_str,
                                                stats.score,
                                                stats.cost,
                                                stats.is_tapped,
                                                card.card_type.as_card_str(),
                                                &stats.need_heart_str,
                                            ),
                                            COL_LIGHT,
                                            0.65f32,
                                        );
                                        let mut ty = 86.0;
                                        for ab in card.resolved_abilities() {
                                            let ab_text = i18n::translate_ability(
                                                &ab.full_text,
                                                current_lang(),
                                            );
                                            let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                                            for line in w.lines() {
                                                render_text_with_icons(
                                                    4.0, ty, line, COL_LIGHT, 0.65,
                                                );
                                                ty += 18.0;
                                            }
                                            ty += 3.0;
                                        }
                                        ability_end = ty;
                                    }
                                }
                            }
                            content_y = if ability_end > 0.0 {
                                ability_end + 6.0
                            } else {
                                158.0
                            };
                        } else {
                            if let Some(vcid) = viewing_card {
                                // Compact card info overlay with stats
                                if let Some(card) = gs.card_database.get_card(vcid) {
                                    let stats = compute_card_stats(card, vcid, &gs);
                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 42.0, 400.0, 76.0, COL_CARD);
                                        let btm_name =
                                            i18n::card_display_name(&card.name, current_lang());
                                        _3ds_top_queue_text(
                                            4.0,
                                            44.0,
                                            COL_BLUE,
                                            0.75f32,
                                            format!(
                                                "[{}] {}\0",
                                                card.card_no,
                                                wrap_text(&btm_name, 392.0, 0.75)
                                            )
                                            .as_ptr(),
                                        );
                                        render_text_with_icons(
                                            4.0,
                                            64.0,
                                            &card_stat_line(
                                                stats.total_blade,
                                                &stats.heart_str,
                                                stats.score,
                                                stats.cost,
                                                stats.is_tapped,
                                                card.card_type.as_card_str(),
                                                &stats.need_heart_str,
                                            ),
                                            COL_LIGHT,
                                            0.65f32,
                                        );
                                        if let Some(ab) = card.resolved_abilities().next() {
                                            let ab_text = i18n::translate_ability(
                                                &ab.full_text,
                                                current_lang(),
                                            );
                                            let first_line =
                                                wrap_ability_text(&ab_text, 392.0, 0.60)
                                                    .lines()
                                                    .next()
                                                    .unwrap_or("")
                                                    .to_string();
                                            render_text_with_icons(
                                                4.0,
                                                82.0,
                                                &first_line,
                                                COL_LIGHT,
                                                0.60,
                                            );
                                        }
                                    }
                                }
                                content_y = 126.0;
                            } else if let Some(entry) = gs.ability_queue.current_entry() {
                                // In image mode with choices, the text subview handles this.
                                // The banner is only for CLI/text mode.
                                if !(choice_image_mode && gs.has_pending_choice()) {
                                    let ab_text = i18n::translate_ability(
                                        &entry.ability.full_text,
                                        current_lang(),
                                    );
                                    let ab_lines: Vec<String> =
                                        wrap_ability_text(&ab_text, 392.0, 0.65)
                                            .lines()
                                            .take(4)
                                            .map(|l| l.to_string())
                                            .collect();
                                    let n_lines = ab_lines.len();
                                    let h = 22.0 + n_lines as f32 * 14.0;
                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 42.0, 400.0, h, COL_ABILITY);
                                        render_text_with_icons(
                                            4.0,
                                            44.0,
                                            &ab_lines[0],
                                            COL_LIGHT,
                                            0.65,
                                        );
                                        for (li, line) in ab_lines.iter().enumerate().skip(1) {
                                            render_text_with_icons(
                                                8.0,
                                                44.0 + li as f32 * 14.0,
                                                line,
                                                COL_LIGHT,
                                                0.65,
                                            );
                                        }
                                    }
                                    content_y = 42.0 + h + 6.0;
                                }
                            }
                        }

                        // ---- Choice image mode: ability banner + card grid ----
                        // When detail_mode is active, the card detail overlay (above)
                        // replaces the grid so card images don't overlap the detail text.
                        {
                            let is_ai_turn =
                                *ai_vs_ai || (*vs_ai && gs.active_player().id != gs.player1.id);
                            let is_opponent_turn_mp =
                                is_multiplayer && !mp_can_act(&gs, if is_host { 0 } else { 1 });
                            if zone_viewer.is_none() {
                                if choice_image_mode
                                    && gs.has_pending_choice()
                                    && !(detail_mode && viewing_card.is_some())
                                    && !is_ai_turn
                                    && !is_opponent_turn_mp
                                {
                                    // ---- Render ability banner first ----
                                    let mut grid_iy: f32 = 42.0;
                                    if let Some(entry) = gs.ability_queue.current_entry() {
                                        let ab_text = i18n::translate_ability(
                                            &entry.ability.full_text,
                                            current_lang(),
                                        );
                                        let ab_lines: Vec<String> =
                                            wrap_ability_text(&ab_text, 392.0, 0.60)
                                                .lines()
                                                .take(2)
                                                .map(|l| l.to_string())
                                                .collect();
                                        let n_lines = ab_lines.len();
                                        let h = 16.0 + n_lines as f32 * 13.0;
                                        unsafe {
                                            _3ds_top_queue_rect(0.0, 42.0, 400.0, h, COL_ABILITY);
                                        }
                                        for (li, line) in ab_lines.iter().enumerate() {
                                            render_text_with_icons(
                                                4.0,
                                                44.0 + li as f32 * 13.0,
                                                line,
                                                COL_LIGHT,
                                                0.60,
                                            );
                                        }
                                        grid_iy = 42.0 + h + 4.0;
                                    }

                                    // ---- Choice cards grid (below ability) ----
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
                                    let cw = 72.0f32;
                                    let ch = cw / 0.711;
                                    let gap = 4.0f32;
                                    let cols = 5usize;
                                    let grid_start = choice_grid_offset;
                                    let grid_len =
                                        display_order.len().saturating_sub(grid_start).min(10);
                                    for gi in 0..grid_len {
                                        let fi = display_order[grid_start + gi];
                                        let di = grid_start + gi;
                                        let act = &acts_cache[fi];
                                        let is_disabled = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.disabled)
                                            .unwrap_or(false);
                                        let col = gi % cols;
                                        let row = gi / cols;
                                        let ix = 4.0 + col as f32 * (cw + gap);
                                        let iy_card = grid_iy + row as f32 * (ch + 16.0 + gap);
                                        if iy_card + ch + 16.0 > 230.0 {
                                            break;
                                        }
                                        // Skip card-image rendering for special
                                        // pay/skip optional cost actions — they use
                                        // card_id as option index, not real card ID.
                                        let is_special_choice = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.card_no.as_deref())
                                            .map(|cn| {
                                                matches!(
                                                    cn,
                                                    "pay_optional_cost"
                                                        | "skip_optional_cost"
                                                        | "yes"
                                                        | "no"
                                                        | "select"
                                                )
                                            })
                                            .unwrap_or(false);
                                        if is_special_choice {
                                            // Render as text label instead
                                            let desc = act
                                                .description
                                                .lines()
                                                .next()
                                                .unwrap_or("")
                                                .to_string();
                                            if !desc.is_empty() {
                                                let c = if is_disabled {
                                                    COL_MED
                                                } else if di == display_pos {
                                                    COL_GOLD
                                                } else {
                                                    COL_LIGHT
                                                };
                                                let pfx =
                                                    if di == display_pos { "> " } else { "  " };
                                                let txt = format!("{}{}", pfx, desc);
                                                unsafe {
                                                    _3ds_top_queue_text(
                                                        ix + 1.0,
                                                        iy_card + 1.0,
                                                        c,
                                                        0.55f32,
                                                        format!("{}\0", txt).as_ptr(),
                                                    );
                                                }
                                            }
                                            continue;
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
                                                            ch + 16.0,
                                                            border,
                                                        );
                                                        _3ds_top_queue_card(
                                                            c_str.as_ptr() as *const u8,
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
                                                            0.45f32,
                                                            format!("{}\0", cn).as_ptr(),
                                                        );
                                                    }
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
                                            let c = if is_disabled {
                                                COL_MED
                                            } else if di == display_pos {
                                                COL_GOLD
                                            } else {
                                                COL_LIGHT
                                            };
                                            let pfx = if di == display_pos { "> " } else { "  " };
                                            let txt = format!("{}{}", pfx, desc);
                                            if txt.contains("{{") {
                                                render_text_with_icons(
                                                    ix + 1.0,
                                                    iy_card + 1.0,
                                                    &txt,
                                                    c,
                                                    0.55,
                                                );
                                            } else {
                                                unsafe {
                                                    _3ds_top_queue_text(
                                                        ix + 1.0,
                                                        iy_card + 1.0,
                                                        c,
                                                        0.55f32,
                                                        format!("{}\0", txt).as_ptr(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    // Hint: L opens text
                                    unsafe {
                                        _3ds_top_queue_text(
                                            4.0,
                                            228.0,
                                            COL_MED,
                                            0.45f32,
                                            format!("{}\0", tl("L=text")).as_ptr(),
                                        );
                                    }
                                    // Page indicator when more choices than visible
                                    if display_order.len() > 10 {
                                        let page = grid_start / 10 + 1;
                                        let total_p = (display_order.len() + 9) / 10;
                                        unsafe {
                                            _3ds_top_queue_text(
                                                300.0,
                                                228.0,
                                                COL_MED,
                                                0.45f32,
                                                format!("{}\0", format!("{}/{}", page, total_p))
                                                    .as_ptr(),
                                            );
                                        }
                                    }
                                    // Text overlay on top of choices grid
                                    if choice_subview {
                                        if let Some(entry) = gs.ability_queue.current_entry() {
                                            let ab_lines: Vec<String> = wrap_ability_text(
                                                &entry.ability.full_text,
                                                384.0,
                                                0.65,
                                            )
                                            .lines()
                                            .map(|l| l.to_string())
                                            .collect();
                                            let lpp = 7usize;
                                            let total_pages =
                                                ((ab_lines.len() + lpp - 1) / lpp).max(1);
                                            if text_page >= total_pages {
                                                text_page = total_pages - 1;
                                            }
                                            let start_line = text_page * lpp;
                                            let end_line = (start_line + lpp).min(ab_lines.len());
                                            unsafe {
                                                _3ds_top_queue_rect(
                                                    0.0, 42.0, 400.0, 198.0, 0xCC000000,
                                                );
                                                _3ds_top_queue_text(
                                                    4.0,
                                                    44.0,
                                                    COL_BLUE,
                                                    0.70f32,
                                                    format!("{}\0", tl("Ability")).as_ptr(),
                                                );
                                            }
                                            let mut oy = 64.0;
                                            for i in start_line..end_line {
                                                render_text_with_icons(
                                                    8.0,
                                                    oy,
                                                    &ab_lines[i],
                                                    COL_LIGHT,
                                                    0.65,
                                                );
                                                oy += 20.0;
                                            }
                                            let page_str =
                                                format!("{}/{}", text_page + 1, total_pages);
                                            unsafe {
                                                _3ds_top_queue_text(
                                                    400.0 - page_str.len() as f32 * 7.0 - 8.0,
                                                    44.0,
                                                    COL_MED,
                                                    0.50f32,
                                                    format!("{}\0", page_str).as_ptr(),
                                                );
                                                _3ds_top_queue_text(
                                                    4.0,
                                                    228.0,
                                                    COL_MED,
                                                    0.50f32,
                                                    format!("{}\0", tl("L/B=close")).as_ptr(),
                                                );
                                            }
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
                                            let name = i18n::card_display_name(
                                                &act.parameters
                                                    .as_ref()
                                                    .and_then(|p| p.card_name.clone())
                                                    .unwrap_or_default(),
                                                current_lang(),
                                            );
                                            let base_cost = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.base_cost)
                                                .unwrap_or(0);
                                            let hdr = if !cn.is_empty() {
                                                format!(
                                                    "[{}] {} {{{{icon_energy.png|E}}}}{}",
                                                    cn, name, base_cost
                                                )
                                            } else {
                                                format!(
                                                    "{} {{{{icon_energy.png|E}}}}{}",
                                                    name, base_cost
                                                )
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
                                            for (li, l) in
                                                wrap_text(&hdr, 392.0, 0.70).lines().enumerate()
                                            {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                let txt = format!(
                                                    "{}{}",
                                                    if li == 0 {
                                                        if is_sel {
                                                            "> "
                                                        } else {
                                                            "  "
                                                        }
                                                    } else {
                                                        ""
                                                    },
                                                    l
                                                );
                                                if txt.contains("{{") {
                                                    render_text_with_icons(
                                                        4.0, ty, &txt, line_color, line_scale,
                                                    );
                                                } else {
                                                    unsafe {
                                                        _3ds_top_queue_text(
                                                            4.0,
                                                            ty,
                                                            line_color,
                                                            line_scale,
                                                            format!("{}\0", txt).as_ptr(),
                                                        );
                                                    }
                                                }
                                                ty += 20.0;
                                            }
                                            for (li, l) in
                                                wrap_text(&areas, 392.0, 0.70).lines().enumerate()
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
                                                                ""
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
                                                game_setup::ActionType::Pass => tl("Pass"),
                                                game_setup::ActionType::PlayMemberToStage => {
                                                    let cn = cn_or_empty(act);
                                                    let name = i18n::card_display_name(
                                                        &act.parameters
                                                            .as_ref()
                                                            .and_then(|p| p.card_name.clone())
                                                            .unwrap_or_default(),
                                                        current_lang(),
                                                    );
                                                    let base_cost = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.base_cost)
                                                        .unwrap_or(0);
                                                    let area = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.stage_area.clone())
                                                        .unwrap_or_default();
                                                    let area_label = tl_area(&area);
                                                    if !cn.is_empty() {
                                                        format!(
                                                            "[{}] {} {{{{icon_energy.png|E}}}}{} {}",
                                                            cn, name, base_cost, area_label
                                                        )
                                                    } else {
                                                        format!(
                                                            "{} {{{{icon_energy.png|E}}}}{} {}",
                                                            name, base_cost, area_label
                                                        )
                                                    }
                                                }
                                                game_setup::ActionType::UseAbility => {
                                                    let name = i18n::card_display_name(
                                                        &act.parameters
                                                            .as_ref()
                                                            .and_then(|p| p.card_name.clone())
                                                            .unwrap_or_default(),
                                                        current_lang(),
                                                    );
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
                                                    let area_label = tl_area(&area);
                                                    let abil = act
                                                        .parameters
                                                        .as_ref()
                                                        .and_then(|p| p.source_ability.clone())
                                                        .unwrap_or_default();
                                                    let abil_short =
                                                        truncate_aware_segments(&abil, 28);
                                                    let cn = cn_or_empty(act);
                                                    if !cn.is_empty() {
                                                        if cost > 0 {
                                                            format!(
                                                                "[{}] {} {} c:{} {}",
                                                                cn,
                                                                name,
                                                                area_label,
                                                                cost,
                                                                abil_short
                                                            )
                                                        } else {
                                                            format!(
                                                                "[{}] {} {} {}",
                                                                cn, name, area_label, abil_short
                                                            )
                                                        }
                                                    } else {
                                                        if cost > 0 {
                                                            format!(
                                                                "{} {} c:{} {}",
                                                                name, area_label, cost, abil_short
                                                            )
                                                        } else {
                                                            format!(
                                                                "{} {} {}",
                                                                name, area_label, abil_short
                                                            )
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    let cn = cn_or_empty(act);
                                                    let name = i18n::card_display_name(
                                                        &act.parameters
                                                            .as_ref()
                                                            .and_then(|p| p.card_name.clone())
                                                            .unwrap_or_default(),
                                                        current_lang(),
                                                    );
                                                    let line = if let Some(sel) = act.selected {
                                                        let label = if sel {
                                                            tl("selected_label")
                                                        } else {
                                                            tl("unselected_label")
                                                        };
                                                        if !cn.is_empty() && !name.is_empty() {
                                                            format!("[{}] [{}] {}", cn, label, name)
                                                        } else if !cn.is_empty() {
                                                            format!("[{}] [{}]", cn, label)
                                                        } else {
                                                            format!("[{}] {}", label, name)
                                                        }
                                                    } else {
                                                        let desc = act
                                                            .display_desc(
                                                                current_lang() == Lang::Japanese,
                                                            )
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
                                                            ability_text
                                                        } else {
                                                            desc
                                                        };
                                                        if !cn.is_empty() && !name.is_empty() {
                                                            format!("[{}] {} {}", cn, name, display)
                                                        } else if !cn.is_empty() {
                                                            format!("[{}] {}", cn, display)
                                                        } else {
                                                            display
                                                        }
                                                    };
                                                    line
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
                                            for (li, l) in
                                                wrap_text(&line, 392.0, 0.70).lines().enumerate()
                                            {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                let pfx = if li == 0 { prefix } else { "" };
                                                let txt = format!("{}{}", pfx, l);
                                                if txt.contains("{{") {
                                                    render_text_with_icons(
                                                        4.0, ty, &txt, color, scale,
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

                        // Highlight interactive zones for all tap-to-deploy action types
                        {
                            let ai_turn =
                                *ai_vs_ai || (*vs_ai && gs.active_player().id != gs.player1.id);
                            let opp_turn =
                                is_multiplayer && !mp_can_act(&gs, if is_host { 0 } else { 1 });
                            if !ai_turn && !opp_turn {
                                for act in &acts_cache {
                                    let p = match &act.parameters {
                                        Some(x) => x,
                                        None => continue,
                                    };
                                    if p.disabled.unwrap_or(false) {
                                        continue;
                                    }
                                    match act.action_type {
                                        // Stage slots for PlayMemberToStage (detail mode, own stage)
                                        game_setup::ActionType::PlayMemberToStage => {
                                            if detail_mode && viewing_card.is_some() {
                                                if p.card_id != viewing_card {
                                                    continue;
                                                }
                                                if let Some(sa) = &p.stage_area {
                                                    let slot = match sa.as_str() {
                                                        "left" => 0i32,
                                                        "center" => 1,
                                                        "right" => 2,
                                                        _ => continue,
                                                    };
                                                    unsafe {
                                                        _3ds_board_set_action_highlight(
                                                            1, slot, false,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        // Stage slots for ChoicePosition (choice mode)
                                        game_setup::ActionType::ChoicePosition => {
                                            if choice_image_mode && gs.has_pending_choice() {
                                                if let Some(sa) = &p.stage_area {
                                                    let slot = match sa.as_str() {
                                                        "left" => 0i32,
                                                        "center" => 1,
                                                        "right" => 2,
                                                        _ => continue,
                                                    };
                                                    unsafe {
                                                        _3ds_board_set_action_highlight(
                                                            1, slot, false,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        // Hand cards for SelectMulligan
                                        game_setup::ActionType::SelectMulligan => {
                                            if let Some(hidx) =
                                                p.card_indices.as_ref().and_then(|v| v.first())
                                            {
                                                unsafe {
                                                    _3ds_board_set_action_highlight(
                                                        3,
                                                        *hidx as i32,
                                                        false,
                                                    );
                                                }
                                            }
                                        }
                                        // Hand cards for SelectLiveCard
                                        game_setup::ActionType::SelectLiveCard => {
                                            if let Some(hidx) =
                                                p.card_indices.as_ref().and_then(|v| v.first())
                                            {
                                                unsafe {
                                                    _3ds_board_set_action_highlight(
                                                        3,
                                                        *hidx as i32,
                                                        false,
                                                    );
                                                }
                                            }
                                        }
                                        // Board cards for choice image mode (ChoiceSelect, ChoiceDecision, ChoiceOption)
                                        _ => {
                                            if choice_image_mode
                                                && gs.has_pending_choice()
                                                && matches!(
                                                    act.action_type,
                                                    game_setup::ActionType::ChoiceSelect
                                                        | game_setup::ActionType::ChoiceDecision
                                                )
                                            {
                                                if let Some(cid) = p.card_id {
                                                    if let Some((zone, slot, opp)) =
                                                        find_card_zone_slot(&gs, cid)
                                                    {
                                                        unsafe {
                                                            _3ds_board_set_action_highlight(
                                                                zone, slot, opp,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Also highlight SelectAutoAbility option cards
                        if !(*ai_vs_ai || (*vs_ai && gs.active_player().id != gs.player1.id))
                            && !(is_multiplayer && !mp_can_act(&gs, if is_host { 0 } else { 1 }))
                            && choice_image_mode
                            && gs.has_pending_choice()
                        {
                            if let Some(c) = gs.get_pending_choice() {
                                use rabuka_engine::ability::types::Choice;
                                if let Choice::SelectAutoAbility { options, .. } = c {
                                    for opt in options {
                                        if let Some(cid) = opt.card_id {
                                            if let Some((zone, slot, opp)) =
                                                find_card_zone_slot(&gs, cid)
                                            {
                                                unsafe {
                                                    _3ds_board_set_action_highlight(
                                                        zone, slot, opp,
                                                    );
                                                }
                                            }
                                        }
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

                    // ===== OVERLAY SYSTEM (START menu, game log, perf stats, revealed cards) =====
                    if zone_viewer.is_none() {
                        match overlay {
                            Overlay::StartMenu(sel) => unsafe {
                                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                _3ds_top_queue_rect(40.0, 50.0, 320.0, 170.0, 0xFF333333);
                                _3ds_top_queue_rect(40.0, 50.0, 320.0, 170.0, 0xFF888888);
                                let menu_title = tl("MENU");
                                _3ds_top_queue_text(
                                    160.0,
                                    58.0,
                                    COL_GOLD,
                                    0.75f32,
                                    format!("{}\0", menu_title).as_ptr(),
                                );
                                let lang_label = current_lang().label();
                                let items = [
                                    tl("Performance"),
                                    tl("Game Log"),
                                    tl("Revealed Cards"),
                                    format!("{}: {}", tl("Language"), lang_label),
                                ];
                                for (i, item) in items.iter().enumerate() {
                                    let iy = 85.0 + i as f32 * 30.0;
                                    let bg = if i == sel { 0xFF557755 } else { 0xFF555555 };
                                    _3ds_top_queue_rect(60.0, iy, 280.0, 26.0, bg);
                                    let prefix = if i == sel { "> " } else { "  " };
                                    _3ds_top_queue_text(
                                        70.0,
                                        iy + 4.0,
                                        COL_LIGHT,
                                        0.60f32,
                                        format!("{}{}\0", prefix, item).as_ptr(),
                                    );
                                }
                                _3ds_top_queue_text(
                                    60.0,
                                    210.0,
                                    COL_MED,
                                    0.50f32,
                                    format!("{}\0", tl("UP/DOWN=move, A=select, B=close")).as_ptr(),
                                );
                            },
                            Overlay::GameLog(offset) => {
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, 0xCC000000);
                                    let log_hdr = tl("Game Log");
                                    _3ds_top_queue_text(
                                        4.0,
                                        2.0,
                                        COL_GOLD,
                                        0.65f32,
                                        format!("{} (B=close, UP/DOWN=scroll)\0", log_hdr).as_ptr(),
                                    );
                                }
                                let logs = &gs.rule_log;
                                let max_vis = 16usize;
                                let mut o = offset;
                                if o + max_vis > logs.len() && logs.len() > max_vis {
                                    o = logs.len() - max_vis;
                                }
                                let mut ly = 22.0_f32;
                                for i in o..logs.len().min(o + max_vis) {
                                    let entry = &logs[i];
                                    let truncated = if entry.chars().count() > 58 {
                                        let cutoff = entry
                                            .char_indices()
                                            .nth(58)
                                            .map(|(i, _)| i)
                                            .unwrap_or(entry.len());
                                        &entry[..cutoff]
                                    } else {
                                        &entry[..]
                                    };
                                    unsafe {
                                        _3ds_top_queue_text(
                                            4.0,
                                            ly,
                                            0xFFCCCCCC,
                                            0.50f32,
                                            format!("{}\0", truncated).as_ptr(),
                                        );
                                    }
                                    ly += 13.0;
                                }
                                if logs.len() > max_vis {
                                    unsafe {
                                        _3ds_top_queue_text(
                                            300.0,
                                            2.0,
                                            COL_MED,
                                            0.50f32,
                                            format!(
                                                "{}-{}/{}\0",
                                                o + 1,
                                                o + max_vis.min(logs.len() - o),
                                                logs.len()
                                            )
                                            .as_ptr(),
                                        );
                                    }
                                }
                            }
                            Overlay::PerfStats(detail) => {
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, 0xCC000000);
                                    let perf_hdr = tl("Performance");
                                    _3ds_top_queue_text(
                                        4.0,
                                        2.0,
                                        COL_GOLD,
                                        0.65f32,
                                        format!("{} (B=close, A=detail)\0", perf_hdr).as_ptr(),
                                    );
                                }
                                let snapshots = &gs.performance_snapshots;
                                if snapshots.is_empty() {
                                    let msg = tl("No performance data yet");
                                    unsafe {
                                        _3ds_top_queue_text(
                                            40.0,
                                            60.0,
                                            COL_MED,
                                            0.65f32,
                                            format!("{}\0", msg).as_ptr(),
                                        );
                                    }
                                } else if let Some(si) = detail {
                                    if si < snapshots.len() {
                                        let s = &snapshots[si];
                                        unsafe {
                                            _3ds_top_queue_text(
                                                4.0,
                                                22.0,
                                                COL_LIGHT,
                                                0.60f32,
                                                format!(
                                                    "{} {} | {} | {}{} | {}{}\0",
                                                    tl("T"),
                                                    s.turn,
                                                    s.player_id,
                                                    tl("Score:"),
                                                    s.total_score,
                                                    tl("Success:"),
                                                    s.success
                                                )
                                                .as_ptr(),
                                            );
                                            _3ds_top_queue_text(
                                                4.0,
                                                36.0,
                                                COL_MED,
                                                0.55f32,
                                                format!("{}\0", tl("Lives:")).as_ptr(),
                                            );
                                        }
                                        let mut ly = 50.0;
                                        for (li, lc) in s.lives.iter().enumerate() {
                                            let cn = gs
                                                .card_database
                                                .get_card(lc.card_id)
                                                .map(|c| &c.card_no[..])
                                                .unwrap_or("?");
                                            let status =
                                                tl(if lc.passed { "PASS" } else { "FAIL" });
                                            unsafe {
                                                _3ds_top_queue_text(
                                                    8.0,
                                                    ly,
                                                    if lc.passed { 0xFF88FF88 } else { 0xFFFF8888 },
                                                    0.50f32,
                                                    format!(
                                                        "{} #{} {} score:{}\0",
                                                        cn, li, status, lc.score
                                                    )
                                                    .as_ptr(),
                                                );
                                            }
                                            ly += 13.0;
                                            if ly > 220.0 {
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    let mut ly = 22.0;
                                    for (_i, s) in snapshots.iter().enumerate().rev().take(10) {
                                        let label = format!(
                                            "T{} {} score:{} hearts:{} pass:{}/{} succ:{}",
                                            s.turn,
                                            s.player_id,
                                            s.total_score,
                                            s.total_hearts.iter().sum::<u32>(),
                                            s.lives.iter().filter(|l| l.passed).count(),
                                            s.lives.len(),
                                            s.success
                                        );
                                        let col = if s.success { 0xFF88FF88 } else { 0xFFFF8888 };
                                        unsafe {
                                            _3ds_top_queue_text(
                                                4.0,
                                                ly,
                                                col,
                                                0.50f32,
                                                format!(
                                                    "{}\0",
                                                    if label.chars().count() > 60 {
                                                        let cutoff = label.char_indices().nth(60).map(|(i, _)| i).unwrap_or(label.len());
                                                        &label[..cutoff]
                                                    } else {
                                                        &label
                                                    }
                                                )
                                                .as_ptr(),
                                            );
                                        }
                                        ly += 12.0;
                                    }
                                }
                            }
                            Overlay::RevealedCards(show_self, rev_scroll) => {
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, 0xCC000000);
                                    let who = if show_self { tl("You") } else { tl("Opponent") };
                                    let rev_hdr = tl("Revealed Cards");
                                    _3ds_top_queue_text(
                                        4.0,
                                        2.0,
                                        COL_GOLD,
                                        0.60f32,
                                        format!("{} ({})  B=close X=toggle\0", rev_hdr, who)
                                            .as_ptr(),
                                    );
                                }
                                // Aggregate sections from all engine vectors
                                struct RevSection<'a> {
                                    label: &'a str,
                                    cards: &'a [i16],
                                }
                                let cheer = if show_self {
                                    &gs.player1_cheer_revealed_cards
                                } else {
                                    &gs.player2_cheer_revealed_cards
                                };
                                let sections: Vec<RevSection> = vec![
                                    RevSection {
                                        label: "Yell",
                                        cards: &gs.initial_yell_revealed_cards,
                                    },
                                    RevSection {
                                        label: "Re-Yell",
                                        cards: &gs.re_yell_revealed_cards,
                                    },
                                    RevSection {
                                        label: "Cheer",
                                        cards: cheer,
                                    },
                                    RevSection {
                                        label: "Cost",
                                        cards: &gs.revealed_cost_cards,
                                    },
                                    RevSection {
                                        label: "Effects",
                                        cards: &gs.revealed_cards,
                                    },
                                ];
                                let total_cards: usize =
                                    sections.iter().map(|s| s.cards.len()).sum();
                                if total_cards == 0 {
                                    let msg = tl("No revealed cards");
                                    unsafe {
                                        _3ds_top_queue_text(
                                            40.0,
                                            60.0,
                                            COL_MED,
                                            0.65f32,
                                            format!("{}\0", msg).as_ptr(),
                                        );
                                    }
                                } else {
                                    let gap = 4.0_f32;
                                    let cw = 56.0_f32;
                                    let ch = cw / 0.711;
                                    let header_h = 14.0_f32;
                                    let mut iy = 18.0_f32;
                                    let mut skip = rev_scroll;
                                    for sec in &sections {
                                        if sec.cards.is_empty() {
                                            continue;
                                        }
                                        // Skip scrolled-off sections
                                        if skip > 0 {
                                            skip -= 1;
                                            continue;
                                        }
                                        // Section header
                                        unsafe {
                                            _3ds_top_queue_text(
                                                4.0,
                                                iy,
                                                COL_GOLD,
                                                0.55f32,
                                                format!("{} ({})\0", sec.label, sec.cards.len())
                                                    .as_ptr(),
                                            );
                                        }
                                        iy += header_h;
                                        if iy > 230.0 {
                                            break;
                                        }
                                        // Card grid for this section
                                        let mut ix = 4.0_f32;
                                        for cid in sec.cards {
                                            let cn = gs
                                                .card_database
                                                .get_card(*cid)
                                                .map(|c| &c.card_no[..])
                                                .unwrap_or("?");
                                            if ix + cw > 396.0 {
                                                ix = 4.0;
                                                iy += ch + gap;
                                                if iy > 230.0 {
                                                    break;
                                                }
                                            }
                                            if let Some((atl, idx)) = atlas.lookup(cn) {
                                                let c_str = std::ffi::CString::new(atl.as_bytes())
                                                    .unwrap_or_default();
                                                unsafe {
                                                    _3ds_top_queue_card(
                                                        c_str.as_ptr() as *const u8,
                                                        *idx as i32,
                                                        ix,
                                                        iy,
                                                        cw,
                                                        ch,
                                                    );
                                                }
                                            } else {
                                                unsafe {
                                                    _3ds_top_queue_rect(ix, iy, cw, ch, 0xFF444444);
                                                    _3ds_top_queue_text(
                                                        ix + 2.0,
                                                        iy + 2.0,
                                                        COL_LIGHT,
                                                        0.40f32,
                                                        format!("{}\0", cn).as_ptr(),
                                                    );
                                                }
                                            }
                                            ix += cw + gap;
                                        }
                                        iy += ch + gap + 4.0;
                                        if iy > 230.0 {
                                            break;
                                        }
                                    }
                                    // Scroll hint
                                    unsafe {
                                        _3ds_top_queue_text(
                                            4.0,
                                            228.0,
                                            COL_MED,
                                            0.45f32,
                                            format!("{}\0", tl("UP/DOWN=scroll")).as_ptr(),
                                        );
                                    }
                                }
                            }
                            Overlay::None => {}
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
                    choice_subview,
                    text_page,
                    choice_grid_offset,
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
                    overlay,
                )
            }
            Step::Done(ref r) => {
                unsafe {
                    _3ds_clear_both();
                }
                match r {
                    Ok(_) => unsafe {
                        _3ds_text_add_bot(format!("{}\n\0", tl("Done! Press START.")).as_ptr());
                    },
                    Err(e) => unsafe {
                        let s = format!("{}\n\0", tl_fmt("ERROR", &[("e", &format!("{}", e))]));
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

#[cfg(feature = "3ds")]
fn find_card_zone_slot(gs: &GameState, cid: i16) -> Option<(i32, i32, bool)> {
    for (pi, p) in [&gs.player1, &gs.player2].iter().enumerate() {
        let opp = pi == 1;
        if let Some(idx) = p.stage.stage.iter().position(|&id| id == cid) {
            return Some((1, idx as i32, opp));
        }
        if let Some(idx) = p.hand.cards.iter().position(|&id| id == cid) {
            return Some((3, idx as i32, opp));
        }
        if let Some(idx) = p
            .success_live_card_zone
            .cards
            .iter()
            .position(|&id| id == cid)
        {
            return Some((0, idx as i32, opp));
        }
        if let Some(idx) = p.energy_zone.cards.iter().position(|&id| id == cid) {
            return Some((2, idx as i32, opp));
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
        Step::Play(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => {
            "Play"
        }
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
    fn _3ds_measure_text_width(text: *const u8, scale: f32) -> f32;

    // Action highlight on board slots
    fn _3ds_board_set_action_highlight(zone: i32, slot: i32, opponent: bool);
    fn _3ds_board_clear_action_highlight();

    // Action overlay (Phase 2: actions on bottom screen, safe per-line copy)
    fn _3ds_board_set_action_overlay_state(count: i32, selected: i32);
    fn _3ds_board_set_action_overlay_text(index: i32, text: *const u8);
    fn _3ds_board_set_overlay_action_idx(display_line: i32, action_index: i32);
    fn _3ds_board_get_overlay_action_idx(display_line: i32) -> i32;
    fn _3ds_board_get_overlay_selected() -> i32;
    fn _3ds_board_clear_action_overlay();
    // QR code scanning (camera + quirc, same tech used by FBI installer)
    fn _3ds_qr_start() -> i32;
    fn _3ds_qr_stop();
    fn _3ds_qr_poll(out_text: *mut u8, out_max: u32) -> i32;
    // Audio (CSND + tremor OGG)
    fn _3ds_audio_init();
    fn _3ds_audio_play_ogg(path: *const u8);
    fn _3ds_audio_stop();
    fn _3ds_audio_set_volume(vol: f32);
}

#[cfg(not(feature = "3ds"))]
fn main() {
    println!("Desktop mode - use: cargo run --bin harness");
}
