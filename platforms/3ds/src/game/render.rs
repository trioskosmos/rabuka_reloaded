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
use crate::ui::grid::{render_card_detail, render_card_grid};
use crate::ui::hint::{render_hint_bar, HINT_BAR_SCALE, HINT_BAR_Y};
use crate::ui::layers::Layer;
use crate::ui::layers::Painter;
use crate::ui::text::*;
use crate::util::{cn_or_empty, tl_area};

use super::{
    compute_live_need, compute_total_hearts, find_card_zone_slot, has_under_cards, pref,
};

/// The number of screen lines a PlayMemberToStage group occupies: the card
/// header line plus one line for all of its stage areas (left/center/right).
const GROUP_LINES: usize = 2;
/// Vertical space (px) reserved per rendered line in the action list.
const LINE_H: f32 = 16.0;
/// Bottom edge (px) of the action list area on the top screen.
const LIST_BOTTOM: f32 = 230.0;

/// Immutable context for one board-render frame. Bundles the game/UI state
/// that every per-mode render function reads, so none of them need the huge
/// parameter list that used to be threaded through the single megafunction.
struct RenderCtx<'a> {
    gs: &'a GameState,
    ap: &'a Player,
    cur: usize,
    acts_cache: &'a [game_setup::Action],
    display_order: &'a [usize],
    display_pos: usize,
    detail_mode: bool,
    choice_subview: bool,
    detail_scroll_y: f32,
    viewing_card: Option<i16>,
    zone_viewer: &'a Option<(String, Vec<i16>)>,
    zone_viewer_offset: usize,
    my_player_idx: usize,
    has_image_choice: bool,
    has_text_choice: bool,
    is_multiplayer: bool,
    is_host: bool,
    vs_ai: bool,
    ai_vs_ai: bool,
    is_ai_turn: bool,
    atlas: &'a CardAtlas,
}

/// The top status line (turn/phase/perspective + both players' hand/energy/
/// deck counts). Shared by the game header and the card-detail overlay so the
/// two never drift apart.
fn header_status_line(ctx: &RenderCtx) -> String {
    let gs = ctx.gs;
    let ap = ctx.ap;
    let my = ctx.my_player_idx;
    let phase_name = if current_lang() == Lang::Japanese {
        gs.current_phase.label_jp().to_string()
    } else {
        format!("{}", gs.current_phase)
    };
    format!(
        "T{} {} [{}]  Me H:{} E:{}/{} D:{}  Opp H:{} E:{}/{} D:{}",
        gs.turn_number,
        phase_name,
        if ap.id == pref(gs, my).id {
            "Me"
        } else {
            "Opp"
        },
        pref(gs, my).hand.cards.len(),
        pref(gs, my).energy_zone.active_count(),
        pref(gs, my).energy_zone.cards.len(),
        pref(gs, my).main_deck.cards.len(),
        pref(gs, 1 - my).hand.cards.len(),
        pref(gs, 1 - my).energy_zone.active_count(),
        pref(gs, 1 - my).energy_zone.cards.len(),
        pref(gs, 1 - my).main_deck.cards.len(),
    )
}

/// Returns the index just past a PlayMemberToStage group starting at `di`.
/// Consecutive same-card PMTS actions (one per stage area) form a single group.
fn group_end(acts_cache: &[game_setup::Action], display_order: &[usize], di: usize, n: usize) -> usize {
    let fi = display_order[di];
    let act = &acts_cache[fi];
    if act.action_type != game_setup::ActionType::PlayMemberToStage {
        return di + 1;
    }
    let cid = act.parameters.as_ref().and_then(|p| p.card_id).unwrap_or(-1);
    if cid == -1 {
        return di + 1;
    }
    let mut ge = di + 1;
    while ge < n {
        let ga = &acts_cache[display_order[ge]];
        if ga.action_type == game_setup::ActionType::PlayMemberToStage
            && ga.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        {
            ge += 1;
        } else {
            break;
        }
    }
    ge
}

