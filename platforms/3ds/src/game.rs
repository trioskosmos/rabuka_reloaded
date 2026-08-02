#![cfg(feature = "3ds")]
// Play state machine (Phase C): the Step::Play handler, moved verbatim from the
// bin (see extract_play.py). PlayState replaces the old 32-field tuple.

use std::collections::HashMap;

use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

use crate::dprintln;
use crate::ffi::*;
use crate::i18n;
use crate::i18n::Lang;
use crate::lang::{current_lang, set_lang, tl};
use crate::net::{execute_received_action, mp_can_act, route_authoritative_action};
use crate::steps::{Overlay, Step};
use crate::uds;
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::grid::{card_grid_input, render_card_detail, render_card_grid, GridAction};
use crate::ui::hint::render_hint_bar;
use crate::ui::text::*;
use crate::util::{cn_or_empty, heart_color_index, tl_area};

/// Full gameplay state carried by `Step::Play`.
#[derive(Clone)]
pub struct PlayState {
    pub gs: GameState,
    pub cur: usize,
    pub acts_cache: Vec<game_setup::Action>,
    pub dirty: bool,
    pub redraw: bool,
    pub atlas: CardAtlas,
    pub vs_ai: bool,
    pub ai_vs_ai: bool,
    pub cli_mode: bool,
    pub detail_mode: bool,
    pub choice_image_mode: bool,
    pub choice_subview: bool,
    pub text_page: usize,
    pub choice_grid_offset: usize,
    pub list_scroll: usize,
    pub detail_scroll_y: f32,
    pub hand_offset: usize,
    pub hand_offset_p2: usize,
    pub touch_tap_count: u32,
    pub viewing_card: Option<i16>,
    pub zone_viewer: Option<(String, Vec<i16>)>,
    pub zone_viewer_offset: usize,
    pub was_touching: bool,
    pub is_multiplayer: bool,
    pub is_host: bool,
    pub waiting_for_opponent: bool,
    pub overlay: Overlay,
    pub pending_client_action: Option<Vec<u8>>,
    pub last_client_action_seq: u32,
    pub next_action_seq: u32,
    pub dbg_tx_bytes: u32,
    pub dbg_rx_bytes: u32,
}

/// Build the compact single-line action description for the CLI action list.
/// Single source of truth for the PlayMemberToStage / UseAbility description
/// building that was previously duplicated inline (the "two sources of truth"
/// smell flagged in the Phase C plan).
fn format_action_line(act: &game_setup::Action, is_ja: bool) -> String {
    match act.action_type {
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
                format!("[{}] E{} {} {}", cn, cost, name, area_label)
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
            let abil_short: String = abil.chars().take(36).collect();
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
            .display_desc(is_ja)
            .lines()
            .next()
            .unwrap_or("")
            .to_string(),
    }
}

