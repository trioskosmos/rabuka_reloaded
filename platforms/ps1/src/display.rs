use psx::gpu::colors::BLACK;
use psx::gpu::VideoMode;
use psx::sys::event::{Event, Poll};
use psx::{Framebuffer, TextBox};

/// Text display on the PS1: double-buffered 320x240 framebuffer + the default
/// font text box (the same rendering path as the psx-sdk-rs examples).
pub struct Display {
    fb: Framebuffer,
    txt: TextBox,
    vblank: Event<Poll>,
}

impl Display {
    pub fn new() -> Display {
        let buf0 = (0, 0);
        let buf1 = (0, 240);
        let res = (320, 240);
        let mut fb =
            Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).expect("framebuffer init");
        let font = fb.load_default_font();
        let txt = font.new_text_box((0, 8), res);
        // The game uses the BIOS gamepad handler, whose VBlank IRQ handling
        // conflicts with Framebuffer::wait_vblank's raw-IRQ spin (it never
        // clears, hanging the game at ~0 FPS). Poll the BIOS VBlank event
        // instead, exactly like the SDK's gamepad example.
        let vblank = Event::<Poll>::new(0xF2000003, 0x0002).expect("vblank event");
        Display { fb, txt, vblank }
    }

    pub fn clear(&mut self) {
        self.fb.set_bg_color(BLACK);
        self.txt.reset();
    }

    pub fn print(&mut self, text: &str) {
        for &b in text.as_bytes() {
            match b {
                b'\n' => self.txt.newline(),
                0x20..=0x7E => self.txt.print_char(b),
                _ => {}
            }
        }
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        self.txt.newline();
    }

    pub fn swap_buffers(&mut self) {
        self.fb.draw_sync();
        self.vblank.wait();
        self.fb.swap();
    }

    /// Wait for the next VBlank without swapping buffers. Used by `wait_vblank`
    /// so a frame is paced but not double-swapped (a second swap per frame
    /// alternates which buffer holds the text, causing a flicker).
    pub fn wait(&mut self) {
        self.vblank.wait();
    }

    pub fn clear_line(&mut self, _row: usize) {
        // No per-line erase; redraw the whole frame (text games redraw each frame).
    }
}
