// Card grid: input handling + rendering for deck/zone viewers.

#![cfg_attr(not(feature = "3ds"), allow(unused_imports, dead_code))]

use rabuka_engine::card::CardDatabase;

use crate::ffi::_3ds_top_queue_card;
use crate::ffi::_3ds_top_queue_rect;
use crate::ffi::_3ds_top_queue_text;
use crate::i18n;
#[cfg(feature = "3ds")]
use crate::lang::current_lang;
#[cfg(feature = "3ds")]
use crate::ui::card_atlas::CardAtlas;
use crate::ui::colors::COL_BLUE;
use crate::ui::colors::COL_CARD;
use crate::ui::colors::COL_GOLD;
use crate::ui::colors::COL_LIGHT;
use crate::ui::colors::COL_MED;
use crate::ui::colors::COL_TOP_BG;
use crate::ui::text::build_heart_str;
use crate::ui::text::card_stat_line;
use crate::ui::text::render_text_with_icons;
use crate::ui::text::wrap_ability_text;
use crate::ui::text::CardDisplayStats;

#[cfg(feature = "3ds")]
#[derive(Clone, Copy, PartialEq)]
pub enum GridAction {
    None,
    CloseGrid,
    CloseDetail,
    OpenDetail(i16),
    Navigate,
}

#[cfg(feature = "3ds")]
pub fn card_grid_input(
    keys: u32,
    cursor: &mut usize,
    viewing_card: &mut Option<i16>,
    card_ids: &[i16],
    cols: usize,
) -> GridAction {
    let total = card_ids.len();
    if total == 0 {
        return GridAction::None;
    }
    if keys & 0x00000002 != 0 {
        if viewing_card.is_some() {
            *viewing_card = None;
            return GridAction::CloseDetail;
        } else {
            return GridAction::CloseGrid;
        }
    }
    if keys & 0x00000400 != 0 && viewing_card.is_none() {
        if *cursor < total {
            *viewing_card = Some(card_ids[*cursor]);
            return GridAction::OpenDetail(card_ids[*cursor]);
        }
    }
    if viewing_card.is_none() {
        let mut moved = false;
        if keys & 0x00000040 != 0 {
            *cursor = cursor.saturating_sub(cols);
            moved = true;
        }
        if keys & 0x00000080 != 0 {
            *cursor = (*cursor + cols).min(total - 1);
            moved = true;
        }
        if keys & 0x00000020 != 0 && *cursor > 0 {
            *cursor -= 1;
            moved = true;
        }
        if keys & 0x00000010 != 0 && *cursor + 1 < total {
            *cursor += 1;
            moved = true;
        }
        if moved {
            return GridAction::Navigate;
        }
    }
    GridAction::None
}

#[cfg(feature = "3ds")]
pub fn render_card_grid(
    card_ids: &[i16],
    cursor: usize,
    cols: usize,
    rows: usize,
    y_start: f32,
    card_db: &CardDatabase,
    atlas: &CardAtlas,
) {
    let gap = 4.0f32;
    let pp = cols * rows;
    let max_ch = ((240.0 - y_start - gap) / rows as f32) - 14.0;
    let cw = (max_ch * 0.711).min((400.0 - 8.0 - (cols as f32 - 1.0) * gap) / cols as f32);
    let ch = cw / 0.711;
    let page = (cursor / pp) * pp;
    let n = card_ids.len();
    for i in page..n.min(page + pp) {
        let col = (i - page) % cols;
        let row = (i - page) / cols;
        let ix = 4.0 + col as f32 * (cw + gap);
        let iy = y_start + row as f32 * (ch + 14.0 + gap);
        let cid = card_ids[i];
        let border = if i == cursor { COL_GOLD } else { COL_CARD };
        unsafe {
            _3ds_top_queue_rect(ix, iy, cw, ch + 14.0, border);
        }
        let cn = card_db
            .get_card(cid)
            .map(|c| c.card_no.as_ref())
            .unwrap_or("?");
        if let Some((atl, idx)) = atlas.lookup(cn) {
            let c_str = std::ffi::CString::new(atl.as_bytes()).unwrap_or_default();
            unsafe {
                _3ds_top_queue_card(
                    c_str.as_ptr() as *const u8,
                    *idx as i32,
                    ix + 1.0,
                    iy + 1.0,
                    cw - 2.0,
                    ch,
                );
                _3ds_top_queue_text(
                    ix + 1.0,
                    iy + ch + 1.0,
                    COL_LIGHT,
                    0.35f32,
                    format!("{}\0", cn).as_ptr(),
                );
            }
        }
    }
    if n > pp {
        let page_n = page / pp + 1;
        let total_p = (n + pp - 1) / pp;
        unsafe {
            _3ds_top_queue_text(
                300.0,
                4.0,
                COL_MED,
                0.50f32,
                format!("{}/{}\0", page_n, total_p).as_ptr(),
            );
        }
    }
}

