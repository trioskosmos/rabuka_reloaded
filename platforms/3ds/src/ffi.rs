// FFI declarations for the C shim (ctru_shim.c). Declared here so all modules
// (bin + lib) share one source of truth for the C-side API.

extern "C" {
    pub fn _3ds_init();
    pub fn _3ds_main_loop() -> i32;
    pub fn _3ds_exit();
    pub fn _3ds_swap_buffers();
    pub fn _3ds_scan_input();
    pub fn _3ds_keys_down() -> u32;
    pub fn _3ds_keys_held() -> u32;
    pub fn _3ds_touch_read(px: *mut u32, py: *mut u32);
    pub fn _3ds_touch_down() -> bool;
    pub fn _3ds_system_tick() -> u64;
    pub fn _3ds_debug_print(msg: *const u8);
    pub fn _3ds_tdbg(msg: *const u8);
    pub fn _3ds_clear_console();
    pub fn _3ds_clear_both();
    pub fn _3ds_clear_top();
    pub fn _3ds_text_add_top(msg: *const u8);
    pub fn _3ds_text_add_bot(msg: *const u8);
    pub fn _3ds_text_set_scroll_y(y: i32);
    pub fn _3ds_text_get_scroll_y() -> i32;
    pub fn _3ds_bot_line_height() -> f32;

    // Board API
    pub fn _3ds_board_enable(on: bool);
    pub fn _3ds_board_cycle_view();
    pub fn _3ds_board_current_view() -> i32;
    pub fn _3ds_board_clear_cache();
    // Player slots
    pub fn _3ds_board_set_stage(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_live(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_live_stats(slot: i32, score: i32, stat_text: *const u8);
    pub fn _3ds_board_set_opp_live_stats(slot: i32, score: i32, stat_text: *const u8);
    pub fn _3ds_board_set_energy(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_energy_count(count: i32);
    pub fn _3ds_board_set_hand(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_hand_count(count: i32);
    pub fn _3ds_board_set_hand_scroll_info(visible: i32, offset: i32, total: i32);
    pub fn _3ds_board_set_utility(deck: i32, edeck: i32, discard: i32, success: i32);
    // Opponent slots
    pub fn _3ds_board_set_opp_stage(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_opp_live(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_opp_energy(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_opp_energy_count(count: i32);
    pub fn _3ds_board_set_opp_hand(
        slot: i32,
        active: bool,
        atlas: *const u8,
        index: i32,
        landscape: bool,
        tapped: bool,
    );
    pub fn _3ds_board_set_opp_hand_count(count: i32);
    pub fn _3ds_board_set_opp_utility(deck: i32, edeck: i32, discard: i32, success: i32);
    pub fn _3ds_board_set_selection(slot: i32, slot_type: i32);
    pub fn _3ds_board_set_section_rect(y0: f32, h: f32, opponent: bool);
    pub fn _3ds_board_get_zone_y(zone_type: i32) -> i32;
    pub fn _3ds_board_get_zone_h(zone_type: i32) -> i32;
    pub fn _3ds_board_get_slot_w(zone_type: i32) -> f32;

    // Top screen graphical drawing
    pub fn _3ds_top_clear();
    pub fn _3ds_top_queue_rect(x: f32, y: f32, w: f32, h: f32, color: u32);
    pub fn _3ds_top_queue_text(x: f32, y: f32, color: u32, scale: f32, text: *const u8);
    pub fn _3ds_top_queue_card(atlas: *const u8, idx: i32, x: f32, y: f32, w: f32, h: f32);
    // Bottom screen graphical drawing (setup menus before the board is enabled)
    pub fn _3ds_bot_clear();
    pub fn _3ds_bot_queue_rect(x: f32, y: f32, w: f32, h: f32, color: u32);
    pub fn _3ds_bot_queue_text(x: f32, y: f32, color: u32, scale: f32, text: *const u8);
    pub fn _3ds_measure_text_width(text: *const u8, scale: f32) -> f32;
    pub fn _3ds_text_wrapped_height(text: *const u8, scale: f32, max_w: f32) -> f32;
    pub fn _3ds_icon_aspect(atlas_name: *const u8) -> f32;

    // Action highlight on board slots
    pub fn _3ds_board_set_action_highlight(zone: i32, slot: i32, opponent: bool);
    pub fn _3ds_board_clear_action_highlight();
    // Energy cost label shown above a stage slot where a hand card can be played.
    pub fn _3ds_board_set_stage_play_cost(player: i32, slot: i32, cost: i32);
    pub fn _3ds_board_clear_stage_play_cost();

    // Action overlay (Phase 2: actions on bottom screen, safe per-line copy)
    pub fn _3ds_board_set_action_overlay_state(count: i32, selected: i32);
    pub fn _3ds_board_set_action_overlay_text(index: i32, text: *const u8);
    pub fn _3ds_board_set_overlay_action_idx(display_line: i32, action_index: i32);
    pub fn _3ds_board_get_overlay_action_idx(display_line: i32) -> i32;
    pub fn _3ds_board_get_overlay_selected() -> i32;
    pub fn _3ds_board_clear_action_overlay();
    // Need hearts counts displayed next to live zone on bottom screen
    pub fn _3ds_set_need_hearts(
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
    pub fn _3ds_qr_start() -> *mut u8;
    pub fn _3ds_qr_stop(ctx: *mut u8);
    pub fn _3ds_qr_free(ctx: *mut u8);
    pub fn _3ds_qr_poll(ctx: *mut u8, out_text: *mut u8, out_max: u32) -> i32;
    // Audio (CSND + tremor OGG); *_init/*_play_ogg return 0 on success
    pub fn _3ds_audio_init() -> i32;
    pub fn _3ds_audio_play_ogg(path: *const u8) -> i32;
    pub fn _3ds_audio_stop();
    pub fn _3ds_audio_set_volume(vol: f32);
}
