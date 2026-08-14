#![cfg(feature = "3ds")]

// Input handling for play_step: overlay input, nav/choice input, MP
// recv/AI/auto, tap-to-deploy, touch. Extracted from the Step::Play handler
// (engine_duplication.md 1.5 input.rs). Mutates the mutable scalar state,
// which is returned to the caller in InputOut.

use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::turn;

use crate::dprintln;
use crate::ffi::*;
use crate::i18n;
use crate::lang::{current_lang, tl};
use crate::net::{execute_received_action, mp_can_act, route_authoritative_action};
use crate::steps::Overlay;
use crate::uds;
use crate::ui::grid::{card_grid_input, GridAction};
use crate::ui::text::*;

use super::{pref, visible_hand_slots};

/// Mutable play state carried back to play_step after input handling.
pub(crate) struct InputOut {
    pub cur: usize,
    pub cli_mode: bool,
    pub detail_mode: bool,
    pub choice_image_mode: bool,
    pub choice_subview: bool,
    pub text_page: usize,
    pub choice_grid_offset: usize,
    pub detail_scroll_y: f32,
    pub hand_offset: usize,
    pub hand_offset_p2: usize,
    pub touch_tap_count: u32,
    pub viewing_card: Option<i16>,
    pub zone_viewer: Option<(String, Vec<i16>)>,
    pub zone_viewer_offset: usize,
    pub was_touching: bool,
    pub waiting_for_opponent: bool,
    pub overlay: Overlay,
    pub pending_client_action: Option<Vec<u8>>,
    pub last_client_action_seq: u32,
    pub next_action_seq: u32,
    pub dbg_tx_bytes: u32,
    pub dbg_rx_bytes: u32,
    pub dirty: bool,
    pub redraw: bool,
    pub display_pos: usize,
    pub is_ai_turn: bool,
}

/// Process one frame of input. `gs`/`acts_cache` are mutated in place; the
/// scalar UI state is passed by value and returned.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_input(
    gs: &mut GameState,
    acts_cache: &mut Vec<game_setup::Action>,
    keys: u32,
    display_order: &[usize],
    mut cur: usize,
    mut cli_mode: bool,
    mut detail_mode: bool,
    mut choice_image_mode: bool,
    mut choice_subview: bool,
    mut text_page: usize,
    mut choice_grid_offset: usize,
    mut detail_scroll_y: f32,
    mut hand_offset: usize,
    mut hand_offset_p2: usize,
    mut touch_tap_count: u32,
    mut viewing_card: Option<i16>,
    mut zone_viewer: Option<(String, Vec<i16>)>,
    mut zone_viewer_offset: usize,
    mut was_touching: bool,
    mut waiting_for_opponent: bool,
    mut overlay: Overlay,
    mut pending_client_action: Option<Vec<u8>>,
    mut last_client_action_seq: u32,
    mut next_action_seq: u32,
    mut dbg_tx_bytes: u32,
    mut dbg_rx_bytes: u32,
    mut dirty: bool,
    mut redraw: bool,
    mut display_pos: usize,
    has_image_choice: bool,
    is_multiplayer: bool,
    is_host: bool,
    vs_ai: &bool,
    ai_vs_ai: &bool,
    my_player_idx: usize,
) -> InputOut {
    // DPAD/menu navigation is suppressed while an overlay (Start/perf/log/revealed)
    // is open — the overlay consumes D-pad/A/B itself via overlay_input.
    if overlay == Overlay::None && detail_mode && viewing_card.is_some() {
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
    } else if overlay == Overlay::None && detail_mode {
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
    } else if overlay == Overlay::None && !has_image_choice {
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
                                execute_received_action(gs, &sync);
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
                gs,
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
                    gs, &ai_action, None, None, None, None,
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
                gs,
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
        game_setup::settle_single_player_state(gs);
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
                                gs,
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
                                gs,
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
                            gs,
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
                            gs,
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
                            gs,
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
    InputOut {
        cur,
        cli_mode,
        detail_mode,
        choice_image_mode,
        choice_subview,
        text_page,
        choice_grid_offset,
        detail_scroll_y,
        hand_offset,
        hand_offset_p2,
        touch_tap_count,
        viewing_card,
        zone_viewer,
        zone_viewer_offset,
        was_touching,
        waiting_for_opponent,
        overlay,
        pending_client_action,
        last_client_action_seq,
        next_action_seq,
        dbg_tx_bytes,
        dbg_rx_bytes,
        dirty,
        redraw,
        display_pos,
        is_ai_turn,
    }
}
