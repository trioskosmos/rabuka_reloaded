use alloc::{string::String, vec::Vec};

use agb::display::font::{
    Font, Layout, LayoutSettings, RegularBackgroundTextRenderer,
};
use agb::display::tiled::{RegularBackground, RegularBackgroundSize, TileFormat};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Priority, Rgb15};

static FONT: Font = agb::include_font!("assets/NotoSubset.otf", 12);

/// Text display on the GBA via a tiled background font.
///
/// This is the approach real (including Japanese) GBA games use for text: a
/// tiled background layer with a shared glyph tile set written into a tile map.
/// Unlike the sprite/object renderer it has no 128-object cap (so a whole
/// screen of text renders) and updates are cheap tile-map writes (so dpad
/// cursor moves are fast, not re-creating hundreds of sprite objects).
///
/// We use agb's `RegularBackgroundTextRenderer` and recreate it (plus the
/// background) on every screen change. Its `DynamicTile16`s free their VRAM
/// when dropped, so old screens don't accumulate in VRAM.
pub struct Display<'a> {
    gfx: Graphics<'a>,
    bg: RegularBackground,
    buf: String,
    last: String,
}

impl<'a> Display<'a> {
    pub fn new(mut gfx: Graphics<'a>) -> Self {
        static PALETTE: Palette16 = const {
            let mut palette = [Rgb15::BLACK; 16];
            palette[1] = Rgb15::WHITE;
            Palette16::new(palette)
        };
        gfx.set_background_palette(0, &PALETTE);
        Display {
            gfx,
            bg: RegularBackground::new(
                Priority::P0,
                RegularBackgroundSize::Background32x32,
                TileFormat::FourBpp,
            ),
            buf: String::new(),
            last: String::new(),
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

        // Recreate the background + renderer so any previous screen's dynamic
        // tiles are freed from VRAM before we draw the new screen.
        let mut bg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );
        let settings = LayoutSettings::new().with_max_line_length(230);
        let layout = Layout::new(&self.buf, &FONT, &settings);

        let mut text_renderer = RegularBackgroundTextRenderer::new((4, 2), 0);
        for group in layout {
            text_renderer.show(&mut bg, &group);
        }
        self.bg = bg;

        let mut frame = self.gfx.frame();
        self.bg.show(&mut frame);
        frame.commit();
    }

    /// Wait for the next VBlank without swapping buffers.
    pub fn wait(&mut self) {
        busy_wait_for_vblank();
    }
}