pub fn play_step(p: PlayState, keys: u32) -> Step {
    let PlayState {
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
        mut pending_client_action,
        mut last_client_action_seq,
        mut next_action_seq,
        mut dbg_tx_bytes,
        mut dbg_rx_bytes,
    } = p;
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
    let _my_id: i32 = my_player_idx as i32;
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
    if has_image_choice && !detail_mode && zone_viewer.is_none() && overlay == Overlay::None {
        if choice_subview {
            // === Text overlay: L/B dismiss, UP/DOWN page through text ===
            if keys & 0x00000200 != 0 || keys & 0x00000002 != 0 {
                choice_subview = false;
                redraw = true;
            }
            if let Some(entry) = gs.ability_queue.current_entry() {
                if keys & 0x00000040 != 0 || keys & 0x00000080 != 0 {
                    let ab_text = i18n::translate_ability(&entry.ability.full_text, current_lang());
                    let ab_lines: Vec<String> = wrap_ability_text(&ab_text, 384.0, 0.65)
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
                if let Some(cid) = acts_cache[cur].parameters.as_ref().and_then(|p| p.card_id) {
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
        let action = card_grid_input(keys, &mut zone_viewer_offset, &mut viewing_card, cards, 5);
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

    // Multiplayer: both consoles run the SAME engine. The only thing that
    // travels is the acting player's choice (~20 bytes), so no full-state
    // sync is ever needed. Each console executes every action locally —
    // its own (via input below) and the opponent's (here) — and settles
    // automatic phases itself, keeping the two GameStates identical.
    if is_multiplayer {
        let mut recv_buf = [0u8; 512];
        for _drain in 0..16 {
            if let Ok(n) = uds::uds_recv(&mut recv_buf) {
                if n > 0 {
                    dbg_rx_bytes += n as u32;
                    if recv_buf[0] == uds::MSG_SYNC_ACTION_ACK {
                        // Opponent processed my action — stop retransmitting it.
                        if let Some(ack_seq) = uds::parse_action_ack(&recv_buf[..n]) {
                            if let Some(bytes) = &pending_client_action {
                                if let Some(sync) = uds::ActionSync::from_bytes(bytes) {
                                    if sync.action_seq == ack_seq {
                                        pending_client_action = None;
                                    }
                                }
                            }
                        }
                    } else if recv_buf[0] == uds::MSG_SYNC_ACTION {
                        // Opponent's choice: dedup retransmits, then execute
                        // locally exactly as the opponent's console did.
                        if let Some(sync) = uds::ActionSync::from_bytes(&recv_buf[..n]) {
                            let is_dup =
                                sync.action_seq != 0 && sync.action_seq <= last_client_action_seq;
                            if !is_dup {
                                if sync.action_seq != 0 {
                                    last_client_action_seq = sync.action_seq;
                                }
                                execute_received_action(&mut gs, &sync);
                                waiting_for_opponent =
                                    !mp_can_act(&gs, if is_host { 0 } else { 1 });
                                cur = 0;
                                dirty = true;
                                redraw = true;
                            }
                            // ACK so the sender stops retransmitting.
                            let _ = uds::uds_send(&uds::action_ack(sync.action_seq));
                            dbg_tx_bytes += 5;
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
    // Retransmit my pending action until the opponent ACKs it. UDS is
    // unreliable; without this a dropped action would deadlock both
    // consoles. Turn-based means only one action is in flight at a time.
    if is_multiplayer {
        if let Some(bytes) = &pending_client_action {
            let _ = uds::uds_send(bytes);
            dbg_tx_bytes += bytes.len() as u32;
        }
    }
    // Recompute whose turn it is every frame from the local engine copy
    // (both consoles compute this identically). This also covers the very
    // first frame, where the initial waiting flag is not yet meaningful.
    if is_multiplayer {
        let my_id = if is_host { 0 } else { 1 };
        waiting_for_opponent = !mp_can_act(&gs, my_id);
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

    // Automatic phases settle on BOTH consoles (they run identical
    // engines), so the two GameStates advance together. In single-player
    // this also settles, exactly as before.
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
                            if act.action_type != game_setup::ActionType::SelectMulligan {
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
                        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
                    )
                {
                    if let Some(hidx) = tapped_hand_idx {
                        for (ai, act) in acts_cache.iter().enumerate() {
                            if act.action_type != game_setup::ActionType::SelectLiveCard {
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
                    let mut act_idx: Option<usize> = acts_cache.iter().position(|act| {
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
                                    act.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
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
                                act.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
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
                    let _card_at_slot = if slot_idx < 3 {
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
                        if act.action_type != game_setup::ActionType::PlayMemberToStage {
                            continue;
                        }
                        let p = match &act.parameters {
                            Some(x) => x,
                            None => continue,
                        };
                        if p.disabled.unwrap_or(false) {
                            continue;
                        }
                        if p.stage_area.as_ref().map(|s| s.as_str()) != Some(sa.as_str()) {
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
                        if p.stage_area.as_ref().map(|s| s.as_str()) != Some(sa.as_str()) {
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
        let _card_no = |cid: i16| -> Option<String> {
            gs.card_database
                .get_card(cid)
                .map(|c| c.card_no.to_string())
        };
        let is_tapped = |cid: i16| -> bool {
            gs.mods.orientation_modifiers.get(&cid).map(|o| o.as_str()) == Some("wait")
        };
        let set_slot = |slot_fn: unsafe extern "C" fn(i32, bool, *const u8, i32, bool, bool),
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
                    let c_str = std::ffi::CString::new(atl.as_bytes()).unwrap_or_default();
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
                            $live_fn(i as i32, false, std::ptr::null(), 0, false, false);
                        }
                        continue;
                    }
                    if live_hidden {
                        // Show the card back so presence is visible but not identity.
                        // Pass tapped=true so the C renderer rotates it 90° to fill
                        // the landscape live slot.
                        let back =
                            std::ffi::CString::new("icon_lltcg-back.png.t3x").unwrap_or_default();
                        unsafe {
                            $live_fn(i as i32, true, back.as_ptr() as *const u8, 0, true, true);
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
                    _3ds_board_set_hand_scroll_info(vis as i32, $hoff as i32, hc.len() as i32);
                }
                for i in 0..vis {
                    let idx = $hoff + i;
                    if idx < hc.len() {
                        set_slot($hand_fn, i as i32, hc[idx], false, false);
                    } else {
                        unsafe {
                            $hand_fn(i as i32, false, std::ptr::null(), 0, false, false);
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
                    _3ds_board_set_opp_hand(i, false, std::ptr::null(), 0, false, false);
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
            let opp_performed = matches!(
                gs.current_phase,
                Phase::SecondAttackerPerformance | Phase::LiveVictoryDetermination
            ) || (matches!(gs.current_phase, Phase::FirstAttackerPerformance)
                && opp_is_first);
            if !opp_performed {
                unsafe {
                    for i in 0..3i32 {
                        _3ds_board_set_opp_live(i, false, std::ptr::null(), 0, false, false);
                    }
                }
            }
        }

        // Set per-card live stats on the C board
        {
            let set_live_stats = |player: &Player, gs: &GameState, is_opp: bool| {
                for (i, &cid) in player.live_card_zone.cards.iter().enumerate().take(3) {
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
                        let c_line =
                            std::ffi::CString::new(stat_line.as_bytes()).unwrap_or_default();
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
                    0, p1_nh[0], p1_nh[1], p1_nh[2], p1_nh[3], p1_nh[4], p1_nh[5], p1_nh[6],
                    p1_nh[7],
                );
            }
            // P2 (opponent) need hearts — hidden until performed
            let opp_is_first = gs.player2.is_first_attacker;
            let opp_performed = matches!(
                gs.current_phase,
                Phase::SecondAttackerPerformance | Phase::LiveVictoryDetermination
            ) || (matches!(gs.current_phase, Phase::FirstAttackerPerformance)
                && opp_is_first);
            if opp_performed {
                let p2_nh = compute_live_need(&gs.player2, &gs);
                unsafe {
                    _3ds_set_need_hearts(
                        1, p2_nh[0], p2_nh[1], p2_nh[2], p2_nh[3], p2_nh[4], p2_nh[5], p2_nh[6],
                        p2_nh[7],
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
                let ap_label = if ap.id == pref(&gs, my_player_idx).id {
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
                            pref(&gs, my_player_idx).hand.cards.len(),
                            pref(&gs, my_player_idx).energy_zone.active_count(),
                            pref(&gs, my_player_idx).energy_zone.cards.len(),
                            pref(&gs, my_player_idx).main_deck.cards.len(),
                            pref(&gs, my_player_idx).waitroom.cards.len(),
                            pref(&gs, my_player_idx).success_live_card_zone.cards.len(),
                            pref(&gs, 1 - my_player_idx).hand.cards.len(),
                            pref(&gs, 1 - my_player_idx).energy_zone.active_count(),
                            pref(&gs, 1 - my_player_idx).energy_zone.cards.len(),
                            pref(&gs, 1 - my_player_idx).main_deck.cards.len(),
                            pref(&gs, 1 - my_player_idx).waitroom.cards.len(),
                            pref(&gs, 1 - my_player_idx)
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
                        let line = format_action_line(act, current_lang() == Lang::Japanese);
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
                            let h_mult = gs.mods.heart_color_multiplier.get(&cid).copied();
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
                            p1_hearts[idx] = (p1_hearts[idx] as i32 + val.total()).max(0) as u32;
                        }
                    }
                }
                for &cid in &gs.player2.stage.stage {
                    if cid == -1 {
                        continue;
                    }
                    if let Some(card) = gs.card_database.get_card(cid) {
                        if let Some(ref base_heart) = card.base_heart {
                            let h_mult = gs.mods.heart_color_multiplier.get(&cid).copied();
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
                            p2_hearts[idx] = (p2_hearts[idx] as i32 + val.total()).max(0) as u32;
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
                    // Compute live_need_hearts from live zone cards
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
                    let opp_is_first = gs.player2.is_first_attacker;
                    let opp_performed =
                        matches!(
                            gs.current_phase,
                            Phase::SecondAttackerPerformance | Phase::LiveVictoryDetermination
                        ) || (matches!(gs.current_phase, Phase::FirstAttackerPerformance)
                            && opp_is_first);
                    // P1 (perspective) need hearts
                    let p1_nh = compute_live_need(&gs.player1, &gs);
                    if p1_nh.iter().any(|&v| v > 0) {
                        let nh_str = format_hearts(&p1_nh);
                        let need_display = format!("{{{{icon_heart_06.png|NEED}}}} {}", nh_str);
                        render_text_with_icons(4.0, 46.0, &need_display, COL_GOLD, 0.50f32);
                    }
                    // P2 (opponent) need hearts — only after performed
                    if opp_performed {
                        let p2_nh = compute_live_need(&gs.player2, &gs);
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
                    render_card_detail(viewing_card.unwrap(), &gs.card_database, detail_scroll_y);
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
                                let ab_text =
                                    i18n::translate_ability(&ab.full_text, current_lang());
                                let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                                line_count += w.lines().count();
                            }
                            // If no abilities, use minimal height; otherwise expand panel
                            let text_h = 86.0
                                + line_count as f32 * 18.0
                                + (card.resolved_abilities().count().saturating_sub(1) as f32)
                                    * 3.0;
                            let min_h = 86.0 + 18.0; // at least one line
                            let panel_end = (text_h.max(min_h) + 8.0).min(232.0);
                            let _rect_h = panel_end - 52.0;

                            unsafe {
                                // Background for scrollable area
                                _3ds_top_queue_rect(0.0, 52.0, 400.0, 188.0, COL_CARD);
                                // Scrollable ability text
                                let mut ty = 86.0 - detail_scroll_y;
                                for ab in card.resolved_abilities() {
                                    let ab_text =
                                        i18n::translate_ability(&ab.full_text, current_lang());
                                    let w = wrap_ability_text(&ab_text, 392.0, 0.65);
                                    for line in w.lines() {
                                        if ty > -20.0 && ty < 240.0 {
                                            render_text_with_icons(4.0, ty, line, COL_LIGHT, 0.65);
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
                            4.0,
                            2.0,
                            COL_GOLD,
                            0.65f32,
                            format!(
                                "T{} {} [{}]  Me H:{} E:{}/{} D:{}  Opp H:{} E:{}/{} D:{}\0",
                                gs.turn_number,
                                ph,
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
                    }
                } // end else (not choice_subview)
            } else {
                if let Some(vcid) = viewing_card {
                    // Compact card info overlay with stats
                    if let Some(card) = gs.card_database.get_card(vcid) {
                        let stats = compute_card_stats(card, vcid, &gs);
                        unsafe {
                            _3ds_top_queue_rect(0.0, 52.0, 400.0, 76.0, COL_CARD);
                            let btm_name = i18n::card_display_name(&card.name, current_lang());
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
                                let ab_text =
                                    i18n::translate_ability(&ab.full_text, current_lang());
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
                        let ab_text =
                            i18n::translate_ability(&entry.ability.full_text, current_lang());
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
                                    _3ds_top_queue_rect(
                                        0.0,
                                        content_y,
                                        400.0,
                                        header_h,
                                        COL_ABILITY,
                                    );
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
                                        let c_str = std::ffi::CString::new(atl.as_bytes())
                                            .unwrap_or_default();
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
                                    text_gis.iter().position(|&g| g == display_pos).unwrap_or(0)
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
                            let is_pmts =
                                act.action_type == game_setup::ActionType::PlayMemberToStage;
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
                                for (_li, l) in
                                    wrap_text(&hdr, 370.0, line_scale).lines().enumerate()
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
                                for (_li, l) in
                                    wrap_text(&areas, 370.0, line_scale).lines().enumerate()
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
                                                format!("[{}] {} {}", cn, name, area_label)
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
                                        let abil_short = truncate_aware_segments(&abil, 28);
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
                                                format!("{} {} {}", name, area_label, abil_short)
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
                                                .display_desc(current_lang() == Lang::Japanese)
                                                .to_string();
                                            let ability_text = if act.action_type
                                                == game_setup::ActionType::ChoiceOption
                                            {
                                                gs.get_pending_choice()
                                                    .and_then(|c| {
                                                        use rabuka_engine::ability::types::Choice;
                                                        if let Choice::SelectAutoAbility {
                                                            options,
                                                            ..
                                                        } = c
                                                        {
                                                            act.parameters
                                                                .as_ref()
                                                                .and_then(|p| p.card_id)
                                                                .and_then(|idx| {
                                                                    options.get(idx as usize)
                                                                })
                                                                .map(|o| o.ability_text.clone())
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                    .unwrap_or_default()
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
                                let wrap_w = if !prefix.is_empty() { 370.0 } else { 392.0 };
                                for (_li, l) in wrap_text(&line, wrap_w, scale).lines().enumerate()
                                {
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
                                            _3ds_board_set_action_highlight(1, slot, false);
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
                                    if let Some(hidx) =
                                        p.card_indices.as_ref().and_then(|v| v.first())
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
                                    if let Some(hidx) =
                                        p.card_indices.as_ref().and_then(|v| v.first())
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
                                            find_card_zone_slot(&gs, cid, my_player_idx)
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
                                        _3ds_board_set_action_highlight(zone, slot, opp);
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
                        "MP|tx={} rx={} ap={} my={} can={} wait={} phase={} acts={}\0",
                        dbg_tx_bytes,
                        dbg_rx_bytes,
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
                            format!("{}  {} entries (B=close, UP/DOWN=scroll)\0", log_hdr, n)
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
                                        format!("{} #{} {} score:{}\0", cn, li, status, lc.score)
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

        dirty = false;
        redraw = false;
    }
    Step::Play(PlayState {
        gs,
        cur,
        acts_cache,
        dirty,
        redraw,
        atlas: atlas.clone(),
        vs_ai: *vs_ai,
        ai_vs_ai: *ai_vs_ai,
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
        pending_client_action,
        last_client_action_seq,
        next_action_seq,
        dbg_tx_bytes,
        dbg_rx_bytes,
    })
}

/// Locate a card's zone slot (zone, index, is-opponent) for board rendering.
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

fn visible_hand_slots() -> usize {
    let hand_h = unsafe { _3ds_board_get_zone_h(3) as f32 };
    let card_h = (hand_h - 4.0).max(1.0);
    let hsw = card_h * 0.711;
    let stride = hsw + 1.0;
    let count = ((316.0 - hsw) / stride) as usize + 1;
    count.max(1).min(15)
}
