#![cfg(feature = "3ds")]
// Play state machine (Phase C): the Step::Play handler, moved verbatim from the
// bin (see extract_play.py). PlayState replaces the old 32-field tuple.

mod overlays;
mod render;

use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

use crate::dprintln;
use crate::ffi::*;
use crate::i18n;
use crate::lang::{current_lang, tl};
use crate::net::{execute_received_action, mp_can_act, route_authoritative_action};
use crate::steps::{Overlay, Step};
use crate::uds;
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::grid::{card_grid_input, GridAction};
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

/// Sum the need-heart counts (8 colors) for a player's live zone, including
/// need_heart_modifiers. Single source of truth (was duplicated twice inline).
fn compute_live_need(player: &Player, gs: &GameState) -> Vec<u32> {
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
}

/// Sum the stage total-heart counts (8 colors) for a player, including
/// heart_modifiers and the heart_color_multiplier. Mirrors display.rs
/// player_to_display total_hearts logic. Single source of truth.
fn compute_total_hearts(player: &Player, gs: &GameState) -> Vec<u32> {
    let mut hearts = vec![0u32; 8];
    for &cid in &player.stage.stage {
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
                                hearts[idx] += *count as u32;
                            }
                        } else {
                            hearts[idx] += *count as u32;
                        }
                    }
                }
            }
        }
    }
    for (cid, modifier) in &gs.mods.heart_modifiers {
        if !player.stage.stage.contains(cid) {
            continue;
        }
        for (color, val) in modifier {
            if let Some(idx) = heart_color_index(color) {
                hearts[idx] = (hearts[idx] as i32 + val.total()).max(0) as u32;
            }
        }
    }
    hearts
}

/// Hoisted player lookup (was an inner fn of play_step; shared by render.rs).
#[inline(always)]
fn pref<'a>(gs: &'a GameState, idx: usize) -> &'a Player {
    if idx == 0 {
        &gs.player1
    } else {
        &gs.player2
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
    overlays::overlay_input(&mut overlay, &gs, keys, is_host, &mut redraw);
    if detail_mode && viewing_card.is_some() {
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
            if !gs.opponent_has_performed(my_player_idx) {
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
            // P1 (perspective player) need hearts — always show if any
            let p1_nh = compute_live_need(&gs.player1, &gs);
            unsafe {
                _3ds_set_need_hearts(
                    0, p1_nh[0], p1_nh[1], p1_nh[2], p1_nh[3], p1_nh[4], p1_nh[5], p1_nh[6],
                    p1_nh[7],
                );
            }
            // P2 (opponent) need hearts — hidden until performed
            if gs.opponent_has_performed(my_player_idx) {
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

        let (new_text_page, new_list_scroll) = render::render_board(
            &gs,
            ap,
            cur,
            &acts_cache,
            &display_order,
            display_pos,
            cli_mode,
            detail_mode,
            choice_subview,
            text_page,
            choice_grid_offset,
            list_scroll,
            detail_scroll_y,
            touch_tap_count,
            viewing_card,
            &zone_viewer,
            zone_viewer_offset,
            my_player_idx,
            has_image_choice,
            has_text_choice,
            is_multiplayer,
            is_host,
            vs_ai,
            ai_vs_ai,
            is_ai_turn,
            atlas,
        );
        text_page = new_text_page;
        list_scroll = new_list_scroll;

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
            overlays::render_overlay(&gs, overlay, is_host, atlas);
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
