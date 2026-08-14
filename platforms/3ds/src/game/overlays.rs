#![cfg(feature = "3ds")]

// Overlay system (START menu, game log, perf stats, revealed cards).
// Co-locates each overlay's input handling with its rendering; both were
// previously ~3,000 lines apart inside play_step (see engine_duplication.md 1.5).

use std::collections::HashMap;

use rabuka_engine::game_state::GameState;

use crate::ffi::*;
use crate::i18n;
use crate::lang::{current_lang, set_lang, tl};
use crate::steps::{Overlay, PerfTab};
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::grid::{card_grid_input, render_card_detail, render_card_grid, GridAction};
use crate::ui::hint::render_hint_bar;
use crate::ui::text::{heart_label_to_icon, SCALE_BODY, SCALE_LARGE, SCALE_SMALL};

/// Number of scrollable entries in a PerfTab for a given snapshot.
fn perf_tab_len(s: &rabuka_engine::types::PerformanceSnapshot, tab: PerfTab) -> usize {
    match tab {
        PerfTab::Overview | PerfTab::Hearts => 1,
        PerfTab::Live => s.lives.len(),
        PerfTab::Yell => s.yell_cards.len(),
        PerfTab::Contributions => s.member_contributions.len(),
        PerfTab::Triggered => s.triggered_abilities.len(),
    }
}

/// Build a compact row of heart texticons for non-zero entries, e.g.
/// `{{heart_01.png|h01}} 2 {{icon_all.png|ALL}} 1`.
fn hearts_row(hearts: &[u8]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (i, &count) in hearts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let label = if i == 7 {
            format!("{{{{icon_all.png|ALL}}}}{}", count)
        } else {
            heart_label_to_icon(&format!("h{:02}{}", i, count))
        };
        parts.push(label);
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" ")
    }
}

fn blade_str(n: u8) -> String {
    if n == 0 {
        "-".to_string()
    } else {
        format!("{{{{icon_blade.png|BLADE}}}}{}", n)
    }
}

fn notes_str(note: u8, draw: u8) -> String {
    let mut parts: Vec<String> = Vec::new();
    if note > 0 {
        parts.push(format!("{{{{icon_score.png|SCORE}}}}{}", note));
    }
    if draw > 0 {
        parts.push(format!("{{{{icon_draw.png|DRAW}}}}{}", draw));
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" ")
    }
}

fn card_display_name(gs: &GameState, cid: i16) -> String {
    gs.card_database
        .get_card(cid)
        .map(|c| c.card_no[..].to_string())
        .unwrap_or_else(|| "?".to_string())
}

