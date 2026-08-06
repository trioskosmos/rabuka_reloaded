#![cfg(feature = "3ds")]

// Overlay system (START menu, game log, perf stats, revealed cards).
// Co-locates each overlay's input handling with its rendering; both were
// previously ~3,000 lines apart inside play_step (see engine_duplication.md 1.5).

use std::collections::HashMap;

use rabuka_engine::game_state::GameState;

use crate::ffi::*;
use crate::i18n;
use crate::lang::{current_lang, set_lang, tl};
use crate::steps::Overlay;
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::grid::{card_grid_input, render_card_detail, render_card_grid, GridAction};
use crate::ui::hint::render_hint_bar;

/// Overlay input handling. Mutates `overlay` and `redraw` based on `keys`.
/// No-op when `*overlay == Overlay::None`.
pub(crate) fn overlay_input(
    overlay: &mut Overlay,
    gs: &GameState,
    keys: u32,
    is_host: bool,
    redraw: &mut bool,
) {
    if *overlay != Overlay::None {
        match *overlay {
            Overlay::StartMenu(ref mut sel) => {
                if keys & 0x00000040 != 0 {
                    *sel = sel.saturating_sub(1);
                    *redraw = true;
                }
                if keys & 0x00000080 != 0 {
                    *sel = sel.saturating_add(1).min(3);
                    *redraw = true;
                }
                if keys & 0x00000001 != 0 {
                    *overlay = match *sel {
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
                    *redraw = true;
                }
                if keys & 0x00000002 != 0 {
                    *overlay = Overlay::None;
                    *redraw = true;
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
                        *redraw = true;
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
                        *redraw = true;
                    }
                    if keys & 0x00000020 != 0 {
                        *offset = offset.saturating_sub(max_vis);
                        if *cursor >= *offset + max_vis || *cursor < *offset {
                            *cursor = *offset;
                        }
                        *redraw = true;
                    }
                    if keys & 0x00000010 != 0 {
                        let max_off = n.saturating_sub(max_vis);
                        *offset = offset.saturating_add(max_vis).min(max_off);
                        if *cursor >= *offset + max_vis || *cursor < *offset {
                            *cursor = *offset;
                        }
                        *redraw = true;
                    }
                }
            }
            Overlay::PerfStats(ref mut detail, ref mut cursor) => {
                let n = gs.performance_snapshots.len();
                if detail.is_some() {
                    if keys & 0x00000002 != 0 {
                        *detail = None;
                        *redraw = true;
                    }
                } else {
                    if keys & 0x00000040 != 0 && *cursor > 0 {
                        *cursor -= 1;
                        *redraw = true;
                    }
                    if keys & 0x00000080 != 0 && *cursor + 1 < n {
                        *cursor += 1;
                        *redraw = true;
                    }
                    if keys & 0x00000001 != 0 && n > 0 {
                        *detail = Some(*cursor);
                        *redraw = true;
                    }
                }
            }
            Overlay::RevealedCards(ref mut show_self, ref mut cursor, ref mut view_card) => {
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
                    *redraw = true;
                } else {
                    let action = card_grid_input(keys, cursor, view_card, &flat, 5);
                    match action {
                        GridAction::CloseGrid => {
                            *overlay = Overlay::None;
                        }
                        _ => {}
                    }
                    if !matches!(action, GridAction::None) {
                        *redraw = true;
                    }
                }
            }
            Overlay::None => {}
        }
    }
}

/// Overlay rendering (top screen). Caller guards with `zone_viewer.is_none()`.
pub(crate) fn render_overlay(gs: &GameState, overlay: Overlay, is_host: bool, atlas: &CardAtlas) {
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
                    format!("{}  {} entries (B=close, UP/DOWN=scroll)\0", log_hdr, n).as_ptr(),
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
                    format!("{}  (B=close, A=detail, UP/DOWN=select)\0", perf_hdr).as_ptr(),
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
                        let status = tl(if lc.passed { "PASS" } else { "FAIL" });
                        unsafe {
                            _3ds_top_queue_text(
                                8.0,
                                ly,
                                if lc.passed { 0xFF88FF88 } else { 0xFFFF8888 },
                                0.60f32,
                                format!("{} #{} {} score:{}\0", cn, li, status, lc.score).as_ptr(),
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
                        s.total_hearts.iter().copied().map(u32::from).sum::<u32>(),
                        s.lives.iter().filter(|l| l.passed).count(),
                        s.lives.len(),
                        s.success
                    );
                    let base_col = if s.success { 0xFF88FF88 } else { 0xFFFF8888 };
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
                render_card_detail(vcid, &gs.card_database, atlas, 0.0);
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
                let total_cards: usize = sections.iter().map(|s| s.cards.len()).sum();
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
