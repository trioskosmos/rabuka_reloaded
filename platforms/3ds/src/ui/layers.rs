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

use crate::ffi::{
    _3ds_set_layer_depth, _3ds_top_queue_card_depth, _3ds_top_queue_card_selected,
    _3ds_top_queue_rect_depth, _3ds_top_queue_text_depth,
};
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

impl Layer {
    /// Stereo depth for this layer: 0.0 = screen plane (sharp) .. 1.0 = max pop.
    /// Header/Hint text is pinned to the plane so small glyphs never ghost;
    /// content floats slightly, cards pop (see ui::stereo for the rationale).
    /// Must stay in sync with `stereo_layer_depth` in ctru_shim.c.
    pub fn default_depth(self) -> f32 {
        match self {
            Layer::Background => 0.05,
            Layer::Content => 0.15,
            Layer::BodyText => 0.10,
            Layer::Cover => 0.10,
            Layer::Header => 0.0,
            Layer::Hint => 0.0,
        }
    }

    /// Push this layer's depth to the C renderer (applies from the next frame).
    pub fn apply_depth(self) {
        unsafe {
            _3ds_set_layer_depth(self as i32, self.default_depth());
        }
    }

    /// Depth for a raw layer rank (used by `Painter::flush`, which only keeps
    /// the rank). Out-of-range ranks fall back to the screen plane.
    pub fn depth_for_rank(rank: u8) -> f32 {
        match rank {
            0 => Layer::Background.default_depth(),
            1 => Layer::Content.default_depth(),
            2 => Layer::BodyText.default_depth(),
            3 => Layer::Cover.default_depth(),
            4 => Layer::Header.default_depth(),
            _ => Layer::Hint.default_depth(),
        }
    }
}

/// One queued draw primitive. Cards carry an explicit stereo depth so the
/// focused card can pop above the rest; rects/texts inherit their layer depth.
enum Op {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
    },
    RectDepth {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
        depth: f32,
    },
    Text {
        x: f32,
        y: f32,
        color: u32,
        scale: f32,
        text: String,
    },
    Card {
        atlas: String,
        idx: i32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        depth: f32,
    },
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

    /// Queue a rect with an explicit stereo depth. Use for frames directly
    /// around a card image so frame and card share one depth and stay
    /// registered on both eyes (e.g. the detail-portrait gold frame).
    pub fn rect_with_depth(
        &mut self,
        layer: Layer,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
        depth: f32,
    ) {
        self.ops.push((
            layer as u8,
            Op::RectDepth {
                x,
                y,
                w,
                h,
                color,
                depth,
            },
        ));
    }

    pub fn text(&mut self, layer: Layer, x: f32, y: f32, color: u32, scale: f32, text: &str) {
        self.ops.push((
            layer as u8,
            Op::Text {
                x,
                y,
                color,
                scale,
                text: text.to_string(),
            },
        ));
    }

    pub fn card(&mut self, layer: Layer, atlas: &str, idx: i32, x: f32, y: f32, w: f32, h: f32) {
        // Resting card depth: floats above the content panel behind it.
        self.card_with_depth(layer, atlas, idx, x, y, w, h, crate::ui::stereo::CARD_DEPTH);
    }

    /// Queue a card image with an explicit stereo depth (0.0..1.0).
    /// Use `stereo::SELECTED_DEPTH` for the cursor/focused card so it pops,
    /// `stereo::PORTRAIT_DEPTH` for large showcase portraits.
    pub fn card_with_depth(
        &mut self,
        layer: Layer,
        atlas: &str,
        idx: i32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        depth: f32,
    ) {
        self.ops.push((
            layer as u8,
            Op::Card {
                atlas: atlas.to_string(),
                idx,
                x,
                y,
                w,
                h,
                depth,
            },
        ));
    }

    /// Queue the focused/selected card: full pop depth + drop shadow.
    pub fn card_selected(
        &mut self,
        layer: Layer,
        atlas: &str,
        idx: i32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        self.card_with_depth(
            layer,
            atlas,
            idx,
            x,
            y,
            w,
            h,
            crate::ui::stereo::SELECTED_DEPTH,
        );
    }

    /// Re-emit queued ops to the C queue in ascending layer order. Within a
    /// layer, ops keep insertion order (stable). Ops are drained, so a painter
    /// can be reused for the next frame (the C queue is cleared each frame).
    /// Stereo depths ride along per op: rects/texts use their layer depth,
    /// cards use the depth they were queued with.
    pub fn flush(&mut self) {
        // Stable sort by layer so intra-layer order is preserved.
        self.ops.sort_by_key(|(rank, _)| *rank);
        for (rank, op) in self.ops.drain(..) {
            // Layer enum discriminants (Background=0..Hint=5) line up with the
            // C-side stereo_layer_depth table; depth = f(rank) keeps the two
            // sides in sync without duplicating the table here.
            let layer_depth = Layer::depth_for_rank(rank);
            match op {
                Op::Rect { x, y, w, h, color } => unsafe {
                    _3ds_top_queue_rect_depth(x, y, w, h, color, layer_depth);
                },
                Op::RectDepth {
                    x,
                    y,
                    w,
                    h,
                    color,
                    depth,
                } => unsafe {
                    _3ds_top_queue_rect_depth(x, y, w, h, color, depth);
                },
                Op::Text {
                    x,
                    y,
                    color,
                    scale,
                    text,
                } => {
                    let c = CString::new(text).unwrap_or_default();
                    unsafe {
                        _3ds_top_queue_text_depth(
                            x,
                            y,
                            color,
                            scale,
                            c.as_ptr() as *const u8,
                            layer_depth,
                        );
                    }
                }
                Op::Card {
                    atlas,
                    idx,
                    x,
                    y,
                    w,
                    h,
                    depth,
                } => {
                    let c = CString::new(atlas).unwrap_or_default();
                    unsafe {
                        if depth >= crate::ui::stereo::SELECTED_DEPTH {
                            _3ds_top_queue_card_selected(c.as_ptr() as *const u8, idx, x, y, w, h);
                        } else {
                            _3ds_top_queue_card_depth(
                                c.as_ptr() as *const u8,
                                idx,
                                x,
                                y,
                                w,
                                h,
                                depth,
                            );
                        }
                    }
                }
            }
        }
    }
}