/// Color + result text for a snapshot's overall status.
fn perf_outcome(s: &rabuka_engine::types::PerformanceSnapshot) -> (u32, &'static str) {
    if s.success {
        (0xFF88FF88, "PASS")
    } else {
        (0xFFFF8888, "FAIL")
    }
}


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
                        0 => Overlay::PerfStats(None, PerfTab::Overview, 0, 0),
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
            Overlay::PerfStats(ref mut si, ref mut tab, ref mut cursor, ref mut summary_cursor) => {
                let n = gs.performance_snapshots.len();
                let a = 0x00000001;
                let b = 0x00000002;
                let up = 0x00000040;
                let down = 0x00000080;
                let left = 0x00000020;
                let right = 0x00000010;
                if si.is_none() {
                    // Summary list of snapshots.
                    if keys & up != 0 && *cursor > 0 {
                        *cursor -= 1;
                        *redraw = true;
                    }
                    if keys & down != 0 && *cursor + 1 < n {
                        *cursor += 1;
                        *redraw = true;
                    }
                    if keys & a != 0 && n > 0 {
                        *summary_cursor = *cursor;
                        *si = Some(*cursor);
                        *tab = PerfTab::Overview;
                        *cursor = 0;
                        *redraw = true;
                    }
                    if keys & b != 0 {
                        *overlay = Overlay::None;
                        *redraw = true;
                    }
                } else {
                    // Tabbed view of one snapshot.
                    let idx = si.unwrap_or(0);
                    let s = &gs.performance_snapshots[idx];
                    let tabs = [
                        PerfTab::Overview,
                        PerfTab::Live,
                        PerfTab::Hearts,
                        PerfTab::Yell,
                        PerfTab::Contributions,
                        PerfTab::Triggered,
                    ];
                    if keys & left != 0 {
                        let pos = tabs.iter().position(|t| *t == *tab).unwrap_or(0);
                        *tab = tabs[(pos + tabs.len() - 1) % tabs.len()];
                        *cursor = 0;
                        *redraw = true;
                    }
                    if keys & right != 0 {
                        let pos = tabs.iter().position(|t| *t == *tab).unwrap_or(0);
                        *tab = tabs[(pos + 1) % tabs.len()];
                        *cursor = 0;
                        *redraw = true;
                    }
                    let max_items = perf_tab_len(s, *tab);
                    if keys & up != 0 && *cursor > 0 {
                        *cursor -= 1;
                        *redraw = true;
                    }
                    if keys & down != 0 && *cursor + 1 < max_items {
                        *cursor += 1;
                        *redraw = true;
                    }
                    if keys & b != 0 {
                        *si = None;
                        *cursor = *summary_cursor;
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
                SCALE_LARGE,
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
                    SCALE_BODY,
                    format!("{}{}\0", prefix, item).as_ptr(),
                );
            }
            render_hint_bar(&tl("UP/DOWN=move, A=select, B=close"));
        },
        Overlay::GameLog(offset, cursor) => {
            let logs = &gs.rule_log;
            let n = logs.len();
            unsafe {
                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                let log_hdr = tl("Game Log");
                _3ds_top_queue_text(
                    4.0,
                    2.0,
                    COL_GOLD,
                    SCALE_BODY,
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
                        SCALE_BODY,
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
                        SCALE_SMALL,
                        format!("{}-{} of {}\0", lo, hi, n).as_ptr(),
                    );
                }
            }
        }
        Overlay::PerfStats(si, tab, cursor, _summary_cursor) => {
            unsafe {
                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
            }
            let snapshots = &gs.performance_snapshots;
            if snapshots.is_empty() {
                let msg = tl("No performance data yet");
                unsafe {
                    _3ds_top_queue_text(
                        40.0,
                        60.0,
                        COL_MED,
                        SCALE_BODY,
                        format!("{}\0", msg).as_ptr(),
                    );
                }
            } else if let Some(idx) = si {
                let idx = idx.min(snapshots.len() - 1);
                let s = &snapshots[idx];
                // Header.
                unsafe {
                    _3ds_top_queue_text(
                        4.0,
                        2.0,
                        COL_GOLD,
                        SCALE_BODY,
                        format!(
                            "{}  T{} {}  (LEFT/RIGHT=tab, B=back)\0",
                            tl("Performance"),
                            s.turn,
                            s.player_id
                        )
                        .as_ptr(),
                    );
                    // Outcome pill.
                    let (out_col, out_txt) = perf_outcome(s);
                    _3ds_top_queue_text(
                        300.0,
                        2.0,
                        out_col,
                        SCALE_BODY,
                        format!("{}\0", tl(out_txt)).as_ptr(),
                    );
                }
                // Tab bar.
                let tab_labels = [
                    (PerfTab::Overview, "Overall"),
                    (PerfTab::Live, "Live"),
                    (PerfTab::Hearts, "Hearts"),
                    (PerfTab::Yell, "Yell"),
                    (PerfTab::Contributions, "Member"),
                    (PerfTab::Triggered, "Abils"),
                ];
                let mut tx = 4.0;
                for (t, label) in tab_labels {
                    let active = t == tab;
                    let bg = if active { 0xFF557755 } else { 0xFF333333 };
                    unsafe {
                        _3ds_top_queue_rect(tx, 20.0, 60.0, 16.0, bg);
                        _3ds_top_queue_text(
                            tx + 2.0,
                            22.0,
                            if active { COL_GOLD } else { COL_MED },
                            SCALE_SMALL,
                            format!("{}\0", label).as_ptr(),
                        );
                    }
                    tx += 62.0;
                }
                match tab {
                    PerfTab::Overview => unsafe {
                        let mut ly = 44.0;
                        let rows = [
                            (
                                format!("{{{{icon_score.png|S}}}}{} {}", tl("Live Pts:"), s.base_score_total),
                                0xFFCCCCCC,
                            ),
                            (
                                format!("{{{{icon_score.png|S}}}}{} {}", tl("Bonus:"), s.card_bonus_total),
                                0xFFCCCCCC,
                            ),
                            (
                                format!("{{{{icon_score.png|S}}}}{} {}", tl("Notes:"), s.note_icons),
                                0xFFCCCCCC,
                            ),
                            (
                                format!("{{{{icon_blade.png|B}}}}{} {}", tl("Yells:"), s.yell_count),
                                0xFFCCCCCC,
                            ),
                            (
                                format!("{{{{icon_score.png|S}}}}{} {}", tl("Total:"), s.total_score),
                                0xFFFFFF88,
                            ),
                            (
                                format!("{} {}", tl("Surplus:"), hearts_row(&s.surplus_hearts)),
                                0xFF88DDFF,
                            ),
                        ];
                        for (r, col) in rows {
                            _3ds_top_queue_text(8.0, ly, col, SCALE_BODY, format!("{}\0", r).as_ptr());
                            ly += 18.0;
                            if ly > 230.0 {
                                break;
                            }
                        }
                        // Pass/fail per live summarized.
                        let passed = s.lives.iter().filter(|l| l.passed).count();
                        _3ds_top_queue_text(
                            8.0,
                            ly,
                            if passed == s.lives.len() && !s.lives.is_empty() {
                                0xFF88FF88
                            } else {
                                0xFFFF8888
                            },
                            SCALE_BODY,
                            format!("{} {}/{} {}\0", tl("Lives:"), passed, s.lives.len(), tl("PASS")).as_ptr(),
                        );
                    },
                    PerfTab::Live => {
                        let mut ly = 42.0;
                        for (li, lc) in s.lives.iter().enumerate() {
                            let is_cur = li == cursor;
                            let cn = card_display_name(gs, lc.card_id);
                            let status = tl(if lc.passed { "PASS" } else { "FAIL" });
                            let bonus = lc.score.saturating_sub(lc.base_score);
                            let name_col = if is_cur { COL_GOLD } else { COL_LIGHT };
                            unsafe {
                                _3ds_top_queue_text(
                                    6.0,
                                    ly,
                                    name_col,
                                    SCALE_BODY,
                                    format!("{} {} {}+{}={}\0", cn, status, lc.base_score, bonus, lc.score)
                                        .as_ptr(),
                                );
                                ly += 15.0;
                                _3ds_top_queue_text(
                                    10.0,
                                    ly,
                                    COL_MED,
                                    SCALE_SMALL,
                                    format!(
                                        "{} {}  {} {}\0",
                                        tl("Need"),
                                        hearts_row(&lc.required),
                                        tl("Got"),
                                        hearts_row(&lc.filled)
                                    )
                                    .as_ptr(),
                                );
                            }
                            ly += 15.0;
                            for adj in &lc.adjustments {
                                if ly > 220.0 {
                                    break;
                                }
                                unsafe {
                                    _3ds_top_queue_text(
                                        10.0,
                                        ly,
                                        COL_MED,
                                        SCALE_SMALL,
                                        format!("  +{} {}\0", adj.value, adj.desc).as_ptr(),
                                    );
                                }
                                ly += 14.0;
                            }
                            if ly > 232.0 {
                                break;
                            }
                        }
                    }
                    PerfTab::Hearts => {
                        unsafe {
                            _3ds_top_queue_text(
                                6.0,
                                44.0,
                                COL_LIGHT,
                                SCALE_BODY,
                                format!("{}  {}\0", tl("Total"), hearts_row(&s.total_hearts)).as_ptr(),
                            );
                            _3ds_top_queue_text(
                                6.0,
                                62.0,
                                COL_MED,
                                SCALE_BODY,
                                format!("{} {}\0", tl("Surplus"), hearts_row(&s.surplus_hearts)).as_ptr(),
                            );
                            let passed = s.lives.iter().filter(|l| l.passed).count();
                            _3ds_top_queue_text(
                                6.0,
                                80.0,
                                COL_MED,
                                SCALE_BODY,
                                format!("{} {}/{}\0", tl("Passed:"), passed, s.lives.len()).as_ptr(),
                            );
                        }
                    }
                    PerfTab::Yell => {
                        let mut ly = 42.0;
                        for (yi, yc) in s.yell_cards.iter().enumerate() {
                            let is_cur = yi == cursor;
                            let cn = card_display_name(gs, yc.card_id);
                            unsafe {
                                _3ds_top_queue_text(
                                    6.0,
                                    ly,
                                    if is_cur { COL_GOLD } else { COL_LIGHT },
                                    SCALE_BODY,
                                    format!(
                                        "{} {} {}\0",
                                        cn,
                                        hearts_row(&yc.blade_hearts),
                                        notes_str(yc.note_icons, yc.draw_icons)
                                    )
                                    .as_ptr(),
                                );
                            }
                            ly += 16.0;
                            if ly > 232.0 {
                                break;
                            }
                        }
                    }
                    PerfTab::Contributions => {
                        let mut ly = 42.0;
                        for (mi, mc) in s.member_contributions.iter().enumerate() {
                            let is_cur = mi == cursor;
                            let cn = card_display_name(gs, mc.source_id);
                            let mut hearts = [0u8; 8];
                            for (c, v) in hearts.iter_mut().enumerate() {
                                *v = mc.base_hearts[c].saturating_add(mc.bonus_hearts[c]);
                            }
                            let blades = mc.base_blades.saturating_add(mc.bonus_blades);
                            unsafe {
                                _3ds_top_queue_text(
                                    6.0,
                                    ly,
                                    if is_cur { COL_GOLD } else { COL_LIGHT },
                                    SCALE_BODY,
                                    format!(
                                        "{} {} {}  {}\0",
                                        cn,
                                        hearts_row(&hearts),
                                        blade_str(blades),
                                        notes_str(mc.base_notes.saturating_add(mc.bonus_notes), mc.draw_icons)
                                    )
                                    .as_ptr(),
                                );
                            }
                            ly += 16.0;
                            if ly > 232.0 {
                                break;
                            }
                        }
                    }
                    PerfTab::Triggered => {
                        let mut ly = 42.0;
                        for (ti, ta) in s.triggered_abilities.iter().enumerate() {
                            let is_cur = ti == cursor;
                            let cn = if ta.card_name.is_empty() {
                                "?".to_string()
                            } else {
                                ta.card_name.to_string()
                            };
                            unsafe {
                                _3ds_top_queue_text(
                                    6.0,
                                    ly,
                                    if is_cur { COL_GOLD } else { COL_MED },
                                    SCALE_BODY,
                                    format!("{}  {}\0", cn, ta.name).as_ptr(),
                                );
                            }
                            ly += 16.0;
                            if ly > 232.0 {
                                break;
                            }
                        }
                    }
                }
            } else {
                // Summary list.
                let mut ly = 20.0;
                let max_vis = 11usize;
                let total = snapshots.len();
                let display_start = total.saturating_sub(cursor + 1);
                let display_end = display_start.saturating_sub(max_vis);
                for idx in (display_end..display_start).rev() {
                    if idx >= total {
                        continue;
                    }
                    let s = &snapshots[idx];
                    let is_cur = idx == cursor;
                    let pc = perf_outcome(s).0;
                    let label = format!(
                        "{} {} score:{} fate:{}  {}{}",
                        tl("T"),
                        s.turn,
                        s.total_score,
                        if s.success { tl("PASS") } else { tl("FAIL") },
                        notes_str(s.note_icons, 0),
                        blade_str(s.yell_count)
                    );
                    let base_col = if s.success { 0xFF88FF88 } else { 0xFFFF8888 };
                    let col = if is_cur { pc } else { base_col };
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
                            SCALE_BODY,
                            format!("{}\0", truncated).as_ptr(),
                        );
                    }
                    ly += 17.0;
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
                        SCALE_BODY,
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
                            SCALE_BODY,
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
                            SCALE_SMALL,
                            format!("{}\0", sec_text).as_ptr(),
                        );
                    }
                }
            }
        }
        Overlay::None => {}
    }
}