/// Backs `idx` up to the start of the group that contains it, so a window
/// never starts mid-group (which would hide earlier areas of a card).
fn snap_group_start(
    acts_cache: &[game_setup::Action],
    display_order: &[usize],
    idx: usize,
) -> usize {
    let n = display_order.len();
    if idx == 0 || idx >= n {
        return idx;
    }
    // Iterate groups from the top; return the group start that contains idx.
    let mut di = 0;
    while di < n {
        let ge = group_end(acts_cache, display_order, di, n);
        if idx < ge {
            return di;
        }
        di = ge;
    }
    idx
}

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
    detail_mode: bool,
    choice_subview: bool,
    text_page: usize,
    _choice_grid_offset: usize,
    list_scroll: usize,
    detail_scroll_y: f32,
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
    let ctx = RenderCtx {
        gs,
        ap,
        cur,
        acts_cache,
        display_order,
        display_pos,
        detail_mode,
        choice_subview,
        detail_scroll_y,
        viewing_card,
        zone_viewer,
        zone_viewer_offset,
        my_player_idx,
        has_image_choice,
        has_text_choice,
        is_multiplayer,
        is_host,
        vs_ai: *vs_ai,
        ai_vs_ai: *ai_vs_ai,
        is_ai_turn,
        atlas,
    };
    render_game_header(&ctx);
    let (text_page, content_y) = render_content_panel(&ctx, text_page, 52.0);
    let (text_page, list_scroll) = render_choice_area(&ctx, text_page, list_scroll, content_y);
    render_board_highlights(&ctx);
    (text_page, list_scroll)
}

fn render_game_header(ctx: &RenderCtx) {
    // Clear the top screen so old menu content doesn't overlap
    unsafe {
        _3ds_top_clear();
    }
    unsafe {
        _3ds_top_queue_rect(0.0, 0.0, 400.0, 50.0, COL_PANEL);
        _3ds_top_queue_text(
            4.0,
            2.0,
            COL_GOLD,
            SCALE_SMALL,
            format!("{}\0", header_status_line(ctx)).as_ptr(),
        );
        let p1_blade: u32 = ctx.gs.player1.stage.total_blades(
            &ctx.gs.card_database,
            &ctx.gs.mods.blade_modifiers,
            &ctx.gs.mods.orientation_modifiers,
            false,
        ) as u32;
        let p2_blade: u32 = ctx.gs.player2.stage.total_blades(
            &ctx.gs.card_database,
            &ctx.gs.mods.blade_modifiers,
            &ctx.gs.mods.orientation_modifiers,
            false,
        ) as u32;
        // Compute total hearts per player from stage members
        // (mirrors display.rs player_to_display total_hearts logic)
        let p1_hearts = compute_total_hearts(&ctx.gs.player1, ctx.gs);
        let p2_hearts = compute_total_hearts(&ctx.gs.player2, ctx.gs);
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
        render_text_with_icons(4.0, 22.0, &p1_stats, COL_LIGHT, SCALE_SMALL);
        // Render P2 hearts+blades on top screen line 3
        let p2_stats = if p2_heart_str.is_empty() {
            format!("BL:{}", p2_blade)
        } else {
            format!("{}  {{{{icon_blade.png|BLADE}}}}{}", p2_heart_str, p2_blade)
        };
        render_text_with_icons(4.0, 34.0, &p2_stats, COL_LIGHT, SCALE_SMALL);
        // Show need hearts during live set phase
        // Rule 8.2.x: opponent's need hearts are hidden
        // until their cards are revealed (performed).
        let is_live_set = matches!(
            ctx.gs.current_phase,
            Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
        );
        if is_live_set {
            // P1 (perspective) need hearts
            let p1_nh = compute_live_need(&ctx.gs.player1, ctx.gs);
            if p1_nh.iter().any(|&v| v > 0) {
                let nh_str = format_hearts(&p1_nh);
                let need_display = format!("{{{{icon_heart_06.png|NEED}}}} {}", nh_str);
                render_text_with_icons(4.0, 46.0, &need_display, COL_GOLD, SCALE_SMALL);
            }
            // P2 (opponent) need hearts — only after performed
            if ctx.gs.opponent_has_performed(ctx.my_player_idx) {
                let p2_nh = compute_live_need(&ctx.gs.player2, ctx.gs);
                if p2_nh.iter().any(|&v| v > 0) {
                    let nh_str = format_hearts(&p2_nh);
                    let need_display = format!("{{{{icon_heart_06.png|NEED}}}} {}", nh_str);
                    render_text_with_icons(4.0, 46.0, &need_display, COL_GOLD, SCALE_SMALL);
                }
            }
        }
    }
}