/// Draw a card portrait image at (x, y) with the given width/height.
#[cfg(feature = "3ds")]
pub fn draw_card_image(
    card_no: &str,
    atlas: &CardAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    if let Some((atl, idx)) = atlas.lookup(card_no) {
        let c_str = std::ffi::CString::new(atl.as_bytes()).unwrap_or_default();
        unsafe {
            _3ds_top_queue_card(
                c_str.as_ptr() as *const u8,
                *idx as i32,
                x,
                y,
                w,
                h,
            );
        }
    }
}

#[cfg(feature = "3ds")]
pub fn render_card_detail(
    card_id: i16,
    card_db: &CardDatabase,
    atlas: &CardAtlas,
    scroll_y: f32,
) {
    if let Some(card) = card_db.get_card(card_id) {
        let total_blade = card.blade as i32;
        let score = card.score.unwrap_or(0) as i32;
        let cost = card.cost.unwrap_or(0);
        let heart_str = build_heart_str(
            &card
                .base_heart
                .as_ref()
                .map(|bh| bh.hearts.clone())
                .unwrap_or_default(),
            card_id,
            &Default::default(),
            false,
        );
        let need_heart_str = build_heart_str(
            &card
                .need_heart
                .as_ref()
                .map(|bh| bh.hearts.clone())
                .unwrap_or_default(),
            card_id,
            &Default::default(),
            true,
        );
        let stats = CardDisplayStats {
            total_blade,
            heart_str,
            need_heart_str,
            score,
            cost,
            is_tapped: false,
        };

        // Layout: 400x240 top screen. Header bar spans full width; the card
        // portrait fills the left column (nearly the full height below the
        // header) and the ability text sits in the right column.
        const HEADER_H: f32 = 40.0;
        let card_h = 240.0 - HEADER_H - 12.0; // ~188px tall
        let card_w = card_h * 0.711; // ~134px portrait
        let card_x = 6.0;
        let card_y = HEADER_H + 6.0; // 46
        let text_x = card_x + card_w + 10.0; // ~150
        let text_w = 400.0 - text_x - 6.0; // ~244

        unsafe {
            _3ds_top_queue_rect(0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
            _3ds_top_queue_rect(0.0, 0.0, 400.0, HEADER_H, COL_CARD);
            let display_name = i18n::card_display_name(&card.name, current_lang());
            _3ds_top_queue_text(
                4.0,
                4.0,
                COL_BLUE,
                0.80f32,
                format!("[{}] {}\0", card.card_no, display_name).as_ptr(),
            );
            render_text_with_icons(
                4.0,
                24.0,
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
            // Content background below the header
            _3ds_top_queue_rect(0.0, HEADER_H, 400.0, 240.0 - HEADER_H, COL_CARD);
            // Card portrait (left column)
            _3ds_top_queue_rect(card_x - 2.0, card_y - 2.0, card_w + 4.0, card_h + 4.0, COL_GOLD);
            draw_card_image(&card.card_no, atlas, card_x, card_y, card_w, card_h);
            // Scrollable ability text (right column)
            let mut ty = card_y - scroll_y;
            let abs: Vec<_> = card.resolved_abilities().collect();
            if abs.is_empty() {
                let raw = card.ability_text();
                if !raw.is_empty() {
                    let clean = raw.replace('\n', " ");
                    let w = wrap_ability_text(&clean, text_w, 0.65);
                    for line in w.lines() {
                        if ty > -20.0 && ty < 240.0 {
                            render_text_with_icons(text_x, ty, line, COL_LIGHT, 0.65);
                        }
                        ty += 18.0;
                    }
                }
            } else {
                for ab in abs {
                    let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
                    let w = wrap_ability_text(&ab_text, text_w, 0.65);
                    for line in w.lines() {
                        if ty > -20.0 && ty < 240.0 {
                            render_text_with_icons(text_x, ty, line, COL_LIGHT, 0.65);
                        }
                        ty += 18.0;
                    }
                    ty += 3.0;
                }
            }
            // Scroll indicator if content extends beyond screen
            let arrow_x = 400.0 - 18.0;
            if ty > 220.0 {
                _3ds_top_queue_text(arrow_x, 225.0, COL_MED, 0.50f32, format!("v\0").as_ptr());
            }
            if scroll_y > 0.0 {
                _3ds_top_queue_text(arrow_x, 42.0, COL_MED, 0.50f32, format!("^\0").as_ptr());
            }
        }
    }
}
