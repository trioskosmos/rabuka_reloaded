use alloc::{string::String, vec::Vec};

use agb::display::font::{Font, Layout, LayoutSettings, ObjectTextRenderer};
use agb::display::object::{Object, Size};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Rgb15};

static FONT: Font = agb::include_font!("assets/pixelated.ttf", 10);

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
        // Small 16x16 sprites (4 VRAM tiles each) so any single allocation
        // always fits and groups stay within the sprite bounds (the default
        // max_group_width is 16). This is the proven object-text size used by
        // the real agb examples/games.
        let text_renderer = ObjectTextRenderer::new((&PALETTE).into(), Size::S16x16);
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

        let settings = LayoutSettings::new();
        let layout = Layout::new(&self.buf, &FONT, &settings);

        let mut new_objects = Vec::new();
        // Safety cap: each 16x16 sprite costs 4 tiles; the sprite allocator has
        // 1024 tiles, so a single screen must stay well under 256 groups.
        for group in layout.take(240) {
            new_objects.push(self.text_renderer.show(&group, (4, 2)));
        }
        self.text_objects = new_objects;

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