fn render_content_panel(ctx: &RenderCtx, mut text_page: usize, mut content_y: f32) -> (usize, f32) {
    let gs = ctx.gs;
    let cur = ctx.cur;
    let acts_cache = ctx.acts_cache;
    let detail_mode = ctx.detail_mode;
    let choice_subview = ctx.choice_subview;
    let detail_scroll_y = ctx.detail_scroll_y;
    let viewing_card = ctx.viewing_card;
    let zone_viewer = ctx.zone_viewer;
    let zone_viewer_offset = ctx.zone_viewer_offset;
    let has_image_choice = ctx.has_image_choice;
    let has_text_choice = ctx.has_text_choice;
    let is_ai_turn = ctx.is_ai_turn;
    let atlas = ctx.atlas;

    if let Some((ref zlabel, ref zcards)) = zone_viewer {
        if viewing_card.is_none() {
            unsafe {
                _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
                _3ds_top_queue_text(
                    4.0,
                    4.0,
                    COL_GOLD,
                    SCALE_BODY,
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
                            SCALE_LARGE,
                            format!("{}\0", tl("Ability")).as_ptr(),
                        );
                    }
                    let mut all_lines: Vec<String> = Vec::new();
                    let abs: Vec<_> = card.resolved_abilities().collect();
                    if abs.is_empty() {
                        let raw = card.ability_text();
                        if !raw.is_empty() {
                            let clean = raw.replace('\n', " ");
                            let w = wrap_ability_text(&clean, 384.0, SCALE_BODY);
                            for l in w.lines() {
                                all_lines.push(l.to_string());
                            }
                        }
                    } else {
                        for ab in &abs {
                            let ab_text =
                                i18n::translate_ability(&ab.full_text, current_lang());
                            let w = wrap_ability_text(&ab_text, 384.0, SCALE_BODY);
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
                        render_text_with_icons(4.0, ty, line, COL_LIGHT, SCALE_BODY);
                        ty += 18.0;
                    }
                    if total_pages > 1 {
                        unsafe {
                            _3ds_top_queue_text(
                                370.0,
                                4.0,
                                COL_MED,
                                SCALE_SMALL,
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
                        let w = wrap_ability_text(&ab_text, 392.0, SCALE_BODY);
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
    
                    let mut p = Painter::new();
                    // Background for the detail area (opaque so scrolled
                    // ability text never bleeds through behind the name)
                    p.rect(Layer::Content, 0.0, 52.0, 400.0, 188.0, COL_CARD_OPAQUE);
                    // Card portrait (left column)
                    p.rect(
                        Layer::Content,
                        card_x - 2.0,
                        card_y - 2.0,
                        card_w + 4.0,
                        card_h + 4.0,
                        COL_GOLD,
                    );
                    if let Some((atl, idx)) = atlas.lookup(&card.card_no) {
                        p.card(Layer::Content, atl, *idx as i32, card_x, card_y, card_w, card_h);
                    }
                    // Scrollable ability text (right column) on the BodyText
                    // layer; scrolled lines may reach up into the header region.
                    let text_top = card_y + 40.0;
                    let mut ty = text_top - detail_scroll_y;
                    for ab in card.resolved_abilities() {
                        let ab_text =
                            i18n::translate_ability(&ab.full_text, current_lang());
                        let w = wrap_ability_text(&ab_text, text_w, SCALE_BODY);
                        for line in w.lines() {
                            if ty > -200.0 && ty < 240.0 {
                                p.text(Layer::BodyText, text_x, ty, COL_LIGHT, SCALE_BODY, line);
                            }
                            ty += 18.0;
                        }
                        ty += 3.0;
                    }
                    ability_end = ty;
                    // Cover layer: an opaque rect the same color as the
                    // background, spanning the whole right column from the
                    // top down to the text start, so any scrolled-up text is
                    // hidden beneath the name/stats. Flushes above BodyText.
                    p.rect(
                        Layer::Cover,
                        text_x,
                        0.0,
                        400.0 - text_x,
                        text_top,
                        COL_CARD_OPAQUE,
                    );
                    // Header layer: card id/name and stats on top of the cover.
                    let display_name = i18n::card_display_name(&card.name, current_lang());
                    let name_label = crate::ui::text::truncate_to_width(
                        &format!("[{}] ", card.card_no),
                        &display_name,
                        SCALE_LARGE,
                        text_w,
                    );
                    p.text(Layer::Header, text_x, card_y - 2.0, COL_BLUE, SCALE_LARGE, &name_label);
                    let stats = compute_card_stats(card, cid, gs);
                    p.text(
                        Layer::Header,
                        text_x,
                        card_y + 20.0,
                        COL_LIGHT,
                        SCALE_BODY,
                        &card_stat_line(
                            stats.total_blade,
                            &stats.heart_str,
                            stats.score,
                            stats.cost.into(),
                            stats.is_tapped,
                            card.card_type.as_card_str(),
                            &stats.need_heart_str,
                        ),
                    );
                    // Scroll indicators (right edge)
                    let arrow_x = 400.0 - 18.0;
                    if ty > 228.0 {
                        p.text(Layer::Header, arrow_x, 228.0, COL_MED, SCALE_SMALL, "v");
                    }
                    if detail_scroll_y > 0.0 {
                        p.text(Layer::Header, arrow_x, 56.0, COL_MED, SCALE_SMALL, "^");
                    }
                    // Game header redrawn on top of detail content: opaque
                    // rect on the Cover layer, its text on the Header layer so
                    // anything scrolled under it is hidden.
                    p.rect(Layer::Cover, 0.0, 0.0, 400.0, 50.0, COL_PANEL);
                    p.text(
                        Layer::Header,
                        4.0,
                        2.0,
                        COL_GOLD,
                        SCALE_BODY,
                        &header_status_line(ctx),
                    );
                    p.flush();
                }
            }
            content_y = if ability_end > 0.0 {
                ability_end + 6.0
            } else {
                158.0
            };
            // If the viewed card is a stage member with cards stacked
            // beneath it, offer a button to open the under-cards viewer.
            let has_under = viewing_card.is_some_and(|cid| has_under_cards(gs, cid));
            if has_under {
                render_hint_bar(&tl("L=text  Y=under"));
            }
        }
    } else {
        if let Some(vcid) = viewing_card {
            // Compact card info overlay with stats
            if let Some(card) = gs.card_database.get_card(vcid) {
                let stats = compute_card_stats(card, vcid, gs);
                unsafe {
                    _3ds_top_queue_rect(0.0, 52.0, 400.0, 76.0, COL_CARD_OPAQUE);
                    let btm_name = i18n::card_display_name(&card.name, current_lang());
                    _3ds_top_queue_text(
                        4.0,
                        44.0,
                        COL_BLUE,
                        SCALE_LARGE,
                        format!("[{}] {}\0", card.card_no, wrap_text(&btm_name, 392.0, SCALE_LARGE))
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
                        SCALE_BODY,
                    );
                    if let Some(ab) = card.resolved_abilities().next() {
                        let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
                        let first_line = wrap_ability_text(&ab_text, 392.0, SCALE_BODY)
                            .lines()
                            .next()
                            .unwrap_or("")
                            .to_string();
                        render_text_with_icons(4.0, 82.0, &first_line, COL_LIGHT, SCALE_BODY);
                    }
                }
            }
            content_y = 126.0;
        } else if let Some(entry) = gs.ability_queue.current_entry() {
            // In image mode with choices, the text subview handles this.
            // The banner is only for CLI/text mode.
            if !(has_image_choice || has_text_choice) && !is_ai_turn {
                let ab_text = i18n::translate_ability(&entry.ability.full_text, current_lang());
                let ab_lines: Vec<String> = wrap_ability_text(&ab_text, 392.0, SCALE_BODY)
                    .lines()
                    .take(4)
                    .map(|l| l.to_string())
                    .collect();
                let n_lines = ab_lines.len();
                let h = 22.0 + n_lines as f32 * 14.0;
                unsafe {
                    _3ds_top_queue_rect(0.0, 52.0, 400.0, h, COL_ABILITY);
                    render_text_with_icons(4.0, 54.0, &ab_lines[0], COL_LIGHT, SCALE_BODY);
                    for (li, line) in ab_lines.iter().enumerate().skip(1) {
                        render_text_with_icons(
                            8.0,
                            54.0 + li as f32 * 14.0,
                            line,
                            COL_LIGHT,
                            SCALE_BODY,
                        );
                    }
                }
                content_y = 52.0 + h + 6.0;
            }
        }
    }
    (text_page, content_y)
}

fn render_choice_area(ctx: &RenderCtx, mut text_page: usize, mut list_scroll: usize, content_y: f32) -> (usize, usize) {
    let gs = ctx.gs;
    let acts_cache = ctx.acts_cache;
    let display_order = ctx.display_order;
    let display_pos = ctx.display_pos;
    let detail_mode = ctx.detail_mode;
    let choice_subview = ctx.choice_subview;
    let viewing_card = ctx.viewing_card;
    let zone_viewer = ctx.zone_viewer;
    let has_image_choice = ctx.has_image_choice;
    let has_text_choice = ctx.has_text_choice;
    let is_multiplayer = ctx.is_multiplayer;
    let is_host = ctx.is_host;
    let vs_ai = &ctx.vs_ai;
    let ai_vs_ai = &ctx.ai_vs_ai;
    let atlas = ctx.atlas;
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
                        let desc_lines: Vec<String> = wrap_text(&desc, 392.0, SCALE_BODY)
                            .lines()
                            .map(|l| l.to_string())
                            .collect();
                        let header_h = 12.0 + desc_lines.len().min(2) as f32 * 14.0;
                        unsafe {
                            _3ds_top_queue_rect(0.0, content_y, 400.0, header_h, COL_ABILITY);
                        }
                        let mut oy = content_y + 3.0;
                        for line in desc_lines.iter().take(2) {
                            render_text_with_icons(4.0, oy, line, COL_GOLD, SCALE_BODY);
                            oy += 14.0;
                        }
                        let mut ty = content_y + header_h + 4.0;
                        let n = options.len();
                        let line_h = 16.0f32;
                        // Keep the selected option at the top of the list so its
                        // (possibly multi-line) ability text is always visible.
                        // Options with many lines no longer push it off-screen.
                        list_scroll = display_pos.min(n.saturating_sub(1));
                        let start = list_scroll;
                        let end = n;
                        if start > 0 {
                            unsafe {
                                _3ds_top_queue_text(
                                    4.0,
                                    ty,
                                    COL_MED,
                                    SCALE_BODY,
                                    format!("\u{25b2} +{}\0", start).as_ptr(),
                                );
                                ty += line_h;
                            }
                        }
                        let mut di = start;
                        while di < end && ty < 230.0 {
                            let opt = &options[di];
                            let is_sel = di == display_pos;
                            let line_color = if is_sel { COL_GOLD } else { COL_LIGHT };
                            let cn = opt
                                .card_id
                                .and_then(|cid| gs.card_database.get_card(cid))
                                .map(|card| card.card_no.to_string())
                                .unwrap_or_default();
                            let header = if cn.is_empty() {
                                opt.card_name.clone()
                            } else {
                                format!("[{}] {}", cn, opt.card_name)
                            };
                            for l in wrap_text(&header, 392.0, SCALE_BODY).lines() {
                                if ty > 230.0 {
                                    break;
                                }
                                render_text_with_icons(4.0, ty, l, line_color, SCALE_BODY);
                                ty += line_h;
                            }
                            let ab_text =
                                i18n::translate_ability(&opt.ability_text, current_lang());
                            let ab_wrapped = wrap_ability_text(&ab_text, 392.0, SCALE_BODY);
                            for (li, l) in ab_wrapped.lines().enumerate() {
                                if ty > 230.0 {
                                    break;
                                }
                                let txt = if li == 0 {
                                    format!("  {}", l)
                                } else {
                                    l.to_string()
                                };
                                render_text_with_icons(4.0, ty, &txt, line_color, SCALE_BODY);
                                ty += line_h;
                            }
                            ty += 4.0;
                            di += 1;
                        }
                        if di < n && ty < 230.0 {
                            unsafe {
                                _3ds_top_queue_text(
                                    4.0,
                                    ty,
                                    COL_MED,
                                    SCALE_BODY,
                                    format!("\u{25bc} +{}\0", n - di).as_ptr(),
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
                    let ab_lines: Vec<String> = wrap_ability_text(&banner_text, 392.0, SCALE_BODY)
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
                            SCALE_BODY,
                        );
                    }
                    grid_iy = 52.0 + h + 4.0;
                }
                // ---- Dynamic card sizing (matches waitroom) ----
                let has_ability = gs.ability_queue.current_entry().is_some();
                let cols = 5usize;
                let gap = 4.0f32;
                let max_rows = if has_ability { 1 } else { 2 };
                let pp = cols * max_rows;
                let n = display_order.len();
                let pos = display_pos.min(n.saturating_sub(1));
    
                // ---- Paginate over CARD items only ----
                // Text-only options (e.g. skip) never consume a page slot:
                // they're drawn as a bottom row on whichever card page the
                // cursor is on. So the page anchor is the first card index
                // of the card-page that contains `pos`.
                let page = {
                    let mut cards_seen = 0usize;
                    let mut page_start = 0usize;
                    for (di, &fi) in display_order.iter().enumerate() {
                        if is_text_only(&acts_cache[fi]) {
                            continue;
                        }
                        if cards_seen % pp == 0 {
                            page_start = di;
                        }
                        if di >= pos {
                            break;
                        }
                        cards_seen += 1;
                    }
                    page_start
                };
    
                // ---- Classify items on this page ----
                // Cards fill the grid slots; text items go to the bottom row.
                let mut card_gis: Vec<usize> = Vec::new();
                let mut text_gis: Vec<usize> = Vec::new();
                {
                    let mut di = page;
                    let mut cards_taken = 0usize;
                    while di < n && cards_taken < pp {
                        let fi = display_order[di];
                        if is_text_only(&acts_cache[fi]) {
                            text_gis.push(di - page);
                        } else {
                            card_gis.push(di - page);
                            cards_taken += 1;
                        }
                        di += 1;
                    }
                    // Trailing text items after the last card also belong here.
                    while di < n && is_text_only(&acts_cache[display_order[di]]) {
                        text_gis.push(di - page);
                        di += 1;
                    }
                }
    
                // ---- Reserve a bottom row for text-only options (e.g. "skip")
                // ---- so they sit below the cards like a menu row instead of
                // ---- an overlay drawn on top of the grid. The whole grid +
                // ---- skip row live above the canonical hint bar (HINT_BAR_Y).
                let has_text_opt = !text_gis.is_empty();
                let skip_row_h = 16.0f32;
                let grid_floor = if has_text_opt {
                    HINT_BAR_Y - skip_row_h - 4.0
                } else {
                    HINT_BAR_Y - 4.0
                };
                let max_ch = ((grid_floor - grid_iy) / max_rows as f32) - 14.0;
                let cw = (max_ch * 0.711)
                    .min((400.0 - 8.0 - (cols as f32 - 1.0) * gap) / cols as f32);
                let ch = cw / 0.711;
                let row_h = ch + 16.0 + gap;
    
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
                                        SCALE_SMALL,
                                        label.as_ptr(),
                                    );
                                }
                            }
                        }
                    }
                }
    
                // ---- Render text-only options (e.g. "skip") as a bottom row ----
                // Styled like main-phase action-list options: text-only, color
                // marks the selected one (no box, no disappearing highlight).
                if has_text_opt {
                    let row_y = HINT_BAR_Y - skip_row_h + 2.0;
                    let mut row_x = 8.0;
                    for &gi in text_gis.iter() {
                        let di = page + gi;
                        if di >= n {
                            break;
                        }
                        let fi = display_order[di];
                        let act = &acts_cache[fi];
                        let is_disabled = act
                            .parameters
                            .as_ref()
                            .and_then(|p| p.disabled)
                            .unwrap_or(false);
                        let is_sel = di == display_pos;
                        let desc = act.display_desc(current_lang() == Lang::Japanese);
                        let label = desc
                            .replace('\n', " ")
                            .trim_start_matches(|c: char| c == '・' || c == '\u{2022}')
                            .trim_start_matches("- ")
                            .trim()
                            .to_string();
                        let color = if is_disabled {
                            COL_MED
                        } else if is_sel {
                            COL_GOLD
                        } else {
                            COL_LIGHT
                        };
                        unsafe {
                            _3ds_top_queue_text(
                                row_x,
                                row_y,
                                color,
                                SCALE_BODY,
                                format!("{}\0", label).as_ptr(),
                            );
                        }
                        row_x += crate::ui::text::measure_text_width(&label, SCALE_BODY) + 12.0;
                        if row_x > 388.0 {
                            break;
                        }
                    }
                }
    
                // Hint: L opens text (canonical hint bar position)
                render_hint_bar(&tl("L=text"));
                // Page indicator: count CARD pages only (text options share the page).
                {
                    let n_cards = display_order
                        .iter()
                        .filter(|&&fi| !is_text_only(&acts_cache[fi]))
                        .count();
                    let total_p = (n_cards + pp - 1) / pp;
                    let cards_before = display_order[..pos]
                        .iter()
                        .filter(|&&fi| !is_text_only(&acts_cache[fi]))
                        .count();
                    let pg = cards_before / pp + 1;
                    if total_p > 1 {
                        unsafe {
                            _3ds_top_queue_text(
                                300.0,
                                HINT_BAR_Y,
                                COL_MED,
                                HINT_BAR_SCALE,
                                format!("{}\0", format!("{}/{}", pg, total_p)).as_ptr(),
                            );
                        }
                    }
                }
                // Text overlay on top of choices grid
                if choice_subview {
                    if let Some(entry) = gs.ability_queue.current_entry() {
                        let ab_lines: Vec<String> =
                            wrap_ability_text(&entry.ability.full_text, 384.0, SCALE_BODY)
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
                                SCALE_BODY,
                                format!("{}\0", tl("Ability")).as_ptr(),
                            );
                        }
                        let mut oy = 64.0;
                        for i in start_line..end_line {
                            render_text_with_icons(8.0, oy, &ab_lines[i], COL_LIGHT, SCALE_BODY);
                            oy += 20.0;
                        }
                        let page_str = format!("{}/{}", text_page + 1, total_pages);
                        unsafe {
                            _3ds_top_queue_text(
                                400.0 - page_str.len() as f32 * 7.0 - 8.0,
                                44.0,
                                COL_MED,
                                SCALE_SMALL,
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
                        SCALE_BODY,
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
                let line_h = LINE_H;
                let n = display_order.len();
                // Walk groups forward from a candidate start to build the
                // visible entry range [start, end). Each group consumes its
                // real entry span (GROUP_LINES worth of vertical space).
                let available = ((LIST_BOTTOM - content_y) / line_h) as usize;
                let build_window = |s: usize| -> (usize, usize) {
                    let mut end = s.min(n);
                    let mut used = 0usize;
                    while end < n {
                        let ge = group_end(acts_cache, display_order, end, n);
                        // Every PlayMemberToStage action renders as a 2-line
                        // group (header + areas), even a single-area one.
                        let is_pmts = acts_cache[display_order[end]].action_type
                            == game_setup::ActionType::PlayMemberToStage;
                        let lines = if is_pmts { GROUP_LINES } else { 1 };
                        if used + lines > available {
                            break;
                        }
                        used += lines;
                        end = ge;
                    }
                    end = end.max(s + 1).min(n);
                    (s, end)
                };
                // Advance the window so the cursor stays visible.
                let mut start = snap_group_start(acts_cache, display_order, list_scroll);
                let mut end;
                loop {
                    let (_, e2) = build_window(start);
                    end = e2;
                    if display_pos >= start && display_pos < end {
                        break;
                    }
                    if display_pos < start {
                        // scroll up: move to the group containing display_pos
                        start = snap_group_start(acts_cache, display_order, display_pos);
                    } else {
                        // scroll down: shift window forward by one group
                        if e2 >= n {
                            break;
                        }
                        start = e2;
                        start = snap_group_start(acts_cache, display_order, start);
                    }
                }
                list_scroll = start;
                if start > 0 {
                    unsafe {
                        _3ds_top_queue_text(
                            4.0,
                            ty,
                            COL_MED,
                            SCALE_BODY,
                            format!("\u{25b2} +{}\0", start).as_ptr(),
                        );
                        ty += line_h;
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
                    // Group = all consecutive same-card PlayMemberToStage areas.
                    // Always render PMTS as a 2-line group (header + areas row),
                    // even when only one area is available, so the layout is
                    // consistent and the selected area is clearly marked.
                    let ge = group_end(acts_cache, display_order, di, n);
                    let is_pmts =
                        act.action_type == game_setup::ActionType::PlayMemberToStage;
                    let is_group = is_pmts;
                    let group_sel = is_pmts && (di..ge).any(|i| i == display_pos);
                    let line_color = if group_sel || is_sel {
                        COL_GOLD
                    } else if is_disabled {
                        COL_MED
                    } else {
                        COL_LIGHT
                    };
                    let line_scale: f32 = SCALE_BODY;
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
                            format!(
                                "{{{{icon_energy.png|E}}}}{} [{}] {}",
                                base_cost, cn, name
                            )
                        } else {
                            format!("{{{{icon_energy.png|E}}}}{} {}", base_cost, name)
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
                                Some((cost, _)) if *cost > 0 => format!(
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
                            ty += line_h;
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
                            ty += line_h;
                        }
                        di = ge;
                    } else {
                        let line = super::action_list::format_action_line_image(act, gs);
                        let color = if is_disabled {
                            COL_MED
                        } else if is_sel {
                            COL_GOLD
                        } else {
                            COL_LIGHT
                        };
                        let scale: f32 = SCALE_BODY;
                        let wrap_w = 392.0;
                        for (_li, l) in wrap_text(&line, wrap_w, scale).lines().enumerate() {
                            if ty > 230.0 {
                                break;
                            }
                            let txt = l.to_string();
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
                            ty += line_h;
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
                            SCALE_BODY,
                            format!("\u{25bc} +{}\0", n - end).as_ptr(),
                        );
                    }
                }
            }
        } // closes if zone_viewer.is_none()
    (text_page, list_scroll)
}

fn render_board_highlights(ctx: &RenderCtx) {
    let gs = ctx.gs;
    let acts_cache = ctx.acts_cache;
    let detail_mode = ctx.detail_mode;
    let viewing_card = ctx.viewing_card;
    let my_player_idx = ctx.my_player_idx;
    let has_image_choice = ctx.has_image_choice;
    let is_multiplayer = ctx.is_multiplayer;
    let is_host = ctx.is_host;
    let vs_ai = &ctx.vs_ai;
    let ai_vs_ai = &ctx.ai_vs_ai;
    // Clear stale action highlight on bottom board
    unsafe {
        _3ds_board_clear_action_highlight();
        _3ds_board_clear_stage_play_cost();
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
                                // Per-area energy cost for the play target.
                                let cost = p
                                    .available_areas
                                    .as_ref()
                                    .and_then(|areas| {
                                        areas.iter().find(|a| &a.area == sa).map(|a| a.cost)
                                    })
                                    .unwrap_or(0);
                                unsafe {
                                    _3ds_board_set_action_highlight(1, slot, false);
                                    _3ds_board_set_stage_play_cost(0, slot, cost as i32);
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
