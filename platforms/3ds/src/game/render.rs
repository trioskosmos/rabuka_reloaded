#![cfg(feature = "3ds")]

// Board rendering for play_step (CLI mode + graphical/image mode). Extracted
// from the Step::Play handler so the monolith shrinks (engine_duplication.md
// 1.5 render.rs). Read-only on game state; pagination clamps text_page and
// list_scroll, which are returned to the caller.

use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::player::Player;

use crate::ffi::*;
use crate::i18n;
use crate::i18n::Lang;
use crate::lang::{current_lang, tl};
use crate::net::mp_can_act;
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::grid::{draw_card_image, render_card_detail, render_card_grid};
use crate::ui::hint::render_hint_bar;
use crate::ui::text::*;
use crate::util::{cn_or_empty, tl_area};

use super::{action_list, compute_live_need, compute_total_hearts, find_card_zone_slot, pref};

/// Render the board (CLI or graphical/image mode). Never returns early; the
/// only side effects on locals are the text_page/list_scroll clamps, returned
/// as a tuple.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_board(
    gs: &GameState,
    ap: &Player,
    cur: usize,
    acts_cache: &[game_setup::Action],
    display_order: &[usize],
    display_pos: usize,
    cli_mode: bool,
    detail_mode: bool,
    choice_subview: bool,
    mut text_page: usize,
    choice_grid_offset: usize,
    mut list_scroll: usize,
    detail_scroll_y: f32,
    touch_tap_count: u32,
    viewing_card: Option<i16>,
    zone_viewer: &Option<(String, Vec<i16>)>,
    zone_viewer_offset: usize,
    my_player_idx: usize,
    has_image_choice: bool,
    has_text_choice: bool,
    is_multiplayer: bool,
    is_host: bool,
    vs_ai: &bool,
    ai_vs_ai: &bool,
    is_ai_turn: bool,
    atlas: &CardAtlas,
) -> (usize, usize) {
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
                            let display_name = i18n::card_display_name(&card.name, current_lang());
                            unsafe {
                                _3ds_text_add_top(
                                    format!("[{}] {}\n\0", card.card_no, display_name).as_ptr(),
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
                        }
                    }
                }
            }
            unsafe {
                _3ds_text_add_top("[X]=back Y=game\0".as_ptr());
            }
        } else {
            let ap_label = if ap.id == pref(gs, my_player_idx).id {
                "P1"
            } else {
                "P2"
            };
            let touch_indicator = if viewing_card.is_some() { "[T]" } else { "   " };
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
                _3ds_text_add_top(
                    format!(
                        "Me H:{} E:{}/{} D:{} W:{} L:{}  Opp H:{} E:{}/{} D:{} W:{} L:{}\n\0",
                        pref(gs, my_player_idx).hand.cards.len(),
                        pref(gs, my_player_idx).energy_zone.active_count(),
                        pref(gs, my_player_idx).energy_zone.cards.len(),
                        pref(gs, my_player_idx).main_deck.cards.len(),
                        pref(gs, my_player_idx).waitroom.cards.len(),
                        pref(gs, my_player_idx).success_live_card_zone.cards.len(),
                        pref(gs, 1 - my_player_idx).hand.cards.len(),
                        pref(gs, 1 - my_player_idx).energy_zone.active_count(),
                        pref(gs, 1 - my_player_idx).energy_zone.cards.len(),
                        pref(gs, 1 - my_player_idx).main_deck.cards.len(),
                        pref(gs, 1 - my_player_idx).waitroom.cards.len(),
                        pref(gs, 1 - my_player_idx)
                            .success_live_card_zone
                            .cards
                            .len(),
                    )
                    .as_ptr(),
                );
            }
            if let Some(vcid) = viewing_card {
                if let Some(card) = gs.card_database.get_card(vcid) {
                    let display_name = i18n::card_display_name(&card.name, current_lang());
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
                        let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
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
                    &i18n::translate_ability(&entry.ability.full_text, current_lang()),
                    390.0,
                    0.85,
                );
                for line in ab_text.lines() {
                    unsafe {
                        _3ds_text_add_top(format!("{}\n\0", line).as_ptr());
                    }
                }
            }
            let is_ai_turn = *ai_vs_ai || (*vs_ai && !mp_can_act(gs, 0));
            let is_opponent_turn_mp = is_multiplayer
                && !mp_can_act(
                    gs,
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
                        .min((n - max_vis) as isize) as usize
                } else {
                    0
                };
                let end = (start + max_vis).min(n);
                if start > 0 {
                    unsafe {
                        _3ds_text_add_top(format!("\u{25b2} +{}\n\0", start).as_ptr());
                    }
                }
                for di in start..end {
                    let fi = display_order[di];
                    let act = &acts_cache[fi];
                    let prefix = if fi == cur { ">" } else { " " };
                    let line =
                        action_list::format_action_line(act, current_lang() == Lang::Japanese);
                    let desc_full = wrap_text(&line, 390.0, 0.85);
                    for (li, l) in desc_full.lines().enumerate() {
                        if li == 0 {
                            unsafe {
                                _3ds_text_add_top(format!("{}{}\n\0", prefix, l).as_ptr());
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
                        _3ds_text_add_top(format!("\u{25bc} +{}\n\0", n - end).as_ptr());
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
                        let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
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
                _3ds_text_add_top(format!("[X]=detail Y=game {}\0", detail_hint).as_ptr());
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
                    if ap.id == pref(gs, my_player_idx).id {
                        "Me"
                    } else {
                        "Opp"
                    },
                    pref(gs, my_player_idx).hand.cards.len(),
                    pref(gs, my_player_idx).energy_zone.active_count(),
                    pref(gs, my_player_idx).energy_zone.cards.len(),
                    pref(gs, my_player_idx).main_deck.cards.len(),
                    pref(gs, 1 - my_player_idx).hand.cards.len(),
                    pref(gs, 1 - my_player_idx).energy_zone.active_count(),
                    pref(gs, 1 - my_player_idx).energy_zone.cards.len(),
                    pref(gs, 1 - my_player_idx).main_deck.cards.len(),
                )
                .as_ptr(),
            );
            let p1_blade: u32 = gs.player1.stage.total_blades(
                &gs.card_database,
                &gs.mods.blade_modifiers,
                &gs.mods.orientation_modifiers,
                false,
            ) as u32;
            let p2_blade: u32 = gs.player2.stage.total_blades(
                &gs.card_database,
                &gs.mods.blade_modifiers,
                &gs.mods.orientation_modifiers,
                false,
            ) as u32;
            // Compute total hearts per player from stage members
            // (mirrors display.rs player_to_display total_hearts logic)
            let p1_hearts = compute_total_hearts(&gs.player1, gs);
            let p2_hearts = compute_total_hearts(&gs.player2, gs);
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
                format!("{}  {{{{icon_blade.png|BLADE}}}}{}", p1_heart_str, p1_blade)
            };
            render_text_with_icons(4.0, 22.0, &p1_stats, COL_LIGHT, 0.55f32);
            // Render P2 hearts+blades on top screen line 3
            let p2_stats = if p2_heart_str.is_empty() {
                format!("BL:{}", p2_blade)
            } else {
                format!("{}  {{{{icon_blade.png|BLADE}}}}{}", p2_heart_str, p2_blade)
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
                // P1 (perspective) need hearts
                let p1_nh = compute_live_need(&gs.player1, gs);
                if p1_nh.iter().any(|&v| v > 0) {
                    let nh_str = format_hearts(&p1_nh);
                    let need_display = format!("{{{{icon_heart_06.png|NEED}}}} {}", nh_str);
                    render_text_with_icons(4.0, 46.0, &need_display, COL_GOLD, 0.50f32);
                }
                // P2 (opponent) need hearts — only after performed
                if gs.opponent_has_performed(my_player_idx) {
                    let p2_nh = compute_live_need(&gs.player2, gs);
                    if p2_nh.iter().any(|&v| v > 0) {
                        let nh_str = format_hearts(&p2_nh);
                        let need_display = format!("{{{{icon_heart_06.png|NEED}}}} {}", nh_str);
                        render_text_with_icons(4.0, 46.0, &need_display, COL_GOLD, 0.50f32);
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
                render_card_detail(viewing_card.unwrap(), &gs.card_database, atlas, detail_scroll_y);
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
                                let ab_text =
                                    i18n::translate_ability(&ab.full_text, current_lang());
                                let w = wrap_ability_text(&ab_text, 384.0, 0.65);
                                for l in w.lines() {
                                    all_lines.push(l.to_string());
                                }
                                all_lines.push(String::new());
                            }
                        }
                        let lpp = 10usize;
                        let total_pages = ((all_lines.len() + lpp - 1) / lpp).max(1);
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
                                    format!("{}/{}\0", text_page + 1, total_pages).as_ptr(),
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
                            let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
                            let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                            line_count += w.lines().count();
                        }
                        // If no abilities, use minimal height; otherwise expand panel
                        let text_h = 86.0
                            + line_count as f32 * 18.0
                            + (card.resolved_abilities().count().saturating_sub(1) as f32) * 3.0;
                        let min_h = 86.0 + 18.0; // at least one line
                        let panel_end = (text_h.max(min_h) + 8.0).min(232.0);
                        let _rect_h = panel_end - 52.0;

                        // Layout below the 50px game header: card portrait fills
                        // the left column, ability text in the right column.
                        let card_h = 240.0 - 56.0 - 10.0; // ~174px tall
                        let card_w = card_h * 0.711; // ~124px portrait
                        let card_x = 6.0;
                        let card_y = 56.0;
                        let text_x = card_x + card_w + 10.0; // ~140
                        let text_w = 400.0 - text_x - 8.0; // ~252

                        unsafe {
                            // Background for the detail area
                            _3ds_top_queue_rect(0.0, 52.0, 400.0, 188.0, COL_CARD);
                            // Card portrait (left column)
                            _3ds_top_queue_rect(
                                card_x - 2.0,
                                card_y - 2.0,
                                card_w + 4.0,
                                card_h + 4.0,
                                COL_GOLD,
                            );
                            draw_card_image(
                                &card.card_no,
                                atlas,
                                card_x,
                                card_y,
                                card_w,
                                card_h,
                            );
                            // Name + stats at top of right column
                            let display_name = i18n::card_display_name(&card.name, current_lang());
                            _3ds_top_queue_text(
                                text_x,
                                card_y - 2.0,
                                COL_BLUE,
                                0.80f32,
                                format!(
                                    "[{}] {}\0",
                                    card.card_no,
                                    wrap_text(&display_name, text_w, 0.80)
                                )
                                .as_ptr(),
                            );
                            let stats = compute_card_stats(card, cid, gs);
                            render_text_with_icons(
                                text_x,
                                card_y + 20.0,
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
                            // Scrollable ability text (right column)
                            let mut ty = card_y + 40.0 - detail_scroll_y;
                            for ab in card.resolved_abilities() {
                                let ab_text =
                                    i18n::translate_ability(&ab.full_text, current_lang());
                                let w = wrap_ability_text(&ab_text, text_w, 0.65);
                                for line in w.lines() {
                                    if ty > -20.0 && ty < 240.0 {
                                        render_text_with_icons(text_x, ty, line, COL_LIGHT, 0.65);
                                    }
                                    ty += 18.0;
                                }
                                ty += 3.0;
                            }
                            ability_end = ty;
                            // Scroll indicators (right edge)
                            let arrow_x = 400.0 - 18.0;
                            if ty > 228.0 {
                                _3ds_top_queue_text(
                                    arrow_x,
                                    228.0,
                                    COL_MED,
                                    0.50f32,
                                    format!("v\0").as_ptr(),
                                );
                            }
                            if detail_scroll_y > 0.0 {
                                _3ds_top_queue_text(
                                    arrow_x,
                                    56.0,
                                    COL_MED,
                                    0.50f32,
                                    format!("^\0").as_ptr(),
                                );
                            }
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
                        4.0,
                        2.0,
                        COL_GOLD,
                        0.65f32,
                        format!(
                            "T{} {} [{}]  Me H:{} E:{}/{} D:{}  Opp H:{} E:{}/{} D:{}\0",
                            gs.turn_number,
                            ph,
                            if ap.id == pref(gs, my_player_idx).id {
                                "Me"
                            } else {
                                "Opp"
                            },
                            pref(gs, my_player_idx).hand.cards.len(),
                            pref(gs, my_player_idx).energy_zone.active_count(),
                            pref(gs, my_player_idx).energy_zone.cards.len(),
                            pref(gs, my_player_idx).main_deck.cards.len(),
                            pref(gs, 1 - my_player_idx).hand.cards.len(),
                            pref(gs, 1 - my_player_idx).energy_zone.active_count(),
                            pref(gs, 1 - my_player_idx).energy_zone.cards.len(),
                            pref(gs, 1 - my_player_idx).main_deck.cards.len(),
                        )
                        .as_ptr(),
                    );
                }
            } // end else (not choice_subview)
        } else {
            if let Some(vcid) = viewing_card {
                // Compact card info overlay with stats
                if let Some(card) = gs.card_database.get_card(vcid) {
                    let stats = compute_card_stats(card, vcid, gs);
                    unsafe {
                        _3ds_top_queue_rect(0.0, 52.0, 400.0, 76.0, COL_CARD);
                        let btm_name = i18n::card_display_name(&card.name, current_lang());
                        _3ds_top_queue_text(
                            4.0,
                            44.0,
                            COL_BLUE,
                            0.75f32,
                            format!("[{}] {}\0", card.card_no, wrap_text(&btm_name, 392.0, 0.75))
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
                            let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
                            let first_line = wrap_ability_text(&ab_text, 392.0, 0.60)
                                .lines()
                                .next()
                                .unwrap_or("")
                                .to_string();
                            render_text_with_icons(4.0, 82.0, &first_line, COL_LIGHT, 0.60);
                        }
                    }
                }
                content_y = 126.0;
            } else if let Some(entry) = gs.ability_queue.current_entry() {
                // In image mode with choices, the text subview handles this.
                // The banner is only for CLI/text mode.
                if !(has_image_choice || has_text_choice) && !is_ai_turn {
                    let ab_text = i18n::translate_ability(&entry.ability.full_text, current_lang());
                    let ab_lines: Vec<String> = wrap_ability_text(&ab_text, 392.0, 0.65)
                        .lines()
                        .take(4)
                        .map(|l| l.to_string())
                        .collect();
                    let n_lines = ab_lines.len();
                    let h = 22.0 + n_lines as f32 * 14.0;
                    unsafe {
                        _3ds_top_queue_rect(0.0, 52.0, 400.0, h, COL_ABILITY);
                        render_text_with_icons(4.0, 54.0, &ab_lines[0], COL_LIGHT, 0.65);
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
            let is_ai_turn = *ai_vs_ai || (*vs_ai && !mp_can_act(gs, 0));
            let is_opponent_turn_mp = is_multiplayer
                && !mp_can_act(
                    gs,
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
                let is_auto_ability_choice = matches!(
                    gs.get_pending_choice(),
                    Some(rabuka_engine::ability::types::Choice::SelectAutoAbility { .. })
                );
                if is_auto_ability_choice
                    && !(detail_mode && viewing_card.is_some())
                    && !is_ai_turn
                    && !is_opponent_turn_mp
                {
                    // ===== Ability queue (SelectAutoAbility): vertical text
                    //      list, styled like the main-phase action list. Each
                    //      queued ability is a row: card-name header + full
                    //      ability text wrapped to multiple lines. =====
                    if let Some(c) = gs.get_pending_choice() {
                        use rabuka_engine::ability::types::Choice;
                        if let Choice::SelectAutoAbility {
                            options,
                            description,
                            description_en,
                            description_ja,
                            ..
                        } = c
                        {
                            // Choice prompt header (same slot as the old banner)
                            let desc = if current_lang() == Lang::Japanese {
                                description_ja.as_deref().unwrap_or(description).to_string()
                            } else {
                                description_en.as_deref().unwrap_or(description).to_string()
                            };
                            let desc_lines: Vec<String> = wrap_text(&desc, 392.0, 0.60)
                                .lines()
                                .map(|l| l.to_string())
                                .collect();
                            let header_h = 12.0 + desc_lines.len().min(2) as f32 * 14.0;
                            unsafe {
                                _3ds_top_queue_rect(0.0, content_y, 400.0, header_h, COL_ABILITY);
                            }
                            let mut oy = content_y + 3.0;
                            for line in desc_lines.iter().take(2) {
                                render_text_with_icons(4.0, oy, line, COL_GOLD, 0.60);
                                oy += 14.0;
                            }
                            let mut ty = content_y + header_h + 4.0;
                            let n = options.len();
                            let max_vis = ((230.0 - ty) / 20.0) as usize + 1;
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
                                let opt = &options[di];
                                let is_sel = di == display_pos;
                                let line_color = if is_sel { COL_GOLD } else { COL_LIGHT };
                                let prefix = if is_sel { ">" } else { " " };
                                let cn = opt
                                    .card_id
                                    .and_then(|cid| gs.card_database.get_card(cid))
                                    .map(|card| card.card_no.to_string())
                                    .unwrap_or_default();
                                let header = if cn.is_empty() {
                                    format!("{}{}", prefix, opt.card_name)
                                } else {
                                    format!("{}[{}] {}", prefix, cn, opt.card_name)
                                };
                                for l in wrap_text(&header, 392.0, 0.65).lines() {
                                    if ty > 230.0 {
                                        break;
                                    }
                                    render_text_with_icons(4.0, ty, l, line_color, 0.65);
                                    ty += 20.0;
                                }
                                let ab_text =
                                    i18n::translate_ability(&opt.ability_text, current_lang());
                                let ab_wrapped = wrap_ability_text(&ab_text, 392.0, 0.65);
                                for (li, l) in ab_wrapped.lines().enumerate() {
                                    if ty > 230.0 {
                                        break;
                                    }
                                    let txt = if li == 0 {
                                        format!("  {}", l)
                                    } else {
                                        l.to_string()
                                    };
                                    render_text_with_icons(4.0, ty, &txt, line_color, 0.65);
                                    ty += 20.0;
                                }
                                ty += 4.0;
                                di += 1;
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
                            render_hint_bar(&tl("UP/DOWN=select  A=confirm"));
                        }
                    }
                } else if (has_image_choice || has_text_choice)
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
                                p.card_id
                                    .and_then(|cid| opt_ability_texts.get(&cid).cloned())
                            })
                        });
                    let banner_text: String = hovered_ability_text
                        .or_else(|| {
                            gs.ability_queue.current_entry().map(|e| {
                                i18n::translate_ability(&e.ability.full_text, current_lang())
                            })
                        })
                        .unwrap_or_default();

                    // ---- Render ability banner first ----
                    let mut grid_iy: f32 = 52.0;
                    if !banner_text.is_empty() {
                        let ab_lines: Vec<String> = wrap_ability_text(&banner_text, 392.0, 0.60)
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
                    let cw = (max_ch * 0.711)
                        .min((400.0 - 8.0 - (cols as f32 - 1.0) * gap) / cols as f32);
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
                            .or_else(|| act.parameters.as_ref().and_then(|p| p.card_id));
                        if let Some(cid) = real_cid {
                            if let Some(cn) = gs
                                .card_database
                                .get_card(cid)
                                .map(|c| c.card_no.to_string())
                            {
                                if let Some((atl, idx)) = atlas.lookup(cn.as_str()) {
                                    let c_str =
                                        std::ffi::CString::new(atl.as_bytes()).unwrap_or_default();
                                    let border = if di == display_pos {
                                        COL_GOLD
                                    } else {
                                        COL_CARD
                                    };
                                    unsafe {
                                        _3ds_top_queue_rect(ix, iy_card, cw, ch + 16.0, border);
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
                                        let label = if act.action_type
                                            == game_setup::ActionType::PlayMemberToStage
                                        {
                                            let cost = act
                                                .parameters
                                                .as_ref()
                                                .and_then(|p| p.base_cost)
                                                .unwrap_or(0);
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
                    if let Some(&sel_gi) = text_gis.iter().find(|&&g| g == display_pos) {
                        let fi = display_order[sel_gi];
                        let act = &acts_cache[fi];
                        let is_disabled = act
                            .parameters
                            .as_ref()
                            .and_then(|p| p.disabled)
                            .unwrap_or(false);
                        let desc = act.display_desc(current_lang() == Lang::Japanese);
                        let desc_nlb = desc.replace('\n', " ");
                        let desc_clean = desc_nlb
                            .trim_start_matches(|c: char| c == '・' || c == '\u{2022}')
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
                            _3ds_top_queue_rect(4.0, iy - 2.0, 392.0, total_h + 4.0, COL_DIM);
                            render_text_with_icons(8.0, iy + 2.0, &full_txt, color, scale);
                        }
                        // Page indicator
                        let total = text_gis.len();
                        if total > 1 {
                            let cur =
                                text_gis.iter().position(|&g| g == display_pos).unwrap_or(0) + 1;
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
                                format!("{}\0", format!("{}/{}", pg, total_p)).as_ptr(),
                            );
                        }
                    }
                    // Text overlay on top of choices grid
                    if choice_subview {
                        if let Some(entry) = gs.ability_queue.current_entry() {
                            let ab_lines: Vec<String> =
                                wrap_ability_text(&entry.ability.full_text, 384.0, 0.65)
                                    .lines()
                                    .map(|l| l.to_string())
                                    .collect();
                            let lpp = 7usize;
                            let total_pages = ((ab_lines.len() + lpp - 1) / lpp).max(1);
                            if text_page >= total_pages {
                                text_page = total_pages - 1;
                            }
                            let start_line = text_page * lpp;
                            let end_line = (start_line + lpp).min(ab_lines.len());
                            unsafe {
                                _3ds_top_queue_rect(0.0, 52.0, 400.0, 198.0, 0xCC000000);
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
                                render_text_with_icons(8.0, oy, &ab_lines[i], COL_LIGHT, 0.65);
                                oy += 20.0;
                            }
                            let page_str = format!("{}/{}", text_page + 1, total_pages);
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
                        let is_pmts = act.action_type == game_setup::ActionType::PlayMemberToStage;
                        let mut ge = di + 1;
                        if is_pmts && this_cid != -1 {
                            while ge < end {
                                let n = &acts_cache[display_order[ge]];
                                if n.action_type == game_setup::ActionType::PlayMemberToStage
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
                        let group_sel = is_group && (di..ge).any(|i| i == display_pos);
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
                                    format!("{{{{icon_energy.png|E}}}}{} {}", base_cost, name)
                                } else {
                                    name.clone()
                                }
                            };
                            let mut areas = String::new();
                            let area_costs: std::collections::HashMap<String, (u8, bool)> =
                                if let Some(ref p) = acts_cache[display_order[di]].parameters {
                                    p.available_areas
                                        .as_ref()
                                        .map(|areas_vec| {
                                            areas_vec
                                                .iter()
                                                .map(|a| {
                                                    (a.area.clone(), (a.cost, a.is_baton_touch))
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
                                let prefix = if i == display_pos { "[" } else { "" };
                                let suffix = if i == display_pos { "]" } else { "" };
                                // For double baton pairs: dest+source(s)
                                // Double baton desc format: "Card (src1+src2)→dst cost:N"
                                // Regular desc format: "Card → dst (cost:N)"
                                // Only parse if ( comes before →
                                let desc = gact.display_desc(current_lang() == Lang::Japanese);
                                if let Some(paren_pos) = desc.find('(') {
                                    let arrow_pos = desc.find('→');
                                    if arrow_pos.map_or(true, |a| paren_pos < a) {
                                        // Double baton: extract sources from (src1+src2)
                                        if let Some(end) = desc[paren_pos..].find(')') {
                                            let sources: String = desc
                                                [paren_pos + 1..paren_pos + end]
                                                .split('+')
                                                .map(|a| a.trim())
                                                .filter(|a| !a.eq_ignore_ascii_case(&stage))
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
                                    _ => format!("{}{}{}", prefix, tl_area(&stage), suffix),
                                };
                                areas.push_str(&area_str);
                            }
                            let hdr_prefix = "";
                            for (_li, l) in wrap_text(&hdr, 370.0, line_scale).lines().enumerate() {
                                if ty > 230.0 {
                                    break;
                                }
                                let txt = format!("{}{}", hdr_prefix, l);
                                if txt.contains("{{") {
                                    render_text_with_icons(4.0, ty, &txt, line_color, line_scale);
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
                            for (_li, l) in wrap_text(&areas, 370.0, line_scale).lines().enumerate()
                            {
                                if ty > 230.0 {
                                    break;
                                }
                                let txt = format!("{}{}", areas_prefix, l);
                                if txt.contains("{{") {
                                    render_text_with_icons(4.0, ty, &txt, line_color, line_scale);
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
                            let line = super::action_list::format_action_line_image(act, gs);
                            let color = if is_disabled {
                                COL_MED
                            } else if is_sel {
                                COL_GOLD
                            } else {
                                COL_LIGHT
                            };
                            let scale: f32 = 0.65;
                            let wrap_w = if !prefix.is_empty() { 370.0 } else { 392.0 };
                            for (_li, l) in wrap_text(&line, wrap_w, scale).lines().enumerate() {
                                if ty > 230.0 {
                                    break;
                                }
                                let txt = format!("{}{}", prefix, l);
                                if txt.contains("{{") {
                                    render_text_with_icons(4.0, ty, &txt, color, scale);
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
            let ai_turn = *ai_vs_ai || (*vs_ai && !mp_can_act(gs, 0));
            let opp_turn = is_multiplayer
                && !mp_can_act(
                    gs,
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
                for act in acts_cache {
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
                                        _3ds_board_set_action_highlight(1, slot, false);
                                    }
                                }
                            } else {
                                // Normal mode: highlight the hand card that can be played
                                if let Some(cid) = p.card_id {
                                    if let Some((zone, slot, opp)) =
                                        find_card_zone_slot(gs, cid, my_player_idx)
                                    {
                                        if zone == 3 {
                                            unsafe {
                                                _3ds_board_set_action_highlight(zone, slot, opp);
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
                                    find_card_zone_slot(gs, cid, my_player_idx)
                                {
                                    unsafe {
                                        _3ds_board_set_action_highlight(zone, slot, opp);
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
                                        _3ds_board_set_action_highlight(1, slot, false);
                                    }
                                }
                            }
                        }
                        // Hand cards for SelectMulligan — only highlight if selected
                        game_setup::ActionType::SelectMulligan => {
                            if act.selected == Some(true) {
                                if let Some(hidx) = p.card_indices.as_ref().and_then(|v| v.first())
                                {
                                    unsafe {
                                        _3ds_board_set_action_highlight(3, *hidx as i32, false);
                                    }
                                }
                            }
                        }
                        // Hand cards for SelectLiveCard — only highlight if selected
                        game_setup::ActionType::SelectLiveCard => {
                            if act.selected == Some(true) {
                                if let Some(hidx) = p.card_indices.as_ref().and_then(|v| v.first())
                                {
                                    unsafe {
                                        _3ds_board_set_action_highlight(3, *hidx as i32, false);
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
                                        find_card_zone_slot(gs, cid, my_player_idx)
                                    {
                                        unsafe {
                                            _3ds_board_set_action_highlight(zone, slot, opp);
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
        if !(*ai_vs_ai || (*vs_ai && !mp_can_act(gs, 0)))
            && !(is_multiplayer
                && !mp_can_act(
                    gs,
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
                                find_card_zone_slot(gs, cid, my_player_idx)
                            {
                                unsafe {
                                    _3ds_board_set_action_highlight(zone, slot, opp);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (text_page, list_scroll)
}
