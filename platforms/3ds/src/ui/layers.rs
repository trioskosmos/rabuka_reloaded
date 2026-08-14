// Layer-ordered top-screen painter.
//
// The C-side draw-op queue (`_3ds_top_queue_*`) is a plain FIFO: a text op drawn
// after a rect op sits ON TOP of the rect. To put a header label above scrolling
// body text we previously had to draw the text, then paint an opaque cover rect,
// then re-draw the label — a fragile "cover-and-redraw" dance that caused the
// bleed-through bugs (e.g. COL_CARD being semi-transparent).
//
// This painter removes that confusion. You accumulate primitive ops tagged with a
// semantic LAYER (Bottom->Top), then call flush() which re-emits them to the C
// queue in a fixed layer order. Picking the right layer is all you ever need to
// think about; the stack order is decided here in one place.

use crate::ffi::{_3ds_top_queue_card, _3ds_top_queue_rect, _3ds_top_queue_text};
use std::ffi::CString;

/// Semantic layers, ordered bottom-to-top. Add new layers here as needed;
/// the painter always flushes them in this enum-declaration order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum Layer {
    /// Full-screen / zone background rects.
    Background = 0,
    /// Content panels, card backdrops, grid cells, borders.
    Content = 1,
    /// Scrollable ability/description body text (may extend under the header).
    BodyText = 2,
    /// Opaque cover rects that hide BodyText scrolled under a header area.
    Cover = 3,
    /// Header text: card id/name, stats, labels drawn over the cover.
    Header = 4,
    /// Bottom-of-screen hint bar and page indicators (always on top).
    Hint = 5,
}

/// One queued draw primitive.
enum Op {
    Rect { x: f32, y: f32, w: f32, h: f32, color: u32 },
    Text { x: f32, y: f32, color: u32, scale: f32, text: String },
    Card { atlas: String, idx: i32, x: f32, y: f32, w: f32, h: f32 },
}

/// Top-screen painter that gathers ops per layer and flushes in layer order.
pub struct Painter {
    ops: Vec<(u8, Op)>,
}

impl Painter {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn rect(&mut self, layer: Layer, x: f32, y: f32, w: f32, h: f32, color: u32) {
        self.ops.push((layer as u8, Op::Rect { x, y, w, h, color }));
    }

    pub fn text(&mut self, layer: Layer, x: f32, y: f32, color: u32, scale: f32, text: &str) {
        self.ops
            .push((layer as u8, Op::Text { x, y, color, scale, text: text.to_string() }));
    }

    pub fn card(&mut self, layer: Layer, atlas: &str, idx: i32, x: f32, y: f32, w: f32, h: f32) {
        self.ops
            .push((layer as u8, Op::Card { atlas: atlas.to_string(), idx, x, y, w, h }));
    }

    /// Re-emit queued ops to the C queue in ascending layer order. Within a
    /// layer, ops keep insertion order (stable). Ops are drained, so a painter
    /// can be reused for the next frame (the C queue is cleared each frame).
    pub fn flush(&mut self) {
        // Stable sort by layer so intra-layer order is preserved.
        self.ops.sort_by_key(|(rank, _)| *rank);
        for (_, op) in self.ops.drain(..) {
            match op {
                Op::Rect { x, y, w, h, color } => unsafe {
                    _3ds_top_queue_rect(x, y, w, h, color);
                },
                Op::Text { x, y, color, scale, text } => {
                    let c = CString::new(text).unwrap_or_default();
                    unsafe {
                        _3ds_top_queue_text(x, y, color, scale, c.as_ptr() as *const u8);
                    }
                }
                Op::Card { atlas, idx, x, y, w, h } => {
                    let c = CString::new(atlas).unwrap_or_default();
                    unsafe {
                        _3ds_top_queue_card(c.as_ptr() as *const u8, idx, x, y, w, h);
                    }
                }
            }
        }
    }
}