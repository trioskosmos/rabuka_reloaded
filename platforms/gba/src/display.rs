use alloc::{string::String, vec::Vec};

use agb::display::font::{Font, Layout, LayoutSettings, ObjectTextRenderer};
use agb::display::object::{Object, Size};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Rgb15};

static FONT: Font = agb::include_font!("assets/yoster_ja.ttf", 12);

/// S32x16 sprites cost 8 tiles each. With single-frame rendering the previous
/// screen's sprites are still in VRAM while the new ones allocate, so we need
/// 2 * groups * 8 < 1024 -> groups <= 64. 60 groups * 8 = 480 tiles per screen,
/// ~180 chars, plenty for the game's menus. This keeps a single fresh frame
/// (no ghost-sprite artifacts) with no sprite-VRAM AllocError.
const MAX_GROUPS: usize = 60;

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
        // 32x16 sprites + max_group_width(32): group width is in pixels, so 32px
        // packs ~3 chars per object. This is the proven fast config from the real
        // agb games — much faster than 16x16 (1 char/object) while keeping the
        // pixelated font the player prefers.
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
        // input loops. If nothing changed, skip entirely.
        if self.buf == self.last {
            return;
        }
        self.last = self.buf.clone();

        // Drop the previous screen's sprite objects, then take ONE fresh frame.
        // A single frame starts with an empty OAM (no ghost sprites) and, because
        // MAX_GROUPS is kept low enough, the new sprites fit in VRAM alongside the
        // old ones' (freed on drop) — so we don't need the double-frame trick.
        self.text_objects.clear();
        let mut frame = self.gfx.frame();

        let settings = LayoutSettings::new()
            .with_max_group_width(32)
            .with_max_line_length(230);
        let layout = Layout::new(&self.buf, &FONT, &settings);

        self.text_objects
            .extend(layout.take(MAX_GROUPS).map(|group| self.text_renderer.show(&group, (4, 2))));

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
