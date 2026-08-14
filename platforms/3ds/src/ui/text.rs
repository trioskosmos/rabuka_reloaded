// Pure text/UI helpers: word wrapping, icon markers, card stat lines.

use std::ffi::CString;

use rabuka_engine::card::{Card, HeartMap};
use rabuka_engine::core::game_modifiers::GameModifiers;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::GameState;

use crate::ffi::_3ds_icon_aspect;
use crate::ffi::_3ds_measure_text_width;
use crate::ffi::_3ds_top_queue_text;

// ---------------------------------------------------------------------------
// Font sizes — the ONLY three sizes the game uses, plus a debug/CLI size.
//
// Every text scale literal in the UI must be one of these constants instead of
// a raw number. To change the overall font size, edit these values (they are
// the single knob). The citro2d renderer multiplies each by FONT_SCALE (1.2 in
// ctru_shim.c), and Rust word-wrapping uses the same multiplier, so these are
// "logical" scales — resize globally by scaling this block.
//
// (Consolidated 2026: the old code had ~10 ad-hoc sizes 0.45–0.85.)
// ---------------------------------------------------------------------------
/// Captions, hints, page dots, stat lines.
pub const SCALE_SMALL: f32 = 0.40;
/// Standard body text, card labels, action lines, ability text.
pub const SCALE_BODY: f32 = 0.52;
/// Titles / headers / card name headers.
pub const SCALE_LARGE: f32 = 0.64;
/// CLI / debug console text.
pub const SCALE_CLI: f32 = 0.68;

/// Render text with inline `{{icon.png|label}}` icon images.
/// Queue text for top screen rendering. C-side OP_TEXT handler parses {{icon}} markup natively.
pub fn render_text_with_icons(x: f32, y: f32, text: &str, color: u32, scale: f32) {
    let c_str = CString::new(text).unwrap_or_default();
    unsafe {
        _3ds_top_queue_text(x, y, color, scale, c_str.as_ptr() as *const u8);
    }
}

/// Calculate icon display width at a given height, using actual texture aspect ratio.
pub fn icon_width_for(file: &str, h: f32) -> f32 {
    let icon_name = file.strip_suffix(".png").unwrap_or(file);
    let atlas_name = format!("icon_{}.png.t3x", icon_name);
    let c_str = CString::new(atlas_name.as_str()).unwrap_or_default();
    let aspect = unsafe { _3ds_icon_aspect(c_str.as_ptr() as *const u8) };
    if aspect > 0.0 {
        h * aspect
    } else {
        h
    }
}

/// Convert heart label like "h061+1" to icon + count: "{{heart_06.png|h06}} 1+1"
pub fn heart_label_to_icon(s: &str) -> String {
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
pub fn build_heart_str(
    hearts: &HeartMap,
    card_id: i16,
    mods: &GameModifiers,
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
pub fn card_stat_line(
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
pub fn measure_text_width(s: &str, scale: f32) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    let c_str = CString::new(s).unwrap_or_default();
    unsafe { _3ds_measure_text_width(c_str.as_ptr() as *const u8, scale) }
}

/// Build a single-line string that never wraps: truncate with an ellipsis if it
/// would exceed `max_px` at `scale`. Keeps the leading `[id] ` prefix intact and
/// lets the tail overflow off-screen rather than wrapping onto a second line.
pub fn truncate_to_width(prefix: &str, text: &str, scale: f32, max_px: f32) -> String {
    let full = format!("{}{}", prefix, text);
    if measure_text_width(&full, scale) <= max_px {
        return full;
    }
    // Shrink the name portion until it fits; always keep the prefix.
    let prefix_w = measure_text_width(prefix, scale);
    let avail = (max_px - prefix_w).max(0.0);
    let mut name = text.to_string();
    loop {
        if name.is_empty() || measure_text_width(&name, scale) <= avail {
            break;
        }
        let chars: Vec<char> = name.chars().collect();
        if chars.len() <= 1 {
            break; // can't shrink further
        }
        let cut = (chars.len() * 9) / 10; // drop ~10% per iteration
        name = chars.into_iter().take(cut.max(1)).collect();
    }
    format!("{}{}", prefix, name)
}

/// Pre-computed card display stats for detail rendering.
pub struct CardDisplayStats {
    pub is_tapped: bool,
    pub total_blade: i32,
    pub score: i32,
    pub cost: u8,
    pub heart_str: String,
    pub need_heart_str: String,
}

pub fn compute_card_stats(card: &Card, cid: i16, gs: &GameState) -> CardDisplayStats {
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
pub fn is_text_only(act: &game_setup::Action) -> bool {
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
pub fn wrap_ability_text(s: &str, max_px: f32, scale: f32) -> String {
    wrap_text(s, max_px, scale)
}

/// Render-side font scale multiplier (must match ctru_shim.c font_scale)
const FONT_SCALE: f32 = 1.2;

/// Truncate text at segment boundaries, keeping `{{...}}` icon markers intact.
/// Only plain text characters count toward the character limit.
pub fn truncate_aware_segments(s: &str, max_chars: usize) -> String {
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
                    break;
                }
            }
        }
    }
    result
}

