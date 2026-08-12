use alloc::{string::String, vec::Vec};

use agb::display::font::{Font, Layout, LayoutSettings, ObjectTextRenderer};
use agb::display::object::{Object, Size};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Rgb15};

static FONT: Font = agb::include_font!("assets/NotoSubset.otf", 10);

/// Hard ceiling on the number of letter-group sprites in a single screen.
///
/// Each letter group is one OAM object, and the GBA OAM only holds 128 objects;
/// `Object::show` silently drops anything beyond 128, which showed up as missing
/// / garbled text (visual artefacts) on long screens. We cap well under the OAM
/// limit. (Each 32x16 sprite costs 8 VRAM tiles, so 120 groups * 8 = 960 tiles,
/// under the 1024-tile sprite budget.)
const MAX_GROUPS: usize = 120;

/// Text display on the GBA (via agb). Text is accumulated into a buffer and
/// rendered as sprites (objects) when `swap_buffers` is called.
///
/// We use [`ObjectTextRenderer`] rather than a tiled background renderer
/// because the background renderer allocates a fresh `DynamicTile16` per glyph
/// position in VRAM, which quickly exhausts the small video RAM budget for a
/// text-heavy game. Object text reuses shared font glyph tiles and each glyph
/// is an object we can simply drop between screens.
pub struct Display<'a> {
    gfx: Graphics<'a>,
    buf: String,
    last: String,
    text_renderer: ObjectTextRenderer,
    text_objects: Vec<Object>,
}

impl<'a> Display<'a> {
    pub fn new(gfx: Graphics<'a>) -> Self {
        static PALETTE: Palette16 = const {
            let mut palette = [Rgb15::BLACK; 16];
            palette[1] = Rgb15::WHITE;
            Palette16::new(palette)
        };
        // Wide 32x16 sprites so each object packs ~3 characters (agb's
        // max_group_width is measured in pixels; the default 16 with 16x16
        // sprites made every character its own OAM object, so text screens blew
        // past the 128-object OAM cap -> dropped objects (artefacts) and slow
        // re-renders on cursor moves). Group width 32 matches the 32px sprite;
        // max_line_length wraps long lines instead of running off the screen.
        let text_renderer = ObjectTextRenderer::new((&PALETTE).into(), Size::S32x16);
        Display {
            gfx,
            buf: String::new(),
            last: String::new(),
            text_renderer,
            text_objects: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn println(&mut self, text: &str) {
        self.buf.push_str(text);
        self.buf.push('\n');
    }

    pub fn swap_buffers(&mut self) {
        // The engine re-renders the same (unchanged) buffer every frame in its
        // input loops. If nothing changed, skip entirely: the GBA keeps showing
        // the previous committed frame, so we avoid re-allocating the whole
        // screen of sprite objects every frame.
        if self.buf == self.last {
            return;
        }
        self.last = self.buf.clone();

        // Free the previous screen's sprite VRAM BEFORE allocating the new
        // screen's objects. `gfx.frame()` only moves the previous frame's
        // sprite clones into a "previous" slot; a SECOND `gfx.frame()` call is
        // what actually drops and deallocates them. Calling it twice guarantees
        // the old buffers are free (and the OAM is cleared) before we allocate
        // the new ones, so a big screen never momentarily co-exists with the
        // previous one (which would overflow the 32KB sprite VRAM during
        // re-render).
        self.text_objects.clear();
        drop(self.gfx.frame());
        let mut frame = self.gfx.frame();

        let settings = LayoutSettings::new()
            .with_max_group_width(32)
            .with_max_line_length(220);
        let layout = Layout::new(&self.buf, &FONT, &settings);

        // Reuse the existing Vec so a screen change doesn't allocate a fresh
        // buffer. `MAX_GROUPS` keeps us under both the 128-object OAM cap and
        // the 1024-tile sprite budget, so no groups are silently dropped and no
        // sprite tiles are over-allocated.
        self.text_objects = layout
            .take(MAX_GROUPS)
            .map(|group| self.text_renderer.show(&group, (4, 2)))
            .collect();

        for object in &self.text_objects {
            object.show(&mut frame);
        }
        frame.commit();
    }

    /// Wait for the next VBlank without swapping buffers (used to pace the
    /// menu loops without double-rendering).
    pub fn wait(&mut self) {
        busy_wait_for_vblank();
    }
}
