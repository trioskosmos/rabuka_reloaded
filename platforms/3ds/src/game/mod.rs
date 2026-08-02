#![cfg(feature = "3ds")]
// Play state machine (Phase C): the Step::Play handler, moved verbatim from the
// bin (see extract_play.py). PlayState replaces the old 32-field tuple.

mod action_list;
mod input;
mod overlays;
mod render;

use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;

use crate::dprintln;
use crate::ffi::*;
use crate::lang::tl;
use crate::net::mp_can_act;
use crate::steps::{Overlay, Step};
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::*;
use crate::ui::text::*;
use crate::util::heart_color_index;

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

#[allow(unused_assignments)] // display_pos snapshot may be recomputed by the render guard
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
    let out = input::handle_input(
        &mut gs,
        &mut acts_cache,
        keys,
        &display_order,
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
        has_image_choice,
        is_multiplayer,
        is_host,
        vs_ai,
        ai_vs_ai,
        my_player_idx,
    );
    cur = out.cur;
    cli_mode = out.cli_mode;
    detail_mode = out.detail_mode;
    choice_image_mode = out.choice_image_mode;
    choice_subview = out.choice_subview;
    text_page = out.text_page;
    choice_grid_offset = out.choice_grid_offset;
    detail_scroll_y = out.detail_scroll_y;
    hand_offset = out.hand_offset;
    hand_offset_p2 = out.hand_offset_p2;
    touch_tap_count = out.touch_tap_count;
    viewing_card = out.viewing_card;
    zone_viewer = out.zone_viewer;
    zone_viewer_offset = out.zone_viewer_offset;
    was_touching = out.was_touching;
    waiting_for_opponent = out.waiting_for_opponent;
    overlay = out.overlay;
    pending_client_action = out.pending_client_action;
    last_client_action_seq = out.last_client_action_seq;
    next_action_seq = out.next_action_seq;
    dbg_tx_bytes = out.dbg_tx_bytes;
    dbg_rx_bytes = out.dbg_rx_bytes;
    dirty = out.dirty;
    redraw = out.redraw;
    // display_pos is only meaningful when the render guard below doesn't
    // recompute it from the freshly rebuilt display_order.
    if !(dirty || redraw) {
        display_pos = out.display_pos;
    }
    let is_ai_turn = out.is_ai_turn;

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