/// Segments of text: either a plain text span or an icon marker like {{icon.png|alt}}
#[derive(Debug)]
pub enum TextSeg {
    Text(String),
    Icon(String),
}

/// Split text into segments, treating `{{...}}` markers as atomic units.
pub fn segment_text(s: &str) -> Vec<TextSeg> {
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

pub fn wrap_text(s: &str, max_px: f32, scale: f32) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rabuka_engine::card::{HeartColor, HeartMap};
    use rabuka_engine::core::game_modifiers::GameModifiers;

    #[test]
    fn heart_label_to_icon_formats() {
        assert_eq!(heart_label_to_icon("h061+1"), "{{heart_06.png|h06}} 1+1");
        assert_eq!(heart_label_to_icon("h012"), "{{heart_01.png|h01}} 2");
        assert_eq!(heart_label_to_icon("b_all"), "{{icon_b_all.png|ALL}}");
        assert_eq!(heart_label_to_icon("draw"), "{{icon_draw.png|DRAW}}");
        assert_eq!(heart_label_to_icon("plain"), "plain");
    }

    #[test]
    fn segment_text_splits_icons() {
        let segs = segment_text("a{{icon.png|x}}b");
        assert_eq!(segs.len(), 3);
        assert!(matches!(&segs[0], TextSeg::Text(t) if t == "a"));
        assert!(matches!(&segs[1], TextSeg::Icon(i) if i == "icon.png|x"));
        assert!(matches!(&segs[2], TextSeg::Text(t) if t == "b"));
        // No markers: one plain text segment
        let plain = segment_text("hello");
        assert_eq!(plain.len(), 1);
        assert!(matches!(&plain[0], TextSeg::Text(t) if t == "hello"));
    }

    #[test]
    fn truncate_aware_segments_keeps_icons() {
        // Icon before the cut is preserved; plain text is truncated to max_chars
        assert_eq!(
            truncate_aware_segments("{{icon.png|x}}abcdef", 3),
            "{{icon.png|x}}abc"
        );
        // Text after the icon does not count toward the character budget
        assert_eq!(truncate_aware_segments("abcdef{{icon.png|x}}gh", 3), "abc");
    }

    #[test]
    fn card_stat_line_member() {
        let s = card_stat_line(10, "h012 h061", 0, 2, false, "member_card", "");
        assert!(s.contains("icon_energy.png"));
        assert!(s.contains("icon_blade.png"));
        assert!(s.contains("h01"));
        assert!(s.contains("10"));
    }

    #[test]
    fn card_stat_line_live() {
        let s = card_stat_line(0, "", 5, 0, false, "live_card", "h012");
        assert!(s.contains("icon_score.png"));
        assert!(s.contains("{{heart_01.png|h01}} 2"));
    }

    #[test]
    fn build_heart_str_sorted_with_bonus() {
        let mut hm = HeartMap::new();
        hm.insert(HeartColor::Heart06, 1);
        hm.insert(HeartColor::Heart01, 2);
        let mods = GameModifiers::new();
        assert_eq!(build_heart_str(&hm, 1, &mods, false), "h012 h061");
    }
}
