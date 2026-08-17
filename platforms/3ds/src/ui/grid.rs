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
#[cfg(feature = "3ds")]
use crate::ui::layers::Layer;
#[cfg(feature = "3ds")]
use crate::ui::layers::Painter;
use crate::ui::colors::COL_BLUE;
use crate::ui::colors::COL_CARD;
use crate::ui::colors::COL_CARD_OPAQUE;
use crate::ui::colors::COL_GOLD;
use crate::ui::colors::COL_LIGHT;
use crate::ui::colors::COL_MED;
use crate::ui::colors::COL_TOP_BG;
use crate::ui::text::build_heart_str;
use crate::ui::text::card_stat_line;
use crate::ui::text::wrap_ability_text;
use crate::ui::text::CardDisplayStats;
use crate::ui::text::{SCALE_BODY, SCALE_LARGE, SCALE_SMALL};

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
                SCALE_SMALL,
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
        // NOTE: this is the card-detail header for the grid/zone viewer, NOT
        // the in-game status header (render.rs HEADER_H = 50). Keep separate.
        const CARD_DETAIL_HEADER_H: f32 = 40.0;
        let card_h = 240.0 - CARD_DETAIL_HEADER_H - 12.0; // ~188px tall
        let card_w = card_h * 0.711; // ~134px portrait
        let card_x = 6.0;
        let card_y = CARD_DETAIL_HEADER_H + 6.0; // 46
        let text_x = card_x + card_w + 10.0; // ~150
        let text_w = 400.0 - text_x - 6.0; // ~244

        let mut p = Painter::new();
        p.rect(Layer::Background, 0.0, 0.0, 400.0, 240.0, COL_TOP_BG);
        p.rect(Layer::Content, 0.0, 0.0, 400.0, CARD_DETAIL_HEADER_H, COL_CARD_OPAQUE);
        let display_name = i18n::card_display_name(&card.name, current_lang());
        let name_label =
            crate::ui::text::truncate_to_width(&format!("[{}] ", card.card_no), &display_name, SCALE_LARGE, 392.0);
        // Single line, let it overflow to the right — a long name must not
        // wrap onto a second line (which would collide with the stat line).
        p.text(Layer::Header, 4.0, 4.0, COL_BLUE, SCALE_LARGE, &name_label);
        p.text(
            Layer::Header,
            4.0,
            24.0,
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
        // Content background below the header
        p.rect(Layer::Content, 0.0, CARD_DETAIL_HEADER_H, 400.0, 240.0 - CARD_DETAIL_HEADER_H, COL_CARD_OPAQUE);
        // Card portrait (left column)
        p.rect(Layer::Content, card_x - 2.0, card_y - 2.0, card_w + 4.0, card_h + 4.0, COL_GOLD);
        if let Some((atl, idx)) = atlas.lookup(&card.card_no) {
            p.card(Layer::Content, atl, *idx as i32, card_x, card_y, card_w, card_h);
        }
        // Scrollable ability text (right column). Text may scroll up under the
        // header: it's drawn on the BodyText layer (below Cover), then the Cover
        // layer is painted opaque over the header region of the right column so
        // scrolled text is hidden beneath the header. The id/name label lives on
        // the Header layer which flushes above the cover — no fragile ordering.
        let text_top = card_y;
        let mut ty = text_top - scroll_y;
        let abs: Vec<_> = card.resolved_abilities().collect();
        if abs.is_empty() {
            let raw = card.ability_text();
            if !raw.is_empty() {
                let clean = raw.replace('\n', " ");
                let w = wrap_ability_text(&clean, text_w, SCALE_BODY);
                for line in w.lines() {
                    if ty > -20.0 && ty < 240.0 {
                        p.text(Layer::BodyText, text_x, ty, COL_LIGHT, SCALE_BODY, line);
                    }
                    ty += 18.0;
                }
            }
        } else {
            for ab in abs {
                let ab_text = i18n::translate_ability(&ab.full_text, current_lang());
                let w = wrap_ability_text(&ab_text, text_w, SCALE_BODY);
                for line in w.lines() {
                    if ty > -20.0 && ty < 240.0 {
                        p.text(Layer::BodyText, text_x, ty, COL_LIGHT, SCALE_BODY, line);
                    }
                    ty += 18.0;
                }
                ty += 3.0;
            }
        }
        // Clip: opaque cover over the header region of the right column on the
        // Cover layer (hides scrolled BodyText). The id/name on the Header layer
        // is flushed above it automatically.
        p.rect(Layer::Cover, text_x, 0.0, 400.0 - text_x, text_top, COL_CARD_OPAQUE);
        p.text(Layer::Header, text_x, 4.0, COL_BLUE, SCALE_LARGE, &name_label);
        // Scroll indicator if content extends beyond screen (right edge,
        // clear of the portrait so it doesn't overlap the card image).
        let arrow_x = 400.0 - 18.0;
        if ty > 220.0 {
            p.text(Layer::Header, arrow_x, 225.0, COL_MED, SCALE_SMALL, "v");
        }
        if scroll_y > 0.0 {
            p.text(Layer::Header, arrow_x, 42.0, COL_MED, SCALE_SMALL, "^");
        }
        p.flush();
    }
}
