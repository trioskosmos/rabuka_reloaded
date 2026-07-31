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

use rabuka_engine::card::{Card, CardDatabase, HeartColor};
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

/// Map HeartColor to index 0-6 (skip BAll/Draw/Score). Returns None for non-color hearts.
fn heart_color_index(color: &HeartColor) -> Option<usize> {
    match color {
        HeartColor::BAll | HeartColor::Draw | HeartColor::Score => None,
        _ => Some(color.index()),
    }
}

/// Format need hearts with text icons matching top screen format.
fn format_need_hearts_icons(hearts: &[u32]) -> String {
    let mut parts = Vec::new();
    for (i, &count) in hearts.iter().enumerate() {
        if count > 0 {
            let label = format!("h{:02}{}", i, count);
            parts.push(heart_label_to_icon(&label));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{{{{icon_heart_06.png|NEED}}}} {}", parts.join(" "))
    }
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
/// Queue text for top screen rendering. C-side OP_TEXT handler parses {{icon}} markup natively.
fn render_text_with_icons(x: f32, y: f32, text: &str, color: u32, scale: f32) {
    let c_str = std::ffi::CString::new(text).unwrap_or_default();
    unsafe {
        _3ds_top_queue_text(x, y, color, scale, c_str.as_ptr() as *const u8);
    }
}

/// Calculate icon display width at a given height, using actual texture aspect ratio.
fn icon_width_for(file: &str, h: f32) -> f32 {
    let icon_name = file.strip_suffix(".png").unwrap_or(file);
    let atlas_name = format!("icon_{}.png.t3x", icon_name);
    let c_str = std::ffi::CString::new(atlas_name.as_str()).unwrap_or_default();
    let aspect = unsafe { _3ds_icon_aspect(c_str.as_ptr() as *const u8) };
    if aspect > 0.0 {
        h * aspect
    } else {
        h
    }
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
    cost: u8,
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

/// Check if a choice action is text-only (no card image to render).
/// Used to separate card choices from text choices in the choice grid.
fn is_text_only(act: &game_setup::Action) -> bool {
    if matches!(
        act.action_type,
        game_setup::ActionType::ChoiceOption | game_setup::ActionType::ChoiceSkip
    ) {
        return true;
    }
    if let Some(cn) = act.parameters.as_ref().and_then(|p| p.card_no.as_deref()) {
        if matches!(
            cn,
            "pay_optional_cost" | "skip_optional_cost" | "yes" | "no" | "select"
        ) {
            return true;
        }
    }
    act.parameters.as_ref().and_then(|p| p.card_id).is_none()
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

/// Render-side font scale multiplier (must match ctru_shim.c font_scale)
const FONT_SCALE: f32 = 1.2;

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
    // C renderer applies FONT_SCALE multiplier to all text — match it here
    let eff_scale = scale * FONT_SCALE;
    let icon_h = (eff_scale * 16.0).max(11.0);
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
                        let seg_w = measure_text_width(&remaining, eff_scale);
                        if line_w + seg_w <= max_px {
                            line_out.push_str(&remaining);
                            line_w += seg_w;
                            break;
                        } else if line_w == 0.0 {
                            let (part, rest) = split_at_px(&remaining, max_px, eff_scale);
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
                        icon_width_for(&icon[..bar], icon_h) + eff_scale * 6.0
                    } else {
                        2.0 * eff_scale * 9.0
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

    /// Build sorted card list from loaded Card database (matches cards.json order).
    /// Build sorted indices into the cards slice by normalized card_no.
    /// Returns just Vec<usize> (18KB) instead of cloning all card strings.
    /// Temporarily allocates normalized strings for sorting, then drops them.
    fn build_qr_sorted(cards: &[Card]) -> Option<Vec<usize>> {
        let n = cards.len();
        let mut pairs: Vec<(String, usize)> = Vec::new();
        pairs.try_reserve(n).ok()?;
        for (i, c) in cards.iter().enumerate() {
            let norm = c
                .card_no
                .replace('\u{FF0B}', "+")
                .replace('\u{FF0D}', "-")
                .replace('\u{30FC}', "-")
                .to_uppercase();
            pairs.push((norm, i));
        }
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let indices: Vec<usize> = pairs.into_iter().map(|(_, idx)| idx).collect();
        Some(indices)
    }

    /// Decode binary QR data: [count+1] [idx_hi+1 idx_lo+1 qty+1] ...
    /// Uses sorted indices to look up card_no from the original cards slice
    /// instead of cloning card_no strings into the sorted list.
    fn decode_qr_binary(
        sorted_indices: &[usize],
        cards: &[Card],
        data: &[u8],
    ) -> Option<Vec<String>> {
        if data.is_empty() {
            return None;
        }
        let count = (data[0] as usize).wrapping_sub(1);
        if count == 0 || data.len() < 1 + count * 3 {
            return None;
        }
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let base = 1 + i * 3;
            let idx = (((data[base] as usize).wrapping_sub(1)) << 8)
                | ((data[base + 1] as usize).wrapping_sub(1));
            let qty = data[base + 2].wrapping_sub(1).max(1) as usize;
            let card_idx = *sorted_indices.get(idx)?;
            let card_no = &cards.get(card_idx)?.card_no;
            for _ in 0..qty {
                result.push(card_no.to_string());
            }
        }
        Some(result)
    }
}

/// Check if a string looks like base64 (QR binary format).
fn looks_like_b64(s: &str) -> bool {
    if s.len() < 4 || s.len() > 3000 {
        return false;
    }
    // Must be all ASCII base64 chars, no spaces/newlines
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

/// Minimal base64 decoder (no_std friendly).
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: i32 = 0;
    for &b in s.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = TABLE.get(b as usize)?;
        if *val < 0 {
            return None;
        }
        buf = (buf << 6) | (*val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
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
    MultiplayerClientScan(usize, u32), // p1_idx, frames_until_rescan
    MultiplayerClientHostSelect(usize, Vec<u16>, usize), // p1_idx, host_node_ids, cursor
    MultiplayerSyncDeck(usize, usize, bool), // p1_idx, p2_idx, is_host
    MultiplayerLoading(usize, usize, bool, Option<Vec<u8>>, u64), // p1_idx, p2_idx, is_host, deck_sync_bytes, seed
    QrScan(usize),          // QR code scanning (usize = context pointer, 0=not started)
    QrResult(Vec<String>),  // QR scan result, user can confirm
    QrNotDeck(String, u32), // QR scanned but not a valid deck, shows decoded text, countdown frames
    ControlGuide(usize),    // Help/control guide overlay (usize = page index)
    DeckViewer(
        Vec<i16>,    // card_ids (resolved to i16, same as zone_viewer)
        usize,       // cursor
        usize,       // offset
        bool,        // vs_ai
        bool,        // is_multiplayer
        Option<i16>, // viewing_card (same as zone_viewer)
        std::sync::Arc<rabuka_engine::card::CardDatabase>, // card_db
        CardAtlas,   // atlas
    ),
}

#[cfg(feature = "3ds")]
#[derive(Clone, Copy, PartialEq)]
enum Overlay {
    None,
    StartMenu(usize),
    GameLog(usize, usize),                   // offset (from end), cursor
    PerfStats(Option<usize>, usize),         // detail snapshot index, cursor
    RevealedCards(bool, usize, Option<i16>), // show_self, flat cursor, viewing card id
}

#[cfg(feature = "3ds")]
#[derive(Clone, Copy, PartialEq)]
enum GridAction {
    None,
    CloseGrid,
    CloseDetail,
    OpenDetail(i16),
    Navigate,
}

#[cfg(feature = "3ds")]
fn card_grid_input(
    keys: u32,
    cursor: &mut usize,
    viewing_card: &mut Option<i16>,
    card_ids: &[i16],
    cols: usize,
) -> GridAction {
    let total = card_ids.len();
    if total == 0 {
        return GridAction::None;
    }
    if keys & 0x00000002 != 0 {
        if viewing_card.is_some() {
            *viewing_card = None;
            return GridAction::CloseDetail;
        } else {
            return GridAction::CloseGrid;
        }
    }
    if keys & 0x00000400 != 0 && viewing_card.is_none() {
        if *cursor < total {
            *viewing_card = Some(card_ids[*cursor]);
            return GridAction::OpenDetail(card_ids[*cursor]);
        }
    }
    if viewing_card.is_none() {
        let mut moved = false;
        if keys & 0x00000040 != 0 {
            *cursor = cursor.saturating_sub(cols);
            moved = true;
        }
        if keys & 0x00000080 != 0 {
            *cursor = (*cursor + cols).min(total - 1);
            moved = true;
        }
        if keys & 0x00000020 != 0 && *cursor > 0 {
            *cursor -= 1;
            moved = true;
        }
        if keys & 0x00000010 != 0 && *cursor + 1 < total {
            *cursor += 1;
            moved = true;
        }
        if moved {
            return GridAction::Navigate;
        }
    }
    GridAction::None
}

#[cfg(feature = "3ds")]
fn render_card_grid(
    card_ids: &[i16],
    cursor: usize,
    cols: usize,
    rows: usize,
    y_start: f32,
    card_db: &rabuka_engine::card::CardDatabase,
    atlas: &CardAtlas,
) {
    let gap = 4.0f32;
    let pp = cols * rows;
    let max_ch = ((240.0 - y_start - gap) / rows as f32) - 14.0;
    let cw = (max_ch * 0.711).min((400.0 - 8.0 - (cols as f32 - 1.0) * gap) / cols as f32);
    let ch = cw / 0.711;
    let page = (cursor / pp) * pp;
    let n = card_ids.len();
    for i in page..n.min(page + pp) {
        let col = (i - page) % cols;
        let row = (i - page) / cols;
        let ix = 4.0 + col as f32 * (cw + gap);
        let iy = y_start + row as f32 * (ch + 14.0 + gap);
        let cid = card_ids[i];
        let border = if i == cursor { COL_GOLD } else { COL_CARD };
        unsafe {
            _3ds_top_queue_rect(ix, iy, cw, ch + 14.0, border);
        }
        let cn = card_db
            .get_card(cid)
            .map(|c| c.card_no.as_ref())
            .unwrap_or("?");
        if let Some((atl, idx)) = atlas.lookup(cn) {
            let c_str = std::ffi::CString::new(atl.as_bytes()).unwrap_or_default();
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
                    0.35f32,
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
}

#[cfg(feature = "3ds")]
fn render_card_detail(card_id: i16, card_db: &rabuka_engine::card::CardDatabase, scroll_y: f32) {
    if let Some(card) = card_db.get_card(card_id) {
        let total_blade = card.blade as i32;
        let score = card.score.unwrap_or(0) as i32;
        let cost = card.cost.unwrap_or(0);
        let heart_str = build_heart_str(
            &card
                .base_heart
                .as_ref()
                .map(|bh| bh.hearts.clone())
                .unwrap_or_default(),
            card_id,
            &Default::default(),
            false,
        );
        let need_heart_str = build_heart_str(
            &card
                .need_heart
                .as_ref()
                .map(|bh| bh.hearts.clone())
                .unwrap_or_default(),
            card_id,
            &Default::default(),
            true,
        );
        let stats = CardDisplayStats {
            total_blade,
            heart_str,
            need_heart_str,
            score,
            cost,
            is_tapped: false,
        };
        unsafe {
            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
            _3ds_top_queue_rect(0.0, 0.0, 400.0, 40.0, COL_CARD);
            let display_name = i18n::card_display_name(&card.name, current_lang());
            _3ds_top_queue_text(
                4.0,
                4.0,
                COL_BLUE,
                0.80f32,
                format!("[{}] {}\0", card.card_no, display_name).as_ptr(),
            );
            render_text_with_icons(
                4.0,
                24.0,
                &card_stat_line(
                    stats.total_blade,
                    &stats.heart_str,
                    stats.score,
                    stats.cost.into(),
                    stats.is_tapped,
                    card.card_type.as_card_str(),
                    &stats.need_heart_str,
                ),
                COL_LIGHT,
                0.65f32,
            );
            // Scrollable ability text area (below the 40px header)
            _3ds_top_queue_rect(0.0, 40.0, 400.0, 200.0, COL_CARD);
            let mut ty = 44.0 - scroll_y;
            let abs: Vec<_> = card.resolved_abilities().collect();
            if abs.is_empty() {
                let raw = card.ability_text();
                if !raw.is_empty() {
                    let clean = raw.replace('\n', " ");
                    let w = wrap_ability_text(&clean, 392.0, 0.65);
                    for line in w.lines() {
                        if ty > -20.0 && ty < 240.0 {
                            render_text_with_icons(4.0, ty, line, COL_LIGHT, 0.65);
                        }
                        ty += 18.0;
                    }
                }
            } else {
                for ab in abs {
                    let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
                    let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                    for line in w.lines() {
                        if ty > -20.0 && ty < 240.0 {
                            render_text_with_icons(4.0, ty, line, COL_LIGHT, 0.65);
                        }
                        ty += 18.0;
                    }
                    ty += 3.0;
                }
            }
            // Scroll indicator if content extends beyond screen
            if ty > 220.0 {
                _3ds_top_queue_text(380.0, 225.0, COL_MED, 0.50f32, format!("v\0").as_ptr());
            }
            if scroll_y > 0.0 {
                _3ds_top_queue_text(380.0, 42.0, COL_MED, 0.50f32, format!("^\0").as_ptr());
            }
        }
    }
}

/// Render a consistent hint bar at the bottom of the top screen.
/// Place this in the overlay's rendering code to show button hints.
/// The y position is always 225 (above the 240px bottom edge).
const HINT_BAR_Y: f32 = 218.0;
const HINT_BAR_SCALE: f32 = 0.58;
#[cfg(feature = "3ds")]
fn render_hint_bar(text: &str) {
    unsafe {
        _3ds_top_queue_text(
            4.0,
            HINT_BAR_Y,
            COL_MED,
            HINT_BAR_SCALE,
            format!("{}\0", text).as_ptr(),
        );
    }
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
        bool,                                   // vs_ai (human vs AI)
        bool,                                   // ai_vs_ai (spectator: both AI)
        bool,                                   // cli_mode
        bool,                                   // detail_mode
        bool,                                   // choice_image_mode
        bool,                       // choice_subview (false=choices grid, true=text overlay)
        usize,                      // text_page (current page index in text subview)
        usize,                      // choice_grid_offset (scroll offset for choice image grid)
        usize,                      // list_scroll (stable scroll offset for action list)
        f32,                        // detail_scroll_y (scroll offset for card detail text)
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
        uds::StateReceiver,         // in-progress authoritative state reassembly (client)
        Option<(u16, Vec<Vec<u8>>, Vec<bool>)>, // host: pending (state_seq, chunks, acked_bitmap) awaiting client ACK
        bool,                                   // host: initial authoritative state has been staged
        Option<Vec<u8>>, // client: last action bytes, retransmitted until host state arrives
        u32,             // host: last client action_seq processed (dedup)
        u32,             // client: next action_seq to send
        u32,             // packet debug counter: bytes sent
        u32,             // packet debug counter: bytes received
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
                        let cur = cur.min(3);
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
                                _3ds_text_add_top(
                                    format!(
                                        "{}\n\0",
                                        match current_lang() {
                                            Lang::English => "English / 英語",
                                            Lang::Japanese => "日本語 / English",
                                        }
                                    )
                                    .as_ptr(),
                                );
                                let tip = tl("L=help R=lang/言語 A=confirm B=back");
                                _3ds_text_add_top(format!("\n{}\0", tip).as_ptr());
                            } else {
                                _3ds_top_clear();
                                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                _3ds_top_queue_text(
                                    100.0,
                                    8.0,
                                    COL_GOLD,
                                    0.65f32,
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
                                render_hint_bar(&tl("L=help  R=lang/言語  A=confirm  B=back"));
                            }
                        }
                        if keys & 0x00000200 != 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::ControlGuide(0),
                                true,
                            )
                        } else if keys & 0x00000040 != 0 && cur > 0 {
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
                        } else if keys & 0x00000002 != 0 {
                            Step::Done(Ok(()))
                        } else if keys & 0x00000008 != 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickMode(cur),
                                false,
                            )
                        } else if keys & 0x00000001 != 0 {
                            if cur == 2 {
                                // "QR Scan"
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::QrScan(0),
                                    true,
                                )
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
                                    _3ds_text_add_top("\nA=select B=back\0".as_ptr());
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        80.0,
                                        8.0,
                                        COL_GOLD,
                                        0.65f32,
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
                                            0.65f32,
                                            format!("{}\0", decks[i].name).as_ptr(),
                                        );
                                    }
                                }
                                render_hint_bar(&tl(
                                    "UP/DOWN=select  A=confirm  X=preview  B=back",
                                ));
                            }
                        }
                        // X = preview deck contents
                        if keys & 0x00000400 != 0 && cur < n {
                            let card_db = std::sync::Arc::new(CardDatabase::load_or_create(
                                cards.as_ref().clone(),
                            ));
                            let card_ids: Vec<i16> =
                                DeckParser::deck_list_to_card_numbers(&decks[cur])
                                    .iter()
                                    .filter_map(|cn| card_db.get_card_id(cn))
                                    .collect();
                            let deck_atlas = CardAtlas::load();
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::DeckViewer(
                                    card_ids,
                                    0,
                                    0,
                                    vs_ai,
                                    is_multiplayer,
                                    None,
                                    card_db,
                                    deck_atlas,
                                ),
                                true,
                            )
                        } else if keys & 0x00000040 != 0 && cur > 0 {
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
                        } else if keys & 0x00000002 != 0 {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(4), true)
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
                                    _3ds_text_add_top("\nA=select B=back\0".as_ptr());
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
                                        0.65f32,
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
                                            0.65f32,
                                            format!("{}\0", decks[i].name).as_ptr(),
                                        );
                                    }
                                }
                                render_hint_bar(&tl("UP/DOWN=select  A=confirm  B=back"));
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
                        } else if keys & 0x00000002 != 0 {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(4), true)
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
                                        0.65f32,
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
                                            0.65f32,
                                            format!("{}\0", decks[i].name).as_ptr(),
                                        );
                                    }
                                }
                                render_hint_bar(&tl("X=preview  A=select  B=use same"));
                            }
                        }
                        // X = preview deck contents
                        if keys & 0x00000400 != 0 && cur < n {
                            let card_db = std::sync::Arc::new(CardDatabase::load_or_create(
                                cards.as_ref().clone(),
                            ));
                            let card_ids: Vec<i16> =
                                DeckParser::deck_list_to_card_numbers(&decks[cur])
                                    .iter()
                                    .filter_map(|cn| card_db.get_card_id(cn))
                                    .collect();
                            let deck_atlas = CardAtlas::load();
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::DeckViewer(
                                    card_ids, 0, 0, false, false, None, card_db, deck_atlas,
                                ),
                                true,
                            )
                        } else if keys & 0x00000040 != 0 && cur > 0 {
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
                                    false,  // ai_vs_ai
                                    false,  // cli_mode (start in game mode)
                                    false,  // detail_mode
                                    true,   // choice_image_mode
                                    false,  // choice_subview (false=choices grid)
                                    0,      // text_page
                                    0,      // choice_grid_offset
                                    0,      // list_scroll
                                    0.0f32, // detail_scroll_y
                                    0,      // hand_offset
                                    0,      // hand_offset_p2
                                    0,      // touch_tap_count
                                    None,   // viewing_card
                                    None,   // zone_viewer
                                    0,      // zone_viewer_offset
                                    false,  // was_touching
                                    false,  // is_multiplayer
                                    false,  // is_host
                                    false,  // waiting_for_opponent
                                    Overlay::None,
                                    uds::StateReceiver::new(),
                                    None,
                                    false,
                                    None,
                                    0,
                                    1,
                                    0,
                                    0,
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
                    SetupPhase::QrScan(ctx) => {
                        let mut qr_ctx = ctx;
                        let mut qr_start_failed = false;
                        if was_dirty && qr_ctx == 0 {
                            let ptr = unsafe { _3ds_qr_start() };
                            if ptr.is_null() {
                                unsafe {
                                    _3ds_clear_both();
                                    _3ds_text_add_top(
                                        format!("{}\n\0", tl("Camera init failed")).as_ptr(),
                                    );
                                    _3ds_text_add_top(format!("{}\0", tl("B=back")).as_ptr());
                                }
                                qr_start_failed = true;
                            } else {
                                qr_ctx = ptr as usize;
                                if unsafe { _3ds_is_cli_mode() } {
                                    unsafe {
                                        _3ds_clear_top();
                                        _3ds_text_add_top(
                                            format!("{}\n\0", tl("QR SCAN")).as_ptr(),
                                        );
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
                                        let qr_hdr = tl("QR SCAN");
                                        _3ds_top_queue_text(
                                            120.0,
                                            8.0,
                                            COL_GOLD,
                                            0.65f32,
                                            format!("{}\0", qr_hdr).as_ptr(),
                                        );
                                        let qr_msg = tl("Point camera at deck QR code");
                                        _3ds_top_queue_text(
                                            40.0,
                                            60.0,
                                            COL_LIGHT,
                                            0.65f32,
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
                                            220.0,
                                            COL_MED,
                                            0.60f32,
                                            format!("{}\0", qr_cancel).as_ptr(),
                                        );
                                    }
                                }
                            }
                        }
                        if qr_start_failed {
                            unsafe {
                                _3ds_clear_both();
                            }
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
                        } else if keys & 0x00000002 != 0 {
                            if qr_ctx != 0 {
                                unsafe {
                                    _3ds_qr_free(qr_ctx as *mut u8);
                                }
                            }
                            unsafe {
                                _3ds_clear_both();
                            }
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
                        } else {
                            let mut buf = [0u8; 2048];
                            let r = unsafe {
                                _3ds_qr_poll(qr_ctx as *mut u8, buf.as_mut_ptr(), buf.len() as u32)
                            };
                            if r > 0 {
                                dprintln!("[QR] poll r={}", r);
                                if qr_ctx != 0 {
                                    unsafe {
                                        _3ds_qr_free(qr_ctx as *mut u8);
                                    }
                                }
                                dprintln!("[QR] freed context");
                                let text = String::from_utf8_lossy(&buf[..r as usize]).to_string();
                                dprintln!(
                                    "[QR] text len={} b64={}",
                                    text.len(),
                                    looks_like_b64(&text)
                                );
                                // Try binary QR: base64-encoded binary index format
                                let cards_read = if looks_like_b64(&text) {
                                    dprintln!("[QR] b64 decode...");
                                    if let Some(decoded) = base64_decode(&text) {
                                        dprintln!(
                                            "[QR] b64 ok len={} building sorted...",
                                            decoded.len()
                                        );
                                        if let Some(sorted) = CardAtlas::build_qr_sorted(&cards) {
                                            dprintln!("[QR] sorted={} decode...", sorted.len());
                                            let result = CardAtlas::decode_qr_binary(
                                                &sorted, &cards, &decoded,
                                            );
                                            dprintln!("[QR] decode={:?})", result.is_some());
                                            result.unwrap_or_default()
                                        } else {
                                            dprintln!("[QR] sorted alloc FAILED");
                                            Vec::new()
                                        }
                                    } else {
                                        dprintln!("[QR] b64 decode FAILED");
                                        Vec::new()
                                    }
                                } else {
                                    dprintln!("[QR] not b64, text={}", &text[..text.len().min(40)]);
                                    Vec::new()
                                };
                                dprintln!("[QR] cards_read={}", cards_read.len());
                                let cards_read = if cards_read.is_empty() {
                                    DeckParser::parse_deck_content(&text)
                                } else {
                                    cards_read
                                };
                                dprintln!(
                                    "[QR] final={} entering QrResult/NotDeck",
                                    cards_read.len()
                                );
                                if cards_read.is_empty() {
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::QrNotDeck(text, 90),
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
                            } else if r < 0 {
                                if qr_ctx != 0 {
                                    unsafe {
                                        _3ds_qr_free(qr_ctx as *mut u8);
                                    }
                                }
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
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::PickMode(2),
                                    true,
                                )
                            } else {
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::QrScan(qr_ctx),
                                    false,
                                )
                            }
                        }
                    }
                    SetupPhase::QrResult(cards_read) => {
                        dprintln!(
                            "[QR] QrResult entered, {} cards, dirty={}",
                            cards_read.len(),
                            was_dirty
                        );
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
                                        0.65f32,
                                        format!("{}\0", tl("QR DECK")).as_ptr(),
                                    );
                                    _3ds_top_queue_text(
                                        200.0,
                                        32.0,
                                        COL_LIGHT,
                                        0.65f32,
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
                            unsafe {
                                _3ds_clear_both();
                            }
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
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
                                    quantity: qty as u8,
                                })
                                .collect();
                            let qr_deck = DeckList {
                                name: "QR Scanned".to_string(),
                                entries,
                            };
                            let mut new_decks = decks.clone();
                            new_decks.push(qr_deck);
                            unsafe {
                                _3ds_clear_both();
                            }
                            Step::Setup(cards.clone(), new_decks, SetupPhase::PickMode(0), true)
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::QrResult(cards_read.clone()),
                                false,
                            )
                        }
                    }
                    SetupPhase::QrNotDeck(scanned_text, frames_left) => {
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                    _3ds_text_add_top(
                                        format!("{}\n\0", tl("NOT A DECK QR")).as_ptr(),
                                    );
                                    let preview = if scanned_text.len() > 40 {
                                        &scanned_text[..40]
                                    } else {
                                        &scanned_text
                                    };
                                    _3ds_text_add_top(format!("  {}\n\0", preview).as_ptr());
                                    _3ds_text_add_top(format!("\n{}\n\0", tl("B=back")).as_ptr());
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        100.0,
                                        8.0,
                                        COL_GOLD,
                                        0.65f32,
                                        format!("{}\0", tl("NOT A DECK QR")).as_ptr(),
                                    );
                                    let preview = if scanned_text.len() > 40 {
                                        &scanned_text[..40]
                                    } else {
                                        &scanned_text
                                    };
                                    _3ds_top_queue_text(
                                        20.0,
                                        60.0,
                                        COL_LIGHT,
                                        0.60f32,
                                        format!("{}\0", preview).as_ptr(),
                                    );
                                    _3ds_top_queue_text(
                                        20.0,
                                        220.0,
                                        COL_MED,
                                        0.60f32,
                                        format!("{}\0", tl("B=back")).as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000002 != 0 {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(2), true)
                        } else if frames_left > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::QrNotDeck(scanned_text, frames_left - 1),
                                false,
                            )
                        } else {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::QrScan(0), true)
                        }
                    }
                    SetupPhase::DeckViewer(
                        ref card_ids,
                        mut offset,
                        _,
                        vs_ai,
                        is_multiplayer,
                        ref mut viewing_card,
                        ref card_db,
                        ref atlas,
                    ) => {
                        if was_dirty {
                            unsafe {
                                _3ds_top_clear();
                                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                _3ds_top_queue_text(
                                    4.0,
                                    4.0,
                                    COL_GOLD,
                                    0.65f32,
                                    format!("{}  (B=close, X=detail)\0", tl("DECK PREVIEW"))
                                        .as_ptr(),
                                );
                            }
                            if viewing_card.is_none() {
                                render_card_grid(card_ids, offset, 5, 2, 28.0, card_db, atlas);
                            } else {
                                render_card_detail(viewing_card.unwrap(), card_db, 0.0);
                            }
                        }
                        let action = card_grid_input(keys, &mut offset, viewing_card, card_ids, 5);
                        match action {
                            GridAction::CloseGrid => Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::PickDeck(offset / 10, vs_ai, is_multiplayer),
                                true,
                            ),
                            GridAction::CloseDetail => Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::DeckViewer(
                                    card_ids.clone(),
                                    offset,
                                    0,
                                    vs_ai,
                                    is_multiplayer,
                                    *viewing_card,
                                    card_db.clone(),
                                    atlas.clone(),
                                ),
                                true,
                            ),
                            GridAction::None => Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::DeckViewer(
                                    card_ids.clone(),
                                    offset,
                                    0,
                                    vs_ai,
                                    is_multiplayer,
                                    *viewing_card,
                                    card_db.clone(),
                                    atlas.clone(),
                                ),
                                false,
                            ),
                            _ => Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::DeckViewer(
                                    card_ids.clone(),
                                    offset,
                                    0,
                                    vs_ai,
                                    is_multiplayer,
                                    *viewing_card,
                                    card_db.clone(),
                                    atlas.clone(),
                                ),
                                true,
                            ),
                        }
                    }
                    SetupPhase::ControlGuide(page) => {
                        const GUIDE_PAGES: &[&str] = &[
                            "=== MENU CONTROLS ===\n\n\
                             UP/DOWN = Navigate items\n\
                             A = Confirm / Select\n\
                             B = Back / Cancel\n\
                             X = Preview deck contents\n\
                             L = Open this help guide\n\
                             R = Toggle language",
                            "=== IN-GAME CONTROLS ===\n\n\
                             Touch = Select cards on board\n\
                             L = View card detail overlay\n\
                             X = Toggle detail mode\n\
                             R = Toggle action view (debug)\n\
                             Y = Switch text/graphic mode (debug)\n\
                             START = In-game pause menu",
                            "=== GAME MODES ===\n\n\
                             VS AI: Play against the computer\n\
                             Sandbox: Local 2-player hotseat\n\
                             QR Scan: Import deck via QR code\n\
                             Local MP: Play on local network",
                            "=== CARD ZONES ===\n\n\
                             Hand: Cards you can play\n\
                             Energy: Powers member abilities\n\
                             Stage: Active battle area\n\
                             Success: Scored cards (win here)\n\
                             Wait: Drawn-from-deck pile\n\
                             Deck: Face-down draw pile",
                        ];
                        let total = GUIDE_PAGES.len();
                        let page = page.min(total - 1);
                        let guide_text = GUIDE_PAGES[page];
                        if was_dirty {
                            if unsafe { _3ds_is_cli_mode() } {
                                unsafe {
                                    _3ds_clear_top();
                                    _3ds_text_add_top(format!("{}\n\0", tl("HELP")).as_ptr());
                                    for line in guide_text.split('\n') {
                                        _3ds_text_add_top(format!("{}\n\0", line).as_ptr());
                                    }
                                    _3ds_text_add_top(
                                        format!(
                                            "\nPage {}/{}  L/R=pages  B=back\0",
                                            page + 1,
                                            total
                                        )
                                        .as_ptr(),
                                    );
                                }
                            } else {
                                unsafe {
                                    _3ds_top_clear();
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_rect(20.0, 15.0, 360.0, 195.0, 0xE60A0E1Au32);
                                    _3ds_top_queue_rect(20.0, 15.0, 360.0, 2.0, COL_DIM);
                                    _3ds_top_queue_rect(20.0, 208.0, 360.0, 2.0, COL_DIM);
                                    _3ds_top_queue_rect(20.0, 15.0, 2.0, 195.0, COL_DIM);
                                    _3ds_top_queue_rect(378.0, 15.0, 2.0, 195.0, COL_DIM);
                                    let mut y = 25.0f32;
                                    for line in guide_text.split('\n') {
                                        if line.starts_with("===") {
                                            _3ds_top_queue_text(
                                                40.0,
                                                y,
                                                COL_GOLD,
                                                0.65f32,
                                                format!("{}\0", line).as_ptr(),
                                            );
                                        } else if !line.is_empty() {
                                            _3ds_top_queue_text(
                                                30.0,
                                                y,
                                                COL_LIGHT,
                                                0.60f32,
                                                format!("{}\0", line).as_ptr(),
                                            );
                                        }
                                        y += 16.0;
                                    }
                                    _3ds_top_queue_text(
                                        4.0,
                                        215.0,
                                        COL_MED,
                                        0.55f32,
                                        format!(
                                            "Page {}/{}   L/R=pages  B=back\0",
                                            page + 1,
                                            total
                                        )
                                        .as_ptr(),
                                    );
                                }
                            }
                        }
                        if keys & 0x00000002 != 0 {
                            Step::Setup(cards.clone(), decks.clone(), SetupPhase::PickMode(0), true)
                        } else if keys & 0x00000100 != 0 && page + 1 < total {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::ControlGuide(page + 1),
                                true,
                            )
                        } else if keys & 0x00000200 != 0 && page > 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::ControlGuide(page - 1),
                                true,
                            )
                        } else {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::ControlGuide(page),
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
                                    _3ds_text_add_top(
                                        "\nUP/DOWN=select A=confirm B=back\0".as_ptr(),
                                    );
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
                                        0.65f32,
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
                                            0.65f32,
                                            format!("{}\0", m).as_ptr(),
                                        );
                                    }
                                }
                                render_hint_bar(&tl("UP/DOWN=select  A=confirm  B=back"));
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
                                    SetupPhase::MultiplayerClientScan(deck_idx, 0),
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
                                                0.65f32,
                                                format!("{}\0", tl("HOST: Network created!"))
                                                    .as_ptr(),
                                            );
                                            let wait_msg = tl("Waiting for client...");
                                            _3ds_top_queue_text(
                                                50.0,
                                                100.0,
                                                COL_LIGHT,
                                                0.65f32,
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
                                                0.65f32,
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
                                // Client connected! Read client's deck index from hello packet
                                let p2_idx = if n >= 2 { hello[1] as usize } else { 0 };
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerSyncDeck(p1_idx, p2_idx, true),
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
                    SetupPhase::MultiplayerClientScan(p1_idx, frames) => {
                        // B = back to role selection
                        if keys & 0x00000002 != 0 {
                            uds::uds_exit();
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerPickRole(p1_idx, 0),
                                true,
                            )
                        } else {
                            // A = force rescan now, or auto-rescan every ~3s (180 frames)
                            let do_scan = was_dirty || keys & 0x00000001 != 0 || frames == 0;
                            if do_scan {
                                if was_dirty || keys & 0x00000001 != 0 {
                                    let _ = uds::uds_init(false);
                                }
                                let hosts = uds::uds_scan_networks();
                                if hosts.is_empty() {
                                    // No hosts found — rescan after delay
                                    if unsafe { _3ds_is_cli_mode() } {
                                        unsafe {
                                            _3ds_clear_top();
                                            _3ds_text_add_top(
                                                format!("{}\n\0", tl("Scanning...")).as_ptr(),
                                            );
                                            _3ds_text_add_top(
                                                format!("{}\n\0", tl("A=refresh B=back")).as_ptr(),
                                            );
                                        }
                                    } else {
                                        unsafe {
                                            _3ds_top_clear();
                                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                            _3ds_top_queue_text(
                                                80.0,
                                                100.0,
                                                COL_MED,
                                                0.75f32,
                                                format!("{}\0", tl("Scanning...")).as_ptr(),
                                            );
                                            _3ds_top_queue_text(
                                                80.0,
                                                230.0,
                                                COL_MED,
                                                0.60f32,
                                                format!("{}\0", tl("A=refresh B=back")).as_ptr(),
                                            );
                                        }
                                    }
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::MultiplayerClientScan(p1_idx, 180),
                                        false,
                                    )
                                } else {
                                    // Hosts found — go to selection
                                    Step::Setup(
                                        cards.clone(),
                                        decks.clone(),
                                        SetupPhase::MultiplayerClientHostSelect(p1_idx, hosts, 0),
                                        true,
                                    )
                                }
                            } else {
                                // Waiting for rescan timer — decrement and keep scanning text
                                Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerClientScan(p1_idx, frames - 1),
                                    false,
                                )
                            }
                        }
                    }
                    // Multiplayer: Client selecting which host to connect to
                    SetupPhase::MultiplayerClientHostSelect(p1_idx, ref hosts, cursor) => {
                        let n = hosts.len();
                        if n == 0 {
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerClientScan(p1_idx, 0),
                                true,
                            )
                        } else if keys & 0x00000002 != 0 {
                            // B = back to scan
                            Step::Setup(
                                cards.clone(),
                                decks.clone(),
                                SetupPhase::MultiplayerClientScan(p1_idx, 0),
                                true,
                            )
                        } else if keys & 0x00000001 != 0 {
                            // A = connect to selected host
                            let selected = hosts[cursor];
                            match uds::uds_connect_network(selected) {
                                Ok(()) => {
                                    let mut hello = [0xAAu8, (p1_idx & 0xFF) as u8];
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
                                    let prefix = "";
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
                                        0.65f32,
                                        format!("{}\0", tl("SELECT HOST")).as_ptr(),
                                    );
                                }
                                for i in 0..n {
                                    let y = 50.0 + i as f32 * 32.0;
                                    let col = if i == new_cursor { COL_SEL } else { COL_LIGHT };
                                    let prefix = "";
                                    unsafe {
                                        _3ds_top_queue_text(
                                            40.0,
                                            y,
                                            col,
                                            0.65f32,
                                            format!("{}{}\0", prefix, format!("Host {}", i + 1))
                                                .as_ptr(),
                                        );
                                    }
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        40.0,
                                        220.0,
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
                            // Host: send template IDs so client calls create_copy in the same order → matching instance IDs.
                            let seed = unsafe { _3ds_system_tick() } as u64;
                            let r = (|| -> Result<(), String> {
                                use rabuka_engine::card::CardDatabase;
                                let mut cards_vec = (**cards).clone();
                                CardLoader::attach_abilities(&mut cards_vec);
                                let db = CardDatabase::load_or_create(cards_vec);
                                let nums1 = DeckParser::deck_list_to_card_numbers(&decks[p1_idx]);
                                let nums2 = if p1_idx == p2_idx {
                                    nums1.clone()
                                } else {
                                    DeckParser::deck_list_to_card_numbers(&decks[p2_idx])
                                };
                                // Convert card_no strings to template IDs (same on both machines)
                                let to_ids = |nos: &Vec<String>| -> Vec<u16> {
                                    nos.iter()
                                        .filter_map(|no| db.get_card_id(no).map(|id| id as u16))
                                        .collect()
                                };
                                let sync = uds::DeckSync {
                                    seed,
                                    p1_main_templates: to_ids(&nums1),
                                    p1_energy_templates: Vec::new(),
                                    p2_main_templates: to_ids(&nums2),
                                    p2_energy_templates: Vec::new(),
                                };
                                let data = sync.to_bytes();
                                uds::uds_send(&data).map_err(|e| format!("Send: {}", e))?;
                                Ok(())
                            })();
                            match r {
                                Ok(()) => Step::Setup(
                                    cards.clone(),
                                    decks.clone(),
                                    SetupPhase::MultiplayerLoading(
                                        p1_idx, p2_idx, true, None, seed,
                                    ),
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
                                            0.65f32,
                                            format!("{}\0", tl("Receiving deck data...")).as_ptr(),
                                        );
                                    }
                                }
                            }
                            // Try to receive deck sync
                            let mut recv_buf = [0u8; 4096];
                            match uds::uds_recv(&mut recv_buf) {
                                Ok(n) if n > 0 => {
                                    if let Some(sync) = uds::DeckSync::from_bytes(&recv_buf[..n]) {
                                        let sync_bytes = recv_buf[..n].to_vec();
                                        Step::Setup(
                                            cards.clone(),
                                            decks.clone(),
                                            SetupPhase::MultiplayerLoading(
                                                p1_idx,
                                                p2_idx,
                                                false,
                                                Some(sync_bytes),
                                                sync.seed,
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
                    SetupPhase::MultiplayerLoading(
                        p1_idx,
                        p2_idx,
                        is_host,
                        deck_sync_bytes,
                        seed,
                    ) => {
                        let r = (|| -> Result<(GameState, CardAtlas), String> {
                            let mut cards_vec = (**cards).clone();
                            CardLoader::attach_abilities(&mut cards_vec);
                            let mut db = Arc::new(CardDatabase::load_or_create(cards_vec));
                            // If we have deck sync data from host, use it directly
                            if let Some(ref sync_bytes) = deck_sync_bytes {
                                let sync = uds::DeckSync::from_bytes(sync_bytes)
                                    .ok_or("Invalid deck sync data")?;
                                // Build decks from template IDs using create_copy in same order → matching instance IDs
                                let build_from_templates = |db: &mut Arc<CardDatabase>,
                                                            templates: &Vec<u16>|
                                 -> Result<
                                    rabuka_engine::deck_builder::Deck,
                                    String,
                                > {
                                    let mut deck = rabuka_engine::deck_builder::Deck {
                                        main_deck: std::collections::VecDeque::new(),
                                        energy_deck: std::collections::VecDeque::new(),
                                    };
                                    for &tid in templates {
                                        let cid = Arc::make_mut(db).create_copy(tid as i16);
                                        if let Some(card) = db.get_card(cid) {
                                            match card.card_type {
                                                rabuka_engine::card::CardType::Energy => {
                                                    deck.energy_deck.push_back(cid)
                                                }
                                                _ => deck.main_deck.push_back(cid),
                                            }
                                        }
                                    }
                                    Ok(deck)
                                };
                                let mut pd1 =
                                    build_from_templates(&mut db, &sync.p1_main_templates)
                                        .map_err(|e| format!("Deck1: {}", e))?;
                                let mut pd2 =
                                    build_from_templates(&mut db, &sync.p2_main_templates)
                                        .map_err(|e| format!("Deck2: {}", e))?;
                                DeckBuilder::add_default_energy_cards_from_database(
                                    &mut pd1, &mut db,
                                )
                                .ok();
                                DeckBuilder::add_default_energy_cards_from_database(
                                    &mut pd2, &mut db,
                                )
                                .ok();
                                // Shuffle with same seed as host for identical deck order
                                rabuka_engine::rng::seed(sync.seed as u32);
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
                            rabuka_engine::rng::seed(seed as u32);
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
                                    0,        // list_scroll
                                    0.0f32,   // detail_scroll_y
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
                                    uds::StateReceiver::new(),
                                    None,
                                    false,
                                    None,
                                    0,
                                    1,
                                    0,
                                    0,
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
                mut list_scroll,
                mut detail_scroll_y,
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
                mut state_rx,
                mut pending_state,
                mut state_init,
                mut pending_client_action,
                mut last_client_action_seq,
                mut next_action_seq,
                mut dbg_tx_bytes,
                mut dbg_rx_bytes,
            ) => {
                // Web server pattern: use player_idx (0 or 1) for perspective.
                // No long-lived borrows on gs — look up inline.
                let my_player_idx: usize = if is_multiplayer {
                    if is_host {
                        0
                    } else {
                        1
                    }
                } else {
                    0
                };
                let my_id: i32 = my_player_idx as i32;
                // General check: do the current choice actions have card images?
                // ChoiceOption actions with card_id → image grid. Otherwise → text fallback.
                // Image mode: SelectCard only. Text mode: ChoiceOption/answer_based.
                let has_image_choice = choice_image_mode
                    && gs.has_pending_choice()
                    && matches!(
                        gs.get_pending_choice(),
                        Some(rabuka_engine::ability::types::Choice::SelectCard { .. })
                    );
                let has_text_choice = gs.has_pending_choice()
                    && acts_cache
                        .iter()
                        .any(|a| a.action_type == game_setup::ActionType::ChoiceOption);
                #[inline(always)]
                fn pref<'a>(gs: &'a GameState, idx: usize) -> &'a Player {
                    if idx == 0 {
                        &gs.player1
                    } else {
                        &gs.player2
                    }
                }
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
                                    0 => Overlay::PerfStats(None, 0),
                                    1 => Overlay::GameLog(0, 0),
                                    2 => Overlay::RevealedCards(true, 0, None),
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
                        Overlay::GameLog(ref mut offset, ref mut cursor) => {
                            let n = gs.rule_log.len();
                            if n == 0 { /* nothing */
                            } else {
                                let max_vis = 12usize;
                                if keys & 0x00000040 != 0 {
                                    if *offset == 0 {
                                        *offset = n.saturating_sub(max_vis);
                                    } else {
                                        *offset = offset.saturating_sub(1);
                                    }
                                    if *cursor >= *offset + max_vis || *cursor < *offset {
                                        *cursor = *offset;
                                    }
                                    redraw = true;
                                }
                                if keys & 0x00000080 != 0 {
                                    let max_off = n.saturating_sub(max_vis);
                                    if *offset >= max_off {
                                        *offset = 0;
                                    } else {
                                        *offset = offset.saturating_add(1).min(max_off);
                                    }
                                    if *cursor >= *offset + max_vis || *cursor < *offset {
                                        *cursor = *offset;
                                    }
                                    redraw = true;
                                }
                                if keys & 0x00000020 != 0 {
                                    *offset = offset.saturating_sub(max_vis);
                                    if *cursor >= *offset + max_vis || *cursor < *offset {
                                        *cursor = *offset;
                                    }
                                    redraw = true;
                                }
                                if keys & 0x00000010 != 0 {
                                    let max_off = n.saturating_sub(max_vis);
                                    *offset = offset.saturating_add(max_vis).min(max_off);
                                    if *cursor >= *offset + max_vis || *cursor < *offset {
                                        *cursor = *offset;
                                    }
                                    redraw = true;
                                }
                            }
                        }
                        Overlay::PerfStats(ref mut detail, ref mut cursor) => {
                            let n = gs.performance_snapshots.len();
                            if detail.is_some() {
                                if keys & 0x00000002 != 0 {
                                    *detail = None;
                                    redraw = true;
                                }
                            } else {
                                if keys & 0x00000040 != 0 && *cursor > 0 {
                                    *cursor -= 1;
                                    redraw = true;
                                }
                                if keys & 0x00000080 != 0 && *cursor + 1 < n {
                                    *cursor += 1;
                                    redraw = true;
                                }
                                if keys & 0x00000001 != 0 && n > 0 {
                                    *detail = Some(*cursor);
                                    redraw = true;
                                }
                            }
                        }
                        Overlay::RevealedCards(
                            ref mut show_self,
                            ref mut cursor,
                            ref mut view_card,
                        ) => {
                            let filter_owner: Option<u8> = if *show_self {
                                if is_host {
                                    Some(0)
                                } else {
                                    Some(1)
                                }
                            } else {
                                if is_host {
                                    Some(1)
                                } else {
                                    Some(0)
                                }
                            };
                            let mut owner_of: HashMap<i16, Option<u8>> = HashMap::new();
                            for (i, &cid) in gs.revealed_cards.iter().enumerate() {
                                if let Some(meta) = gs.revealed_card_meta.get(i) {
                                    owner_of.insert(cid, meta.owner);
                                }
                            }
                            for (i, &cid) in gs.revealed_cost_cards.iter().enumerate() {
                                if let Some(meta) = gs.revealed_cost_card_meta.get(i) {
                                    owner_of.insert(cid, meta.owner);
                                }
                            }
                            let filter_cards = |cards: &[i16]| -> Vec<i16> {
                                cards
                                    .iter()
                                    .filter(|&&cid| {
                                        if let Some(owner) = owner_of.get(&cid) {
                                            *owner == filter_owner || owner.is_none()
                                        } else {
                                            true
                                        }
                                    })
                                    .copied()
                                    .collect()
                            };
                            let mut flat: Vec<i16> = Vec::new();
                            flat.extend(filter_cards(&gs.initial_yell_revealed_cards));
                            flat.extend(filter_cards(&gs.re_yell_revealed_cards));
                            flat.extend(filter_cards(&gs.revealed_cost_cards));
                            flat.extend(filter_cards(&gs.revealed_cards));
                            if keys & 0x00000100 != 0 || keys & 0x00000200 != 0 {
                                *show_self = !*show_self;
                                *cursor = 0;
                                *view_card = None;
                                redraw = true;
                            } else {
                                let action = card_grid_input(keys, cursor, view_card, &flat, 5);
                                match action {
                                    GridAction::CloseGrid => {
                                        overlay = Overlay::None;
                                    }
                                    _ => {}
                                }
                                if !matches!(action, GridAction::None) {
                                    redraw = true;
                                }
                            }
                        }
                        Overlay::None => {}
                    }
                } else if detail_mode && viewing_card.is_some() {
                    // Detail mode with ability subview: L/B dismiss, Up/Down scrolls
                    if choice_subview {
                        if keys & 0x00000200 != 0 || keys & 0x00000002 != 0 {
                            choice_subview = false;
                            detail_scroll_y = 0.0;
                            redraw = true;
                        }
                        if keys & 0x00000040 != 0 && text_page > 0 {
                            text_page -= 1;
                            redraw = true;
                        }
                        if keys & 0x00000080 != 0 {
                            text_page += 1;
                            redraw = true;
                        }
                    } else {
                        // L opens full ability text overlay
                        if keys & 0x00000200 != 0 {
                            choice_subview = true;
                            text_page = 0;
                            redraw = true;
                        }
                        // Up/Down scrolls card detail
                        if keys & 0x00000040 != 0 {
                            detail_scroll_y -= 18.0;
                            if detail_scroll_y < 0.0 {
                                detail_scroll_y = 0.0;
                            }
                            redraw = true;
                        }
                        if keys & 0x00000080 != 0 {
                            detail_scroll_y += 18.0;
                            redraw = true;
                        }
                    }
                } else if detail_mode {
                    // Detail view without card: Up/Down scrolls
                    if keys & 0x00000040 != 0 {
                        detail_scroll_y -= 18.0;
                        if detail_scroll_y < 0.0 {
                            detail_scroll_y = 0.0;
                        }
                        redraw = true;
                    }
                    if keys & 0x00000080 != 0 {
                        detail_scroll_y += 18.0;
                        redraw = true;
                    }
                } else if !has_image_choice {
                    // Navigate in display space with wrap-around
                    // Skipped when choice grid handles its own navigation
                    // L opens full ability text overlay for text choices too
                    if keys & 0x00000200 != 0 && !choice_subview {
                        let has_ab = gs.ability_queue.current_entry().is_some();
                        if has_ab {
                            choice_subview = true;
                            text_page = 0;
                            redraw = true;
                        }
                    }
                    if choice_subview {
                        if keys & 0x00000200 != 0 || keys & 0x00000002 != 0 {
                            choice_subview = false;
                            detail_scroll_y = 0.0;
                            redraw = true;
                        }
                        if keys & 0x00000040 != 0 && text_page > 0 {
                            text_page -= 1;
                            redraw = true;
                        }
                        if keys & 0x00000080 != 0 {
                            text_page += 1;
                            redraw = true;
                        }
                    }
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
                if has_image_choice
                    && !detail_mode
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
                        // === Choices: L opens text overlay, DPAD navigates items ===
                        // Card items use grid navigation; text items use vertical list navigation.
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
                            let cols_c = 5usize;
                            let has_ability = gs.ability_queue.current_entry().is_some();
                            let pp = if has_ability { cols_c } else { cols_c * 2 };

                            // Detect current item type for navigation style
                            let cur_is_text = display_order
                                .get(display_pos)
                                .map_or(false, |&fi| is_text_only(&acts_cache[fi]));

                            if cur_is_text {
                                // Text item: navigate by 1
                                if keys & 0x00000040 != 0 && display_pos > 0 {
                                    display_pos -= 1;
                                }
                                if keys & 0x00000080 != 0 && display_pos + 1 < n {
                                    display_pos += 1;
                                }
                            } else {
                                // Card item: DOWN from last card jumps to first text item
                                let has_text = display_order
                                    .iter()
                                    .any(|&fi| is_text_only(&acts_cache[fi]));
                                if keys & 0x00000080 != 0 {
                                    let next = (display_pos + cols_c).min(n - 1);
                                    let next_is_text = display_order
                                        .get(next)
                                        .map_or(false, |&fi| is_text_only(&acts_cache[fi]));
                                    if has_text && next_is_text {
                                        display_pos = next;
                                    } else if has_text {
                                        let last_card = display_order
                                            .iter()
                                            .rposition(|&fi| !is_text_only(&acts_cache[fi]))
                                            .unwrap_or(0);
                                        display_pos = (display_pos + cols_c).min(last_card);
                                    } else {
                                        display_pos = next;
                                    }
                                }
                                if keys & 0x00000040 != 0 {
                                    display_pos = display_pos.saturating_sub(cols_c);
                                }
                            }
                            // LEFT/RIGHT always by 1
                            if keys & 0x00000020 != 0 && display_pos > 0 {
                                display_pos -= 1;
                            }
                            if keys & 0x00000010 != 0 && display_pos + 1 < n {
                                display_pos += 1;
                            }

                            choice_grid_offset = (display_pos / pp) * pp;
                            cur = display_order[display_pos];
                            redraw = true;
                        }
                    }
                }

                // B: close menus / overlays / card detail
                if keys & 0x00000002 != 0 {
                    if viewing_card.is_some() {
                        viewing_card = None;
                        detail_mode = false;
                        detail_scroll_y = 0.0;
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
                    let is_my_turn = gs.active_player().id == pref(&gs, my_player_idx).id;
                    let (off, max) = if is_my_turn {
                        (
                            hand_offset,
                            pref(&gs, my_player_idx)
                                .hand
                                .cards
                                .len()
                                .saturating_sub(vis),
                        )
                    } else {
                        (
                            hand_offset_p2,
                            pref(&gs, 1 - my_player_idx)
                                .hand
                                .cards
                                .len()
                                .saturating_sub(vis),
                        )
                    };
                    if keys & 0x00000020 != 0 && off > 0 {
                        if is_my_turn {
                            hand_offset -= 1;
                        } else {
                            hand_offset_p2 -= 1;
                        }
                        redraw = true;
                    }
                    if keys & 0x00000010 != 0 && off + vis < max + vis {
                        if is_my_turn {
                            hand_offset += 1;
                        } else {
                            hand_offset_p2 += 1;
                        }
                        redraw = true;
                    }
                }

                // X toggles card detail mode + narrows action list to selected card
                if overlay == Overlay::None && keys & 0x00000400 != 0 {
                    let has_card = cur < acts_cache.len()
                        && acts_cache[cur]
                            .parameters
                            .as_ref()
                            .and_then(|p| p.card_id)
                            .is_some();
                    if !has_card && !detail_mode {
                        // No card on this action — do nothing
                    } else {
                        detail_mode = !detail_mode;
                        detail_scroll_y = 0.0;
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
                    }
                    redraw = true;
                }

                // Zone viewer controls
                if zone_viewer.is_some() {
                    let cards = zone_viewer.as_ref().map_or(&[][..], |z| &z.1);
                    let action =
                        card_grid_input(keys, &mut zone_viewer_offset, &mut viewing_card, cards, 5);
                    match action {
                        GridAction::CloseGrid => {
                            zone_viewer = None;
                        }
                        _ => {}
                    }
                    if !matches!(action, GridAction::None) {
                        redraw = true;
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
                // Track the pending state seq before this frame mutates it, so the
                // host retransmit logic can detect "just staged this frame" and send
                // immediately instead of waiting for the next throttle tick.
                let pending_seq_before = pending_state.as_ref().map(|(s, _, _)| *s);
                if is_multiplayer {
                    // udsPullPacket returns one packet per call, so drain up to 48
                    // packets per frame. Without this a multi-chunk state transfer
                    // would be stretched across N frames (~N/60 s) before reassembly.
                    let mut recv_buf = [0u8; 1024];
                    for _drain in 0..48 {
                        if let Ok(n) = uds::uds_recv(&mut recv_buf) {
                            if n > 0 {
                                dbg_rx_bytes += n as u32;
                                if recv_buf[0] == uds::MSG_SYNC_STATE_ACK && is_host {
                                    // Client ACKed a state transfer. The bitmap tells us
                                    // which chunks got through — mark them acked so we only
                                    // retransmit the ones UDS actually dropped.
                                    if n >= 4 {
                                        let ack_seq =
                                            (recv_buf[1] as u16) | ((recv_buf[2] as u16) << 8);
                                        let blen = (recv_buf[3] as usize).min(n - 4);
                                        if let Some((seq, _, acked)) = &mut pending_state {
                                            if *seq == ack_seq {
                                                for (bi, byte) in
                                                    recv_buf[4..4 + blen].iter().enumerate()
                                                {
                                                    for bit in 0..8 {
                                                        if byte & (1 << bit) != 0 {
                                                            let idx = bi * 8 + bit;
                                                            if idx < acked.len() {
                                                                acked[idx] = true;
                                                            }
                                                        }
                                                    }
                                                }
                                                if acked.iter().all(|&b| b) {
                                                    pending_state = None;
                                                }
                                            }
                                        }
                                    }
                                } else if recv_buf[0] == uds::MSG_SYNC_STATE && !is_host {
                                    // Client: reassemble authoritative GameState from host.
                                    // Capture the completion ACK AFTER feed() marks the final
                                    // chunk so the bitmap is complete.
                                    if state_rx.feed(&recv_buf[..n]) {
                                        let final_ack = state_rx.partial_ack();
                                        if let Some(bytes) = state_rx.take() {
                                            match rmp_serde::from_slice::<GameState>(&bytes) {
                                                Ok(mut new_gs) => {
                                                    // Keep our own identical card database (skipped on wire)
                                                    new_gs.card_database = gs.card_database.clone();
                                                    gs = new_gs;
                                                    let my_id = if is_host { 0 } else { 1 };
                                                    waiting_for_opponent = !mp_can_act(&gs, my_id);
                                                    cur = 0;
                                                    dirty = true;
                                                    redraw = true;
                                                    // The host has processed our action — stop retransmitting it.
                                                    pending_client_action = None;
                                                    // ACK so the host stops retransmitting
                                                    if let Some(ack) = final_ack {
                                                        let _ = uds::uds_send(&ack);
                                                        dbg_tx_bytes += ack.len() as u32;
                                                    }
                                                }
                                                Err(e) => unsafe {
                                                    _3ds_debug_print(
                                                        format!(
                                                            "[STATE] deserialize err: {}\n\0",
                                                            e
                                                        )
                                                        .as_ptr(),
                                                    );
                                                },
                                            }
                                        }
                                    } else if state_rx.wants_reack(
                                        (recv_buf[1] as u16) | ((recv_buf[2] as u16) << 8),
                                    ) {
                                        // A retransmitted chunk of an already-completed state:
                                        // re-send the full ACK so the host stops (heals a dropped
                                        // final ACK) without re-adopting the stale state.
                                        if let Some(ack) = state_rx.completed_ack() {
                                            let _ = uds::uds_send(&ack);
                                            dbg_tx_bytes += ack.len() as u32;
                                        }
                                    }
                                } else if is_host {
                                    // Host: execute the client's action authoritatively, then ship state
                                    if let Some(sync) = uds::ActionSync::from_bytes(&recv_buf[..n])
                                    {
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
                                        // Dedup retransmitted actions: only execute once per seq.
                                        // A duplicate still gets a fresh state reply (the client
                                        // retransmits to recover a dropped state, not a new action).
                                        let is_dup = sync.action_seq != 0
                                            && sync.action_seq <= last_client_action_seq;
                                        if is_dup {
                                            let prev = pending_state
                                                .as_ref()
                                                .map(|(s, _, _)| *s)
                                                .unwrap_or(0);
                                            let staged = stage_authoritative_state(&gs, prev);
                                            pending_state = Some(staged);
                                        } else {
                                            if sync.action_seq != 0 {
                                                last_client_action_seq = sync.action_seq;
                                            }
                                            // RPS choices from the client are P2's (player 1).
                                            // Must be cleared by the action execution.
                                            if matches!(
                                                action_type,
                                                game_setup::ActionType::RockChoice
                                                    | game_setup::ActionType::PaperChoice
                                                    | game_setup::ActionType::ScissorsChoice
                                            ) {
                                                gs.pending_rps_player_id = Some(1);
                                            }
                                            let _ = turn::TurnEngine::execute_main_phase_action_with_ability_index(
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
                                            game_setup::settle_single_player_state(&mut gs);
                                            let my_id = if is_host { 0 } else { 1 };
                                            waiting_for_opponent = !mp_can_act(&gs, my_id);
                                            cur = 0;
                                            dirty = true;
                                            redraw = true;
                                            // Stage authoritative state for reliable delivery
                                            let prev = pending_state
                                                .as_ref()
                                                .map(|(s, _, _)| *s)
                                                .unwrap_or(0);
                                            let staged = stage_authoritative_state(&gs, prev);
                                            pending_state = Some(staged);
                                        }
                                    }
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                // Client: report partial progress on the in-progress state transfer so
                // the host prunes already-received chunks instead of resending them.
                if is_multiplayer && !is_host {
                    if let Some(ack) = state_rx.partial_ack() {
                        let _ = uds::uds_send(&ack);
                        dbg_tx_bytes += ack.len() as u32;
                    }
                }
                // Client: retransmit its last action until the host's new state arrives.
                if is_multiplayer && !is_host {
                    if let Some(bytes) = &pending_client_action {
                        let _ = uds::uds_send(bytes);
                        dbg_tx_bytes += bytes.len() as u32;
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
                        // Authoritative model: host/single executes, client sends action.
                        let executed = route_authoritative_action(
                            &mut gs,
                            &action,
                            is_multiplayer,
                            is_host,
                            &mut waiting_for_opponent,
                            &mut pending_state,
                            &mut pending_client_action,
                            &mut next_action_seq,
                        );
                        // VS AI RPS: after human picks P1, AI auto-picks P2 (only when authority)
                        if executed
                            && *vs_ai
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
                        match turn::TurnEngine::execute_main_phase_action(
                            &mut gs,
                            &action.action_type,
                            p.as_ref().and_then(|x| x.card_id),
                            p.as_ref().and_then(|x| x.card_indices.clone()),
                            p.as_ref()
                                .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
                            p.as_ref().and_then(|x| x.use_baton_touch),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                dprintln!("[AI] action failed: {}", e);
                            }
                        }
                        gs.reset_loop_detection();
                    }
                    acts_cache.clear();
                    cur = 0;
                    dirty = true;
                    redraw = true;
                }

                // Authoritative model: only the HOST settles automatic phases.
                // The client never runs the engine — it adopts the host's shipped state.
                let is_authority_here = !is_multiplayer || is_host;
                let auto = is_authority_here
                    && !gs.has_pending_choice()
                    && gs.game_result == GameResult::Ongoing
                    && game_setup::is_automatic_phase(&gs);
                if auto {
                    game_setup::settle_single_player_state(&mut gs);
                    if is_multiplayer {
                        let my_id = if is_host { 0 } else { 1 };
                        waiting_for_opponent = !mp_can_act(&gs, my_id);
                        if is_host {
                            let prev = pending_state.as_ref().map(|(s, _, _)| *s).unwrap_or(0);
                            let staged = stage_authoritative_state(&gs, prev);
                            pending_state = Some(staged);
                        }
                    }
                    cur = 0;
                    dirty = true;
                }

                // Host: retransmit pending state chunks until the client ACKs.
                // UDS is unreliable — a dropped chunk would otherwise deadlock the
                // client forever. Only unacked chunks are sent (the client's bitmap
                // ACK prunes the ones that got through), immediately when a new state
                // is staged this frame, otherwise throttled to every other frame.
                if is_multiplayer && is_host {
                    // Ensure the client has the host's authoritative state from the start.
                    if !state_init {
                        let staged = stage_authoritative_state(&gs, 0);
                        pending_state = Some(staged);
                        state_init = true;
                    }
                    let just_staged =
                        pending_state.as_ref().map(|(s, _, _)| *s) != pending_seq_before;
                    if just_staged || (_frame % 2 == 0) {
                        if let Some((_, chunks, acked)) = &pending_state {
                            for (i, chunk) in chunks.iter().enumerate() {
                                if !acked[i] {
                                    let _ = uds::uds_send(chunk);
                                    dbg_tx_bytes += chunk.len() as u32;
                                }
                            }
                        }
                    }
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
                            let pb = if y0 == p1y0 {
                                pref(&gs, my_player_idx)
                            } else {
                                pref(&gs, 1 - my_player_idx)
                            };
                            let ap_id = &gs.active_player().id;
                            let is_ap_me = *ap_id == pref(&gs, my_player_idx).id;
                            tap_active_side = if is_ap_me { y0 == p1y0 } else { y0 != p1y0 };
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
                                // Live set cards are face-down until the performance phase —
                                // not clickable until then (you can't see which card is there).
                                let live_clickable = matches!(
                                    gs.current_phase,
                                    Phase::FirstAttackerPerformance
                                        | Phase::SecondAttackerPerformance
                                        | Phase::LiveVictoryDetermination
                                );
                                if live_clickable {
                                    let idx = ((tx as f32 - 5.0) / (live_slot_w + 2.0)) as usize;
                                    if idx < 3 && idx < pb.live_card_zone.cards.len() {
                                        Some(pb.live_card_zone.cards[idx])
                                    } else {
                                        None
                                    }
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
                                        let _ = route_authoritative_action(
                                            &mut gs,
                                            &action,
                                            is_multiplayer,
                                            is_host,
                                            &mut waiting_for_opponent,
                                            &mut pending_state,
                                            &mut pending_client_action,
                                            &mut next_action_seq,
                                        );
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
                                        let _ = route_authoritative_action(
                                            &mut gs,
                                            &action,
                                            is_multiplayer,
                                            is_host,
                                            &mut waiting_for_opponent,
                                            &mut pending_state,
                                            &mut pending_client_action,
                                            &mut next_action_seq,
                                        );
                                        cur = 0;
                                        dirty = true;
                                        redraw = true;
                                        break;
                                    }
                                }
                            // Choice image mode: board tap executes the choice directly
                            } else if has_image_choice {
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
                                    let _ = route_authoritative_action(
                                        &mut gs,
                                        &action,
                                        is_multiplayer,
                                        is_host,
                                        &mut waiting_for_opponent,
                                        &mut pending_state,
                                        &mut pending_client_action,
                                        &mut next_action_seq,
                                    );
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
                                        detail_scroll_y = 0.0;
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
                            // Default: toggle card detail view for any tapped card
                            // Skip if a stage zone was tapped — stage handler takes priority
                            } else if stage_tap.is_none() {
                                if Some(cid) == viewing_card {
                                    viewing_card = None;
                                    detail_mode = false;
                                } else {
                                    viewing_card = Some(cid);
                                    detail_mode = true;
                                    detail_scroll_y = 0.0;
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
                                let player = if y0 == p1y0 {
                                    pref(&gs, my_player_idx)
                                } else {
                                    pref(&gs, 1 - my_player_idx)
                                };
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
                                    let _ = route_authoritative_action(
                                        &mut gs,
                                        &act2,
                                        is_multiplayer,
                                        is_host,
                                        &mut waiting_for_opponent,
                                        &mut pending_state,
                                        &mut pending_client_action,
                                        &mut next_action_seq,
                                    );
                                    detail_mode = false;
                                    viewing_card = None;
                                    cur = 0;
                                    dirty = true;
                                    redraw = true;
                                    stage_handled = true;
                                    break;
                                }
                            }
                            // ChoicePosition: select stage position during choice prompt
                            if !stage_handled && has_image_choice {
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
                                    let _ = route_authoritative_action(
                                        &mut gs,
                                        &act2,
                                        is_multiplayer,
                                        is_host,
                                        &mut waiting_for_opponent,
                                        &mut pending_state,
                                        &mut pending_client_action,
                                        &mut next_action_seq,
                                    );
                                    viewing_card = None;
                                    cur = 0;
                                    dirty = true;
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
                    if dirty {
                        acts_cache = game_setup::generate_possible_actions(&gs);
                        choice_grid_offset = 0;
                        list_scroll = 0;
                    }

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

                    // Debug: dump acts_cache when there's a pending choice
                    if dirty && gs.has_pending_choice() {
                        dprintln!(
                            "[CHOICE] acts_cache={} display_order={} choice_img={}",
                            acts_cache.len(),
                            display_order.len(),
                            choice_image_mode
                        );
                        for (i, act) in acts_cache.iter().enumerate() {
                            let cid = act
                                .parameters
                                .as_ref()
                                .and_then(|p| p.card_id)
                                .unwrap_or(-1);
                            let cn = act
                                .parameters
                                .as_ref()
                                .and_then(|p| p.card_no.clone())
                                .unwrap_or_default();
                            dprintln!("  [{}] {:?} cid={} cn={}", i, act.action_type, cid, cn);
                        }
                    }

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
                            let cn = gs
                                .card_database
                                .get_card(cid)
                                .map(|c| c.card_no.to_string());
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
                            // Live set cards are face-down (card back) until the live
                            // performance phase. You can see a card is there, not which.
                            let live_hidden = !matches!(
                                gs.current_phase,
                                Phase::FirstAttackerPerformance
                                    | Phase::SecondAttackerPerformance
                                    | Phase::LiveVictoryDetermination
                            );
                            for i in 0..3.min(lc.len()) {
                                let cid = lc[i];
                                if cid == -1 {
                                    unsafe {
                                        $live_fn(
                                            i as i32,
                                            false,
                                            std::ptr::null(),
                                            0,
                                            false,
                                            false,
                                        );
                                    }
                                    continue;
                                }
                                if live_hidden {
                                    // Show the card back so presence is visible but not identity.
                                    // Pass tapped=true so the C renderer rotates it 90° to fill
                                    // the landscape live slot.
                                    let back = std::ffi::CString::new("icon_lltcg-back.png.t3x")
                                        .unwrap_or_default();
                                    unsafe {
                                        $live_fn(
                                            i as i32,
                                            true,
                                            back.as_ptr() as *const u8,
                                            0,
                                            true,
                                            true,
                                        );
                                    }
                                } else {
                                    let tapped = if cid != -1 { is_tapped(cid) } else { false };
                                    set_slot($live_fn, i as i32, cid, true, tapped);
                                }
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
                        pref(&gs, my_player_idx),
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
                            pref(&gs, 1 - my_player_idx),
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
                            pref(&gs, 1 - my_player_idx),
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
                        // Hide opponent's live cards until they perform
                        let opp_is_first = pref(&gs, 1 - my_player_idx).is_first_attacker;
                        let opp_performed =
                            matches!(
                                gs.current_phase,
                                Phase::SecondAttackerPerformance | Phase::LiveVictoryDetermination
                            ) || (matches!(gs.current_phase, Phase::FirstAttackerPerformance)
                                && opp_is_first);
                        if !opp_performed {
                            unsafe {
                                for i in 0..3i32 {
                                    _3ds_board_set_opp_live(
                                        i,
                                        false,
                                        std::ptr::null(),
                                        0,
                                        false,
                                        false,
                                    );
                                }
                            }
                        }
                    }

                    // Set per-card live stats on the C board
                    {
                        let set_live_stats = |player: &Player, gs: &GameState, is_opp: bool| {
                            for (i, &cid) in player.live_card_zone.cards.iter().enumerate().take(3)
                            {
                                if cid == -1 || cid == 0 {
                                    continue;
                                }
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    let stats = compute_card_stats(card, cid, gs);
                                    // Opponent: hide score and need hearts
                                    let stat_line = if is_opp {
                                        String::new()
                                    } else {
                                        card_stat_line(
                                            stats.total_blade,
                                            &stats.heart_str,
                                            stats.score,
                                            stats.cost.into(),
                                            stats.is_tapped,
                                            card.card_type.as_card_str(),
                                            &stats.need_heart_str,
                                        )
                                    };
                                    let c_line = std::ffi::CString::new(stat_line.as_bytes())
                                        .unwrap_or_default();
                                    unsafe {
                                        if is_opp {
                                            _3ds_board_set_opp_live_stats(
                                                i as i32,
                                                stats.score,
                                                c_line.as_ptr() as *const u8,
                                            );
                                        } else {
                                            _3ds_board_set_live_stats(
                                                i as i32,
                                                stats.score,
                                                c_line.as_ptr() as *const u8,
                                            );
                                        }
                                    }
                                }
                            }
                        };
                        set_live_stats(pref(&gs, my_player_idx), &gs, false);
                        set_live_stats(pref(&gs, 1 - my_player_idx), &gs, true);
                    }

                    // Compute and set need hearts text for bottom screen live zone
                    {
                        let compute_live_need = |player: &Player, gs: &GameState| -> Vec<u32> {
                            let mut nh = vec![0u32; 8];
                            for &cid in &player.live_card_zone.cards {
                                if cid == -1 {
                                    continue;
                                }
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    if let Some(ref need) = card.need_heart {
                                        for (color, count) in &need.hearts {
                                            if let Some(idx) = heart_color_index(color) {
                                                nh[idx] += *count as u32;
                                            }
                                        }
                                    }
                                }
                            }
                            for (&cid, colors) in &gs.mods.need_heart_modifiers {
                                if player.live_card_zone.cards.contains(&cid) {
                                    for (color, &val) in colors {
                                        if let Some(idx) = heart_color_index(color) {
                                            nh[idx] = (nh[idx] as i32 + val.total()).max(0) as u32;
                                        }
                                    }
                                }
                            }
                            nh
                        };
                        // P1 (perspective player) need hearts — always show if any
                        let p1_nh = compute_live_need(&gs.player1, &gs);
                        unsafe {
                            _3ds_set_need_hearts(
                                0, p1_nh[0], p1_nh[1], p1_nh[2], p1_nh[3], p1_nh[4], p1_nh[5],
                                p1_nh[6], p1_nh[7],
                            );
                        }
                        // P2 (opponent) need hearts — hidden until performed
                        let opp_is_first = gs.player2.is_first_attacker;
                        let opp_performed =
                            matches!(
                                gs.current_phase,
                                Phase::SecondAttackerPerformance | Phase::LiveVictoryDetermination
                            ) || (matches!(gs.current_phase, Phase::FirstAttackerPerformance)
                                && opp_is_first);
                        if opp_performed {
                            let p2_nh = compute_live_need(&gs.player2, &gs);
                            unsafe {
                                _3ds_set_need_hearts(
                                    1, p2_nh[0], p2_nh[1], p2_nh[2], p2_nh[3], p2_nh[4], p2_nh[5],
                                    p2_nh[6], p2_nh[7],
                                );
                            }
                        } else {
                            unsafe {
                                _3ds_set_need_hearts(1, 0, 0, 0, 0, 0, 0, 0, 0);
                            }
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
                            let ap_label = if ap.id == pref(&gs, my_player_idx).id {
                                "P1"
                            } else {
                                "P2"
                            };
                            let touch_indicator =
                                if viewing_card.is_some() { "[T]" } else { "   " };
                            unsafe {
                                let phase_name;
                                _3ds_text_add_top(
                                    {
                                        phase_name = if current_lang() == Lang::Japanese {
                                            gs.current_phase.label_jp().to_string()
                                        } else {
                                            format!("{}", gs.current_phase)
                                        };
                                        format!(
                                            "{} {} | {} | {}{} | taps:{}\n\0",
                                            tl("Turn").trim_end_matches(':'),
                                            gs.turn_number,
                                            phase_name,
                                            ap_label,
                                            touch_indicator,
                                            touch_tap_count,
                                        )
                                    }
                                    .as_ptr(),
                                );
                                _3ds_text_add_top(format!("Me H:{} E:{}/{} D:{} W:{} L:{}  Opp H:{} E:{}/{} D:{} W:{} L:{}\n\0",
                                    pref(&gs, my_player_idx).hand.cards.len(), pref(&gs, my_player_idx).energy_zone.active_count(), pref(&gs, my_player_idx).energy_zone.cards.len(),
                                    pref(&gs, my_player_idx).main_deck.cards.len(), pref(&gs, my_player_idx).waitroom.cards.len(), pref(&gs, my_player_idx).success_live_card_zone.cards.len(),
                                    pref(&gs, 1 - my_player_idx).hand.cards.len(), pref(&gs, 1 - my_player_idx).energy_zone.active_count(), pref(&gs, 1 - my_player_idx).energy_zone.cards.len(),
                                    pref(&gs, 1 - my_player_idx).main_deck.cards.len(), pref(&gs, 1 - my_player_idx).waitroom.cards.len(), pref(&gs, 1 - my_player_idx).success_live_card_zone.cards.len(),
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
                            let is_ai_turn = *ai_vs_ai || (*vs_ai && !mp_can_act(&gs, 0));
                            let is_opponent_turn_mp = is_multiplayer
                                && !mp_can_act(
                                    &gs,
                                    if is_multiplayer {
                                        if is_host {
                                            0
                                        } else {
                                            1
                                        }
                                    } else {
                                        0
                                    },
                                );
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
                                                    "[{}] E{} {} {}→{}",
                                                    cn,
                                                    cost,
                                                    name,
                                                    src_labels.join("+"),
                                                    area_label
                                                )
                                            } else {
                                                format!(
                                                    "[{}] E{} {} {}",
                                                    cn, cost, name, area_label
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
                        // Top screen: stats bar (0-50px) + content panel (52-240px).
                        // Clear the top screen so old menu content doesn't overlap
                        unsafe {
                            _3ds_top_clear();
                        }
                        unsafe {
                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 50.0, COL_PANEL);
                            let phase_name = if current_lang() == Lang::Japanese {
                                gs.current_phase.label_jp().to_string()
                            } else {
                                format!("{}", gs.current_phase)
                            };
                            _3ds_top_queue_text(
                                4.0,
                                2.0,
                                COL_GOLD,
                                0.65f32,
                                format!(
                                    "T{} {} [{}]  Me H:{} E:{}/{} D:{}  Opp H:{} E:{}/{} D:{}\0",
                                    gs.turn_number,
                                    phase_name,
                                    if ap.id == pref(&gs, my_player_idx).id {
                                        "Me"
                                    } else {
                                        "Opp"
                                    },
                                    pref(&gs, my_player_idx).hand.cards.len(),
                                    pref(&gs, my_player_idx).energy_zone.active_count(),
                                    pref(&gs, my_player_idx).energy_zone.cards.len(),
                                    pref(&gs, my_player_idx).main_deck.cards.len(),
                                    pref(&gs, 1 - my_player_idx).hand.cards.len(),
                                    pref(&gs, 1 - my_player_idx).energy_zone.active_count(),
                                    pref(&gs, 1 - my_player_idx).energy_zone.cards.len(),
                                    pref(&gs, 1 - my_player_idx).main_deck.cards.len(),
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
                                        card.blade as u32
                                    };
                                    Some(total)
                                })
                                .sum::<u32>();
                            let p2_blade: u32 = gs
                                .player2
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
                                        card.blade as u32
                                    };
                                    Some(total)
                                })
                                .sum::<u32>();
                            // Compute total hearts per player from stage members
                            // (mirrors display.rs player_to_display total_hearts logic)
                            let mut p1_hearts = vec![0u32; 8];
                            let mut p2_hearts = vec![0u32; 8];
                            for &cid in &gs.player1.stage.stage {
                                if cid == -1 {
                                    continue;
                                }
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    if let Some(ref base_heart) = card.base_heart {
                                        let h_mult =
                                            gs.mods.heart_color_multiplier.get(&cid).copied();
                                        for (color, count) in &base_heart.hearts {
                                            if let Some(idx) = heart_color_index(color) {
                                                if let Some(hc) = h_mult {
                                                    if hc == *color {
                                                        p1_hearts[idx] += *count as u32;
                                                    }
                                                } else {
                                                    p1_hearts[idx] += *count as u32;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            for (cid, modifier) in &gs.mods.heart_modifiers {
                                if !gs.player1.stage.stage.contains(cid) {
                                    continue;
                                }
                                for (color, val) in modifier {
                                    if let Some(idx) = heart_color_index(color) {
                                        p1_hearts[idx] =
                                            (p1_hearts[idx] as i32 + val.total()).max(0) as u32;
                                    }
                                }
                            }
                            for &cid in &gs.player2.stage.stage {
                                if cid == -1 {
                                    continue;
                                }
                                if let Some(card) = gs.card_database.get_card(cid) {
                                    if let Some(ref base_heart) = card.base_heart {
                                        let h_mult =
                                            gs.mods.heart_color_multiplier.get(&cid).copied();
                                        for (color, count) in &base_heart.hearts {
                                            if let Some(idx) = heart_color_index(color) {
                                                if let Some(hc) = h_mult {
                                                    if hc == *color {
                                                        p2_hearts[idx] += *count as u32;
                                                    }
                                                } else {
                                                    p2_hearts[idx] += *count as u32;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            for (cid, modifier) in &gs.mods.heart_modifiers {
                                if !gs.player2.stage.stage.contains(cid) {
                                    continue;
                                }
                                for (color, val) in modifier {
                                    if let Some(idx) = heart_color_index(color) {
                                        p2_hearts[idx] =
                                            (p2_hearts[idx] as i32 + val.total()).max(0) as u32;
                                    }
                                }
                            }
                            // Format hearts as texticon string
                            let format_hearts = |hearts: &[u32]| -> String {
                                let mut parts = Vec::new();
                                for (i, &count) in hearts.iter().enumerate() {
                                    if count > 0 {
                                        let label = format!("h{:02}{}", i, count);
                                        parts.push(heart_label_to_icon(&label));
                                    }
                                }
                                if parts.is_empty() {
                                    return String::new();
                                }
                                parts.join(" ")
                            };
                            let p1_heart_str = format_hearts(&p1_hearts);
                            let p2_heart_str = format_hearts(&p2_hearts);
                            // Render P1 hearts+blades on top screen line 2
                            let p1_stats = if p1_heart_str.is_empty() {
                                format!("BL:{}", p1_blade)
                            } else {
                                format!(
                                    "{}  {{{{icon_blade.png|BLADE}}}}{}",
                                    p1_heart_str, p1_blade
                                )
                            };
                            render_text_with_icons(4.0, 22.0, &p1_stats, COL_LIGHT, 0.55f32);
                            // Render P2 hearts+blades on top screen line 3
                            let p2_stats = if p2_heart_str.is_empty() {
                                format!("BL:{}", p2_blade)
                            } else {
                                format!(
                                    "{}  {{{{icon_blade.png|BLADE}}}}{}",
                                    p2_heart_str, p2_blade
                                )
                            };
                            render_text_with_icons(4.0, 34.0, &p2_stats, COL_LIGHT, 0.55f32);
                            // Show need hearts during live set phase
                            // Rule 8.2.x: opponent's need hearts are hidden
                            // until their cards are revealed (performed).
                            let is_live_set = matches!(
                                gs.current_phase,
                                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
                            );
                            if is_live_set {
                                // Compute live_need_hearts from live zone cards
                                let compute_live_need =
                                    |player: &Player, gs: &GameState| -> Vec<u32> {
                                        let mut nh = vec![0u32; 8];
                                        for &cid in &player.live_card_zone.cards {
                                            if cid == -1 {
                                                continue;
                                            }
                                            if let Some(card) = gs.card_database.get_card(cid) {
                                                if let Some(ref need) = card.need_heart {
                                                    for (color, count) in &need.hearts {
                                                        if let Some(idx) = heart_color_index(color)
                                                        {
                                                            nh[idx] += *count as u32;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        for (&cid, colors) in &gs.mods.need_heart_modifiers {
                                            if player.live_card_zone.cards.contains(&cid) {
                                                for (color, &val) in colors {
                                                    if let Some(idx) = heart_color_index(color) {
                                                        nh[idx] = (nh[idx] as i32 + val.total())
                                                            .max(0)
                                                            as u32;
                                                    }
                                                }
                                            }
                                        }
                                        nh
                                    };
                                let opp_is_first = gs.player2.is_first_attacker;
                                let opp_performed = matches!(
                                    gs.current_phase,
                                    Phase::SecondAttackerPerformance
                                        | Phase::LiveVictoryDetermination
                                ) || (matches!(
                                    gs.current_phase,
                                    Phase::FirstAttackerPerformance
                                ) && opp_is_first);
                                // P1 (perspective) need hearts
                                let p1_nh = compute_live_need(&gs.player1, &gs);
                                if p1_nh.iter().any(|&v| v > 0) {
                                    let nh_str = format_hearts(&p1_nh);
                                    let need_display =
                                        format!("{{{{icon_heart_06.png|NEED}}}} {}", nh_str);
                                    render_text_with_icons(
                                        4.0,
                                        46.0,
                                        &need_display,
                                        COL_GOLD,
                                        0.50f32,
                                    );
                                }
                                // P2 (opponent) need hearts — only after performed
                                if opp_performed {
                                    let p2_nh = compute_live_need(&gs.player2, &gs);
                                    if p2_nh.iter().any(|&v| v > 0) {
                                        let nh_str = format_hearts(&p2_nh);
                                        let need_display =
                                            format!("{{{{icon_heart_06.png|NEED}}}} {}", nh_str);
                                        render_text_with_icons(
                                            4.0,
                                            46.0,
                                            &need_display,
                                            COL_GOLD,
                                            0.50f32,
                                        );
                                    }
                                }
                            }
                        }

                        // Content panel — rendering stack (bottom to top):
                        //   1. zone_viewer       — zone card grid (own/opponent stage)
                        //   2. detail_mode        — full-screen card detail overlay
                        //   3. ability_queue      — compact ability banner (CLI/text only)
                        //   4. choice_image_mode  — ability banner + card choice grid
                        //   5. action list        — text action list (bottom text area)

                        let mut content_y: f32 = 52.0;

                        if let Some((ref zlabel, ref zcards)) = zone_viewer {
                            if viewing_card.is_none() {
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                    _3ds_top_queue_text(
                                        4.0,
                                        4.0,
                                        COL_GOLD,
                                        0.65f32,
                                        format!("{}  (B=close, X=detail)\0", zlabel).as_ptr(),
                                    );
                                }
                                render_card_grid(
                                    zcards,
                                    zone_viewer_offset,
                                    5,
                                    2,
                                    28.0,
                                    &gs.card_database,
                                    atlas,
                                );
                            } else {
                                render_card_detail(
                                    viewing_card.unwrap(),
                                    &gs.card_database,
                                    detail_scroll_y,
                                );
                            }
                        } else if detail_mode {
                            // L pressed: show full ability text overlay
                            if choice_subview {
                                if let Some(cid) = viewing_card {
                                    if let Some(card) = gs.card_database.get_card(cid) {
                                        unsafe {
                                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                            _3ds_top_queue_text(
                                                4.0,
                                                4.0,
                                                COL_GOLD,
                                                0.70f32,
                                                format!("{}\0", tl("Ability")).as_ptr(),
                                            );
                                        }
                                        let mut all_lines: Vec<String> = Vec::new();
                                        let abs: Vec<_> = card.resolved_abilities().collect();
                                        if abs.is_empty() {
                                            let raw = card.ability_text();
                                            if !raw.is_empty() {
                                                let clean = raw.replace('\n', " ");
                                                let w = wrap_ability_text(&clean, 384.0, 0.65);
                                                for l in w.lines() {
                                                    all_lines.push(l.to_string());
                                                }
                                            }
                                        } else {
                                            for ab in &abs {
                                                let ab_text = i18n::translate_ability(
                                                    &ab.full_text,
                                                    current_lang(),
                                                );
                                                let w = wrap_ability_text(&ab_text, 384.0, 0.65);
                                                for l in w.lines() {
                                                    all_lines.push(l.to_string());
                                                }
                                                all_lines.push(String::new());
                                            }
                                        }
                                        let lpp = 10usize;
                                        let total_pages =
                                            ((all_lines.len() + lpp - 1) / lpp).max(1);
                                        text_page = text_page.min(total_pages - 1);
                                        let start = text_page * lpp;
                                        let mut ty = 24.0;
                                        for line in &all_lines[start..] {
                                            if ty > 220.0 {
                                                break;
                                            }
                                            render_text_with_icons(4.0, ty, line, COL_LIGHT, 0.65);
                                            ty += 18.0;
                                        }
                                        if total_pages > 1 {
                                            unsafe {
                                                _3ds_top_queue_text(
                                                    370.0,
                                                    4.0,
                                                    COL_MED,
                                                    0.50f32,
                                                    format!("{}/{}\0", text_page + 1, total_pages)
                                                        .as_ptr(),
                                                );
                                            }
                                        }
                                        render_hint_bar(&tl("L/B=close  Up/Down=scroll"));
                                    }
                                }
                            } else {
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
                                            let ab_text = i18n::translate_ability(
                                                &ab.full_text,
                                                current_lang(),
                                            );
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
                                        let rect_h = panel_end - 52.0;

                                        unsafe {
                                            // Background for scrollable area
                                            _3ds_top_queue_rect(0.0, 52.0, 400.0, 188.0, COL_CARD);
                                            // Scrollable ability text
                                            let mut ty = 86.0 - detail_scroll_y;
                                            for ab in card.resolved_abilities() {
                                                let ab_text = i18n::translate_ability(
                                                    &ab.full_text,
                                                    current_lang(),
                                                );
                                                let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                                                for line in w.lines() {
                                                    if ty > -20.0 && ty < 240.0 {
                                                        render_text_with_icons(
                                                            4.0, ty, line, COL_LIGHT, 0.65,
                                                        );
                                                    }
                                                    ty += 18.0;
                                                }
                                                ty += 3.0;
                                            }
                                            ability_end = ty;
                                            // Header overlay on top: covers name + stats, clips scrolling text
                                            _3ds_top_queue_rect(0.0, 0.0, 400.0, 86.0, COL_TOP_BG);
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
                                                    stats.cost.into(),
                                                    stats.is_tapped,
                                                    card.card_type.as_card_str(),
                                                    &stats.need_heart_str,
                                                ),
                                                COL_LIGHT,
                                                0.65f32,
                                            );
                                        }
                                    }
                                }
                                content_y = if ability_end > 0.0 {
                                    ability_end + 6.0
                                } else {
                                    158.0
                                };
                                render_hint_bar(&tl("B/X=close  Up/Down=scroll"));
                                // Redraw game header on top of detail content
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 50.0, COL_PANEL);
                                    let ph = if current_lang() == Lang::Japanese {
                                        gs.current_phase.label_jp().to_string()
                                    } else {
                                        format!("{}", gs.current_phase)
                                    };
                                    _3ds_top_queue_text(
                                    4.0, 2.0, COL_GOLD, 0.65f32,
                                    format!(
                                        "T{} {} [{}]  Me H:{} E:{}/{} D:{}  Opp H:{} E:{}/{} D:{}\0",
                                        gs.turn_number, ph,
                                        if ap.id == pref(&gs, my_player_idx).id { "Me" } else { "Opp" },
                                        pref(&gs, my_player_idx).hand.cards.len(),
                                        pref(&gs, my_player_idx).energy_zone.active_count(),
                                        pref(&gs, my_player_idx).energy_zone.cards.len(),
                                        pref(&gs, my_player_idx).main_deck.cards.len(),
                                        pref(&gs, 1 - my_player_idx).hand.cards.len(),
                                        pref(&gs, 1 - my_player_idx).energy_zone.active_count(),
                                        pref(&gs, 1 - my_player_idx).energy_zone.cards.len(),
                                        pref(&gs, 1 - my_player_idx).main_deck.cards.len(),
                                    ).as_ptr(),
                                );
                                }
                            } // end else (not choice_subview)
                        } else {
                            if let Some(vcid) = viewing_card {
                                // Compact card info overlay with stats
                                if let Some(card) = gs.card_database.get_card(vcid) {
                                    let stats = compute_card_stats(card, vcid, &gs);
                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 52.0, 400.0, 76.0, COL_CARD);
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
                                                stats.cost.into(),
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
                                if !(has_image_choice || has_text_choice) && !is_ai_turn {
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
                                        _3ds_top_queue_rect(0.0, 52.0, 400.0, h, COL_ABILITY);
                                        render_text_with_icons(
                                            4.0,
                                            54.0,
                                            &ab_lines[0],
                                            COL_LIGHT,
                                            0.65,
                                        );
                                        for (li, line) in ab_lines.iter().enumerate().skip(1) {
                                            render_text_with_icons(
                                                8.0,
                                                54.0 + li as f32 * 14.0,
                                                line,
                                                COL_LIGHT,
                                                0.65,
                                            );
                                        }
                                    }
                                    content_y = 52.0 + h + 6.0;
                                }
                            }
                        }

                        // ---- Choice image mode: ability banner + card grid ----
                        // When detail_mode is active, the card detail overlay (above)
                        // replaces the grid so card images don't overlap the detail text.
                        {
                            let is_ai_turn = *ai_vs_ai || (*vs_ai && !mp_can_act(&gs, 0));
                            let is_opponent_turn_mp = is_multiplayer
                                && !mp_can_act(
                                    &gs,
                                    if is_multiplayer {
                                        if is_host {
                                            0
                                        } else {
                                            1
                                        }
                                    } else {
                                        0
                                    },
                                );
                            if zone_viewer.is_none() {
                                if (has_image_choice || has_text_choice)
                                    && !(detail_mode && viewing_card.is_some())
                                    && !is_ai_turn
                                    && !is_opponent_turn_mp
                                {
                                    // ---- Build option→text map from SelectAutoAbility ----
                                    let (opt_map, opt_ability_texts): (
                                        std::collections::HashMap<i16, i16>,
                                        std::collections::HashMap<i16, String>,
                                    ) = {
                                        let mut m = std::collections::HashMap::new();
                                        let mut t = std::collections::HashMap::new();
                                        if let Some(c) = gs.get_pending_choice() {
                                            use rabuka_engine::ability::types::Choice;
                                            if let Choice::SelectAutoAbility { options, .. } = c {
                                                for (i, opt) in options.iter().enumerate() {
                                                    let idx = i as i16;
                                                    if let Some(cid) = opt.card_id {
                                                        m.insert(idx, cid);
                                                    }
                                                    t.insert(idx, opt.ability_text.clone());
                                                }
                                            }
                                        }
                                        (m, t)
                                    };

                                    // ---- Resolve ability text for hovered card ----
                                    let hovered_ability_text: Option<String> =
                                        display_order.get(display_pos).and_then(|&fi| {
                                            let act = &acts_cache[fi];
                                            act.parameters.as_ref().and_then(|p| {
                                                p.card_id.and_then(|cid| {
                                                    opt_ability_texts.get(&cid).cloned()
                                                })
                                            })
                                        });
                                    let banner_text: String = hovered_ability_text
                                        .or_else(|| {
                                            gs.ability_queue.current_entry().map(|e| {
                                                i18n::translate_ability(
                                                    &e.ability.full_text,
                                                    current_lang(),
                                                )
                                            })
                                        })
                                        .unwrap_or_default();

                                    // ---- Render ability banner first ----
                                    let mut grid_iy: f32 = 52.0;
                                    if !banner_text.is_empty() {
                                        let ab_lines: Vec<String> =
                                            wrap_ability_text(&banner_text, 392.0, 0.60)
                                                .lines()
                                                .take(2)
                                                .map(|l| l.to_string())
                                                .collect();
                                        let n_lines = ab_lines.len();
                                        let h = 16.0 + n_lines as f32 * 13.0;
                                        unsafe {
                                            _3ds_top_queue_rect(0.0, 52.0, 400.0, h, COL_ABILITY);
                                        }
                                        for (li, line) in ab_lines.iter().enumerate() {
                                            render_text_with_icons(
                                                4.0,
                                                52.0 + 2.0 + li as f32 * 13.0,
                                                line,
                                                COL_LIGHT,
                                                0.60,
                                            );
                                        }
                                        grid_iy = 52.0 + h + 4.0;
                                    }
                                    // ---- Dynamic card sizing (matches waitroom) ----
                                    let has_ability = gs.ability_queue.current_entry().is_some();
                                    let cols = 5usize;
                                    let gap = 4.0f32;
                                    let max_rows = if has_ability { 1 } else { 2 };
                                    let max_ch = ((230.0 - grid_iy) / max_rows as f32) - 14.0;
                                    let cw = (max_ch * 0.711).min(
                                        (400.0 - 8.0 - (cols as f32 - 1.0) * gap) / cols as f32,
                                    );
                                    let ch = cw / 0.711;
                                    let row_h = ch + 16.0 + gap;
                                    let pp = cols * max_rows;
                                    let page = (choice_grid_offset / pp) * pp;
                                    let n = display_order.len();

                                    // ---- Classify items on this page ----
                                    let mut card_gis: Vec<usize> = Vec::new();
                                    let mut text_gis: Vec<usize> = Vec::new();
                                    for gi in 0..pp {
                                        let di = page + gi;
                                        if di >= n {
                                            break;
                                        }
                                        let fi = display_order[di];
                                        if is_text_only(&acts_cache[fi]) {
                                            text_gis.push(gi);
                                        } else {
                                            card_gis.push(gi);
                                        }
                                    }

                                    // ---- Render card items in grid ----
                                    for (ci, &gi) in card_gis.iter().enumerate() {
                                        let di = page + gi;
                                        let fi = display_order[di];
                                        let act = &acts_cache[fi];
                                        let is_disabled = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.disabled)
                                            .unwrap_or(false);
                                        let col = ci % cols;
                                        let row = ci / cols;
                                        let ix = 4.0 + col as f32 * (cw + gap);
                                        let iy_card = grid_iy + row as f32 * row_h;

                                        let real_cid = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.card_id)
                                            .and_then(|idx| opt_map.get(&idx).copied())
                                            .or_else(|| {
                                                act.parameters.as_ref().and_then(|p| p.card_id)
                                            });
                                        if let Some(cid) = real_cid {
                                            if let Some(cn) = gs
                                                .card_database
                                                .get_card(cid)
                                                .map(|c| c.card_no.to_string())
                                            {
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
                                                        let label = if act.action_type == game_setup::ActionType::PlayMemberToStage {
                                                            let cost = act.parameters.as_ref().and_then(|p| p.base_cost).unwrap_or(0);
                                                            format!("E{} {}\0", cost, cn)
                                                        } else {
                                                            format!("{}\0", cn)
                                                        };
                                                        _3ds_top_queue_text(
                                                            ix + 1.0,
                                                            iy_card + ch + 1.0,
                                                            COL_LIGHT,
                                                            0.45f32,
                                                            label.as_ptr(),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // ---- Render text items as one-per-page ----
                                    if let Some(&sel_gi) =
                                        text_gis.iter().find(|&&g| g == display_pos)
                                    {
                                        let fi = display_order[sel_gi];
                                        let act = &acts_cache[fi];
                                        let is_disabled = act
                                            .parameters
                                            .as_ref()
                                            .and_then(|p| p.disabled)
                                            .unwrap_or(false);
                                        let desc =
                                            act.display_desc(current_lang() == Lang::Japanese);
                                        let desc_nlb = desc.replace('\n', " ");
                                        let desc_clean = desc_nlb
                                            .trim_start_matches(|c: char| {
                                                c == '・' || c == '\u{2022}'
                                            })
                                            .trim_start_matches("- ")
                                            .trim();
                                        let color = if is_disabled { COL_MED } else { COL_LIGHT };
                                        let scale = 0.70f32;
                                        let full_txt = desc_clean.to_string();
                                        let total_h = unsafe {
                                            _3ds_text_wrapped_height(
                                                format!("{}\0", full_txt).as_ptr(),
                                                scale,
                                                380.0,
                                            )
                                        };
                                        let iy = grid_iy + ((230.0 - grid_iy) - total_h) / 2.0;

                                        unsafe {
                                            _3ds_top_queue_rect(
                                                4.0,
                                                iy - 2.0,
                                                392.0,
                                                total_h + 4.0,
                                                COL_DIM,
                                            );
                                            render_text_with_icons(
                                                8.0,
                                                iy + 2.0,
                                                &full_txt,
                                                color,
                                                scale,
                                            );
                                        }
                                        // Page indicator
                                        let total = text_gis.len();
                                        if total > 1 {
                                            let cur = text_gis
                                                .iter()
                                                .position(|&g| g == display_pos)
                                                .unwrap_or(0)
                                                + 1;
                                            unsafe {
                                                _3ds_top_queue_text(
                                                    4.0,
                                                    232.0,
                                                    COL_MED,
                                                    0.55f32,
                                                    format!("{}/{}\0", cur, total).as_ptr(),
                                                );
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
                                    if n > pp {
                                        let pg = page / pp + 1;
                                        let total_p = (n + pp - 1) / pp;
                                        unsafe {
                                            _3ds_top_queue_text(
                                                300.0,
                                                228.0,
                                                COL_MED,
                                                0.45f32,
                                                format!("{}\0", format!("{}/{}", pg, total_p))
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
                                                    0.0, 52.0, 400.0, 198.0, 0xCC000000,
                                                );
                                                _3ds_top_queue_text(
                                                    4.0,
                                                    44.0,
                                                    COL_BLUE,
                                                    0.65f32,
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
                                                render_hint_bar(&tl("L/B=close"));
                                            }
                                        }
                                    }
                                } else if is_ai_turn && content_y < 230.0 {
                                    unsafe {
                                        _3ds_top_queue_text(
                                            4.0,
                                            content_y,
                                            COL_MED,
                                            0.65f32,
                                            format!("{}\0", tl("AI is thinking...")).as_ptr(),
                                        );
                                    }
                                } else if !is_ai_turn
                                    && !is_opponent_turn_mp
                                    && !display_order.is_empty()
                                    && content_y < 240.0
                                    && !detail_mode
                                {
                                    let mut ty = content_y;
                                    let max_vis = ((230.0 - content_y) / 20.0) as usize + 1;
                                    let n = display_order.len();
                                    // Stable scroll: only adjust when cursor goes out of visible range
                                    if list_scroll >= n.saturating_sub(max_vis) {
                                        list_scroll = n.saturating_sub(max_vis);
                                    }
                                    if display_pos < list_scroll {
                                        list_scroll = display_pos.saturating_sub(max_vis / 3);
                                    } else if display_pos >= list_scroll + max_vis {
                                        list_scroll = display_pos.saturating_sub(max_vis / 3);
                                    }
                                    let start = list_scroll.min(n.saturating_sub(max_vis));
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
                                    while di < end && ty < 230.0 {
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
                                        let is_group = is_pmts && this_cid != -1;
                                        let group_sel =
                                            is_group && (di..ge).any(|i| i == display_pos);
                                        let line_color = if group_sel || is_sel {
                                            COL_GOLD
                                        } else if is_disabled {
                                            COL_MED
                                        } else {
                                            COL_LIGHT
                                        };
                                        let line_scale: f32 = 0.65;
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
                                                if base_cost > 0 {
                                                    format!(
                                                        "{{{{icon_energy.png|E}}}}{} [{}] {}",
                                                        base_cost, cn, name
                                                    )
                                                } else {
                                                    format!("[{}] {}", cn, name)
                                                }
                                            } else {
                                                if base_cost > 0 {
                                                    format!(
                                                        "{{{{icon_energy.png|E}}}}{} {}",
                                                        base_cost, name
                                                    )
                                                } else {
                                                    name.clone()
                                                }
                                            };
                                            let mut areas = String::new();
                                            let area_costs: std::collections::HashMap<
                                                String,
                                                (u8, bool),
                                            > = if let Some(ref p) =
                                                acts_cache[display_order[di]].parameters
                                            {
                                                p.available_areas
                                                    .as_ref()
                                                    .map(|areas_vec| {
                                                        areas_vec
                                                            .iter()
                                                            .map(|a| {
                                                                (
                                                                    a.area.clone(),
                                                                    (a.cost, a.is_baton_touch),
                                                                )
                                                            })
                                                            .collect()
                                                    })
                                                    .unwrap_or_default()
                                            } else {
                                                Default::default()
                                            };
                                            for i in di..ge {
                                                let gact = &acts_cache[display_order[i]];
                                                let stage = gact
                                                    .parameters
                                                    .as_ref()
                                                    .and_then(|p| p.stage_area.clone())
                                                    .unwrap_or_default();
                                                let prefix =
                                                    if i == display_pos { "[" } else { "" };
                                                let suffix =
                                                    if i == display_pos { "]" } else { "" };
                                                // For double baton pairs: dest+source(s)
                                                // Double baton desc format: "Card (src1+src2)→dst cost:N"
                                                // Regular desc format: "Card → dst (cost:N)"
                                                // Only parse if ( comes before →
                                                let desc = gact
                                                    .display_desc(current_lang() == Lang::Japanese);
                                                if let Some(paren_pos) = desc.find('(') {
                                                    let arrow_pos = desc.find('→');
                                                    if arrow_pos.map_or(true, |a| paren_pos < a) {
                                                        // Double baton: extract sources from (src1+src2)
                                                        if let Some(end) =
                                                            desc[paren_pos..].find(')')
                                                        {
                                                            let sources: String = desc
                                                                [paren_pos + 1..paren_pos + end]
                                                                .split('+')
                                                                .map(|a| a.trim())
                                                                .filter(|a| {
                                                                    !a.eq_ignore_ascii_case(&stage)
                                                                })
                                                                .map(|a| tl_area(a).to_string())
                                                                .collect::<Vec<_>>()
                                                                .join("+");
                                                            areas.push_str(&format!(
                                                                "{}{}+{}{} ",
                                                                prefix,
                                                                tl_area(&stage),
                                                                sources,
                                                                suffix
                                                            ));
                                                            continue;
                                                        }
                                                    }
                                                }
                                                // Regular single-area action with per-area cost
                                                let area_cost_info = area_costs.get(&stage);
                                                let area_str = match area_cost_info {
                                                    Some((cost, true)) if *cost > 0 => format!(
                                                        "{} {{{{icon_energy.png|E}}}}{}BT{}{}",
                                                        prefix,
                                                        cost,
                                                        tl_area(&stage),
                                                        suffix
                                                    ),
                                                    Some((cost, false)) if *cost > 0 => format!(
                                                        "{} {{{{icon_energy.png|E}}}}{}{}{}",
                                                        prefix,
                                                        cost,
                                                        tl_area(&stage),
                                                        suffix
                                                    ),
                                                    _ => format!(
                                                        "{}{}{}",
                                                        prefix,
                                                        tl_area(&stage),
                                                        suffix
                                                    ),
                                                };
                                                areas.push_str(&area_str);
                                            }
                                            let hdr_prefix = "";
                                            for (li, l) in wrap_text(&hdr, 370.0, line_scale)
                                                .lines()
                                                .enumerate()
                                            {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                let txt = format!("{}{}", hdr_prefix, l);
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
                                            let areas_prefix = "";
                                            for (li, l) in wrap_text(&areas, 370.0, line_scale)
                                                .lines()
                                                .enumerate()
                                            {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                let txt = format!("{}{}", areas_prefix, l);
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
                                            di = ge;
                                        } else {
                                            let prefix = if is_sel {
                                                ""
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
                                                        if base_cost > 0 {
                                                            format!(
                                                                "{{{{icon_energy.png|E}}}}{} [{}] {} {}",
                                                                base_cost, cn, name, area_label
                                                            )
                                                        } else {
                                                            format!(
                                                                "[{}] {} {}",
                                                                cn, name, area_label
                                                            )
                                                        }
                                                    } else {
                                                        if base_cost > 0 {
                                                            format!(
                                                                "{{{{icon_energy.png|E}}}}{} {} {}",
                                                                base_cost, name, area_label
                                                            )
                                                        } else {
                                                            format!("{} {}", name, area_label)
                                                        }
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
                                                                "{{{{icon_energy.png|E}}}}{} [{}] {} {} {}",
                                                                cost, cn, name, area_label, abil_short
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
                                                                "{{{{icon_energy.png|E}}}}{} {} {} {}",
                                                                cost, name, area_label, abil_short
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
                                                            format!("[{}] [{}] {}", label, cn, name)
                                                        } else if !cn.is_empty() {
                                                            format!("[{}] [{}]", label, cn)
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
                                            let scale: f32 = 0.65;
                                            let wrap_w =
                                                if !prefix.is_empty() { 370.0 } else { 392.0 };
                                            for (li, l) in
                                                wrap_text(&line, wrap_w, scale).lines().enumerate()
                                            {
                                                if ty > 230.0 {
                                                    break;
                                                }
                                                let txt = format!("{}{}", prefix, l);
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
                                    if end < n && ty < 230.0 {
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
                            let ai_turn = *ai_vs_ai || (*vs_ai && !mp_can_act(&gs, 0));
                            let opp_turn = is_multiplayer
                                && !mp_can_act(
                                    &gs,
                                    if is_multiplayer {
                                        if is_host {
                                            0
                                        } else {
                                            1
                                        }
                                    } else {
                                        0
                                    },
                                );
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
                                        // Hand card for PlayMemberToStage + stage slots in detail mode
                                        game_setup::ActionType::PlayMemberToStage => {
                                            if detail_mode && viewing_card.is_some() {
                                                // In detail mode: highlight stage target slots
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
                                            } else {
                                                // Normal mode: highlight the hand card that can be played
                                                if let Some(cid) = p.card_id {
                                                    if let Some((zone, slot, opp)) =
                                                        find_card_zone_slot(&gs, cid, my_player_idx)
                                                    {
                                                        if zone == 3 {
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
                                        // Stage cards for UseAbility
                                        game_setup::ActionType::UseAbility => {
                                            if let Some(cid) = p.card_id {
                                                if let Some((zone, slot, opp)) =
                                                    find_card_zone_slot(&gs, cid, my_player_idx)
                                                {
                                                    unsafe {
                                                        _3ds_board_set_action_highlight(
                                                            zone, slot, opp,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        // Stage slots for ChoicePosition (choice mode)
                                        game_setup::ActionType::ChoicePosition => {
                                            if has_image_choice {
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
                                        // Hand cards for SelectMulligan — only highlight if selected
                                        game_setup::ActionType::SelectMulligan => {
                                            if act.selected == Some(true) {
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
                                        }
                                        // Hand cards for SelectLiveCard — only highlight if selected
                                        game_setup::ActionType::SelectLiveCard => {
                                            if act.selected == Some(true) {
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
                                        }
                                        // Board cards for choice image mode (ChoiceSelect, ChoiceDecision, ChoiceOption)
                                        _ => {
                                            if has_image_choice
                                                && matches!(
                                                    act.action_type,
                                                    game_setup::ActionType::ChoiceSelect
                                                        | game_setup::ActionType::ChoiceDecision
                                                )
                                            {
                                                if let Some(cid) = p.card_id {
                                                    if let Some((zone, slot, opp)) =
                                                        find_card_zone_slot(&gs, cid, my_player_idx)
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
                        if !(*ai_vs_ai || (*vs_ai && !mp_can_act(&gs, 0)))
                            && !(is_multiplayer
                                && !mp_can_act(
                                    &gs,
                                    if is_multiplayer {
                                        if is_host {
                                            0
                                        } else {
                                            1
                                        }
                                    } else {
                                        0
                                    },
                                ))
                            && has_image_choice
                        {
                            if let Some(c) = gs.get_pending_choice() {
                                use rabuka_engine::ability::types::Choice;
                                if let Choice::SelectAutoAbility { options, .. } = c {
                                    for opt in options {
                                        if let Some(cid) = opt.card_id {
                                            if let Some((zone, slot, opp)) =
                                                find_card_zone_slot(&gs, cid, my_player_idx)
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
                                    "MP|tx={} rx={} pseq={} ap={} my={} can={} wait={} phase={} acts={}\0",
                                    dbg_tx_bytes,
                                    dbg_rx_bytes,
                                    if is_host {
                                        pending_state.as_ref().map(|(s, _, _)| *s).unwrap_or(0)
                                    } else {
                                        state_rx.in_progress_seq().unwrap_or(0)
                                    },
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
                                    let prefix = "";
                                    _3ds_top_queue_text(
                                        70.0,
                                        iy + 4.0,
                                        COL_LIGHT,
                                        0.60f32,
                                        format!("{}{}\0", prefix, item).as_ptr(),
                                    );
                                }
                                render_hint_bar(&tl("UP/DOWN=move, A=select, B=close"));
                            },
                            Overlay::GameLog(offset, cursor) => {
                                let logs = &gs.rule_log;
                                let n = logs.len();
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, 0xCC000000);
                                    let log_hdr = tl("Game Log");
                                    _3ds_top_queue_text(
                                        4.0,
                                        2.0,
                                        COL_GOLD,
                                        0.65f32,
                                        format!(
                                            "{}  {} entries (B=close, UP/DOWN=scroll)\0",
                                            log_hdr, n
                                        )
                                        .as_ptr(),
                                    );
                                }
                                let max_vis = 12usize;
                                let end_idx = n.saturating_sub(offset);
                                let start_idx = end_idx.saturating_sub(max_vis);
                                let mut ly = 20.0_f32;
                                for idx in (start_idx..end_idx).rev() {
                                    let entry = &logs[idx];
                                    let truncated = if entry.chars().count() > 55 {
                                        let cutoff = entry
                                            .char_indices()
                                            .nth(55)
                                            .map(|(i, _)| i)
                                            .unwrap_or(entry.len());
                                        &entry[..cutoff]
                                    } else {
                                        &entry[..]
                                    };
                                    let is_cursor = idx == cursor;
                                    let col = if is_cursor { COL_GOLD } else { 0xFFCCCCCC };
                                    let prefix = "";
                                    unsafe {
                                        _3ds_top_queue_text(
                                            4.0,
                                            ly,
                                            col,
                                            0.60f32,
                                            format!("{}{}\0", prefix, truncated).as_ptr(),
                                        );
                                    }
                                    ly += 16.0;
                                }
                                if n > max_vis {
                                    let lo = start_idx + 1;
                                    let hi = end_idx.min(n);
                                    unsafe {
                                        _3ds_top_queue_text(
                                            300.0,
                                            2.0,
                                            COL_MED,
                                            0.50f32,
                                            format!("{}-{} of {}\0", lo, hi, n).as_ptr(),
                                        );
                                    }
                                }
                            }
                            Overlay::PerfStats(detail, cursor) => {
                                unsafe {
                                    _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, 0xCC000000);
                                    let perf_hdr = tl("Performance");
                                    _3ds_top_queue_text(
                                        4.0,
                                        2.0,
                                        COL_GOLD,
                                        0.65f32,
                                        format!(
                                            "{}  (B=close, A=detail, UP/DOWN=select)\0",
                                            perf_hdr
                                        )
                                        .as_ptr(),
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
                                                20.0,
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
                                                34.0,
                                                COL_MED,
                                                0.55f32,
                                                format!("{}\0", tl("Lives:")).as_ptr(),
                                            );
                                        }
                                        let mut ly = 48.0;
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
                                                    0.60f32,
                                                    format!(
                                                        "{} #{} {} score:{}\0",
                                                        cn, li, status, lc.score
                                                    )
                                                    .as_ptr(),
                                                );
                                            }
                                            ly += 16.0;
                                            if ly > 225.0 {
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    let mut ly = 20.0;
                                    let max_vis = 8usize;
                                    let total = snapshots.len();
                                    let display_start = total.saturating_sub(cursor + 1);
                                    let display_end = display_start.saturating_sub(max_vis);
                                    for idx in (display_end..display_start).rev() {
                                        if idx >= total {
                                            continue;
                                        }
                                        let s = &snapshots[idx];
                                        let is_cur = idx == cursor;
                                        let label = format!(
                                            "T{} {} score:{} hearts:{} pass:{}/{} succ:{}",
                                            s.turn,
                                            s.player_id,
                                            s.total_score,
                                            s.total_hearts
                                                .iter()
                                                .copied()
                                                .map(u32::from)
                                                .sum::<u32>(),
                                            s.lives.iter().filter(|l| l.passed).count(),
                                            s.lives.len(),
                                            s.success
                                        );
                                        let base_col =
                                            if s.success { 0xFF88FF88 } else { 0xFFFF8888 };
                                        let col = if is_cur { COL_GOLD } else { base_col };
                                        let prefix = "";
                                        let truncated = if label.chars().count() > 55 {
                                            let cutoff = label
                                                .char_indices()
                                                .nth(55)
                                                .map(|(i, _)| i)
                                                .unwrap_or(label.len());
                                            &label[..cutoff]
                                        } else {
                                            &label
                                        };
                                        unsafe {
                                            _3ds_top_queue_text(
                                                4.0,
                                                ly,
                                                col,
                                                0.60f32,
                                                format!("{}{}\0", prefix, truncated).as_ptr(),
                                            );
                                        }
                                        ly += 15.0;
                                    }
                                }
                            }
                            Overlay::RevealedCards(show_self, ref cursor, view_card) => {
                                if let Some(vcid) = view_card {
                                    render_card_detail(vcid, &gs.card_database, 0.0);
                                } else {
                                    let who = if show_self { tl("You") } else { tl("Opponent") };
                                    let rev_hdr = tl("Revealed Cards");
                                    let filter_owner: Option<u8> = if show_self {
                                        if is_host {
                                            Some(0)
                                        } else {
                                            Some(1)
                                        }
                                    } else {
                                        if is_host {
                                            Some(1)
                                        } else {
                                            Some(0)
                                        }
                                    };
                                    let mut owner_of: HashMap<i16, Option<u8>> = HashMap::new();
                                    for (i, &cid) in gs.revealed_cards.iter().enumerate() {
                                        if let Some(meta) = gs.revealed_card_meta.get(i) {
                                            owner_of.insert(cid, meta.owner);
                                        }
                                    }
                                    for (i, &cid) in gs.revealed_cost_cards.iter().enumerate() {
                                        if let Some(meta) = gs.revealed_cost_card_meta.get(i) {
                                            owner_of.insert(cid, meta.owner);
                                        }
                                    }
                                    let filter_cards = |cards: &[i16]| -> Vec<i16> {
                                        cards
                                            .iter()
                                            .filter(|&&cid| {
                                                if let Some(owner) = owner_of.get(&cid) {
                                                    *owner == filter_owner || owner.is_none()
                                                } else {
                                                    true
                                                }
                                            })
                                            .copied()
                                            .collect()
                                    };
                                    struct RevSection {
                                        label: &'static str,
                                        cards: Vec<i16>,
                                    }
                                    let sections: Vec<RevSection> = vec![
                                        RevSection {
                                            label: "Yell",
                                            cards: filter_cards(&gs.initial_yell_revealed_cards),
                                        },
                                        RevSection {
                                            label: "Re-Yell",
                                            cards: filter_cards(&gs.re_yell_revealed_cards),
                                        },
                                        RevSection {
                                            label: "Cost",
                                            cards: filter_cards(&gs.revealed_cost_cards),
                                        },
                                        RevSection {
                                            label: "Effects",
                                            cards: filter_cards(&gs.revealed_cards),
                                        },
                                    ];
                                    let total_cards: usize =
                                        sections.iter().map(|s| s.cards.len()).sum();
                                    let mut flat: Vec<i16> = Vec::new();
                                    for sec in &sections {
                                        flat.extend(sec.cards.iter().copied());
                                    }
                                    unsafe {
                                        _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                                        _3ds_top_queue_text(
                                            4.0,
                                            4.0,
                                            COL_GOLD,
                                            0.65f32,
                                            format!(
                                                "{} ({})  {} cards  (B=close, X=detail)\0",
                                                rev_hdr, who, total_cards
                                            )
                                            .as_ptr(),
                                        );
                                    }
                                    if flat.is_empty() {
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
                                        render_card_grid(
                                            &flat,
                                            *cursor as usize,
                                            5,
                                            2,
                                            28.0,
                                            &gs.card_database,
                                            atlas,
                                        );
                                        let mut sec_text = String::new();
                                        for sec in &sections {
                                            if !sec.cards.is_empty() {
                                                if !sec_text.is_empty() {
                                                    sec_text.push_str("  ");
                                                }
                                                sec_text.push_str(sec.label);
                                                sec_text.push('(');
                                                sec_text.push_str(&sec.cards.len().to_string());
                                                sec_text.push(')');
                                            }
                                        }
                                        unsafe {
                                            _3ds_top_queue_text(
                                                4.0,
                                                228.0,
                                                COL_MED,
                                                0.45f32,
                                                format!("{}\0", sec_text).as_ptr(),
                                            );
                                        }
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
                    list_scroll,
                    detail_scroll_y,
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
                    state_rx,
                    pending_state,
                    state_init,
                    pending_client_action,
                    last_client_action_seq,
                    next_action_seq,
                    dbg_tx_bytes,
                    dbg_rx_bytes,
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
fn find_card_zone_slot(gs: &GameState, cid: i16, my_player_idx: usize) -> Option<(i32, i32, bool)> {
    for (pi, p) in [&gs.player1, &gs.player2].iter().enumerate() {
        let opp = pi != my_player_idx;
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
        Step::Play(
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
            _,
        ) => "Play",
        Step::Done(_) => "Done",
    }
}

/// Serialize the authoritative GameState and stage it for delivery to the
/// client. Returns (state_seq, chunks, acked_bitmap). The per-frame loop
/// retransmits only the unacked chunks until the client's bitmap ACK clears
/// them, because UDS is unreliable and can drop frames.
fn stage_authoritative_state(gs: &GameState, prev_seq: u16) -> (u16, Vec<Vec<u8>>, Vec<bool>) {
    use rabuka_3ds::uds;
    let seq = prev_seq.wrapping_add(1);
    if let Ok(bytes) = rmp_serde::to_vec(gs) {
        let chunks = uds::state_chunks(&bytes, seq);
        let acked = vec![false; chunks.len()];
        (seq, chunks, acked)
    } else {
        (seq, Vec::new(), Vec::new())
    }
}

/// Convert an ActionType to its wire tag (matches ActionSync::from_bytes).
fn action_tag_of(at: &game_setup::ActionType) -> u16 {
    match at {
        game_setup::ActionType::RockChoice => 0,
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
    }
}

/// Route an action through the authoritative model:
/// - single player: execute locally
/// - host: execute locally + ship state to client
/// - client: send action to host (no local execution; host ships state back)
/// Returns true if the action was executed locally (host/single).
/// When executed on the host in multiplayer, stages the new state into
/// `pending_state` for reliable delivery (retransmit-until-ACK).
#[allow(clippy::too_many_arguments)]
fn route_authoritative_action(
    gs: &mut GameState,
    action: &game_setup::Action,
    is_multiplayer: bool,
    is_host: bool,
    waiting_for_opponent: &mut bool,
    pending_state: &mut Option<(u16, Vec<Vec<u8>>, Vec<bool>)>,
    pending_client_action: &mut Option<Vec<u8>>,
    next_action_seq: &mut u32,
) -> bool {
    use rabuka_3ds::uds;
    let p = action.parameters.clone();
    if !is_multiplayer || is_host {
        // Tag RPS choices with the acting player id so the engine routes them
        // to the right slot (host=P1). Only in multiplayer — sandbox uses the
        // sequential P1-then-P2 fallback. Cleared by the action execution.
        if is_multiplayer
            && matches!(
                action.action_type,
                game_setup::ActionType::RockChoice
                    | game_setup::ActionType::PaperChoice
                    | game_setup::ActionType::ScissorsChoice
            )
        {
            gs.pending_rps_player_id = Some(0);
        }
        let result = turn::TurnEngine::execute_main_phase_action_with_ability_index(
            gs,
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
        if is_multiplayer {
            *waiting_for_opponent = !mp_can_act(gs, 0);
            let prev = pending_state.as_ref().map(|(s, _, _)| *s).unwrap_or(0);
            let staged = stage_authoritative_state(gs, prev);
            *pending_state = Some(staged);
        }
        true
    } else {
        // Client: send action, don't execute. Host will ship the new state.
        // Store the bytes so the per-frame loop can retransmit until the host
        // replies with a new state (implicit ACK) — UDS may drop the packet.
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
            action_tag: action_tag_of(&action.action_type),
            card_id: p.as_ref().and_then(|x| x.card_id),
            card_indices: p
                .as_ref()
                .and_then(|x| x.card_indices.clone())
                .unwrap_or_default(),
            stage_area,
            use_baton_touch: p.as_ref().and_then(|x| x.use_baton_touch).unwrap_or(false),
            ability_index: p.as_ref().and_then(|x| x.ability_index).map(|x| x as u16),
            action_seq: *next_action_seq,
        };
        *next_action_seq = next_action_seq.wrapping_add(1);
        let bytes = sync.to_bytes();
        *pending_client_action = Some(bytes.clone());
        let _ = uds::uds_send(&bytes);
        *waiting_for_opponent = true;
        false
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
    fn _3ds_board_set_live_stats(slot: i32, score: i32, stat_text: *const u8);
    fn _3ds_board_set_opp_live_stats(slot: i32, score: i32, stat_text: *const u8);
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
    fn _3ds_text_wrapped_height(text: *const u8, scale: f32, max_w: f32) -> f32;
    fn _3ds_icon_aspect(atlas_name: *const u8) -> f32;

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
    // Need hearts counts displayed next to live zone on bottom screen
    fn _3ds_set_need_hearts(
        player: i32,
        h0: u32,
        h1: u32,
        h2: u32,
        h3: u32,
        h4: u32,
        h5: u32,
        h6: u32,
        h7: u32,
    );
    // QR code scanning (camera + quirc, same tech used by FBI installer)
    fn _3ds_qr_start() -> *mut u8;
    fn _3ds_qr_stop(ctx: *mut u8);
    fn _3ds_qr_free(ctx: *mut u8);
    fn _3ds_qr_poll(ctx: *mut u8, out_text: *mut u8, out_max: u32) -> i32;
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
