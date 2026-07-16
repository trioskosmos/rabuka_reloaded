use embedded_graphics::{
    geometry::Size,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use psp::embedded_graphics::Framebuffer;

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 272;
pub const FONT_H: u32 = 10;
pub const FONT_W: u32 = 6;
pub const COLS: u32 = WIDTH / FONT_W;
pub const ROWS: u32 = HEIGHT / FONT_H;

pub struct Display {
    framebuffer: Framebuffer,
    cursor_x: i32,
    cursor_y: i32,
    style: MonoTextStyle<'static, Rgb888>,
    bg: Rgb888,
}

impl Display {
    pub fn new() -> Self {
        let mut disp = Framebuffer::new();
        let bg = Rgb888::new(0, 0, 0);
        let style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(255, 255, 255));

        Rectangle::new(Point::new(0, 0), Size::new(WIDTH, HEIGHT))
            .into_styled(PrimitiveStyleBuilder::new().fill_color(bg).build())
            .draw(&mut disp)
            .ok();

        Display {
            framebuffer: disp,
            cursor_x: 0,
            cursor_y: 0,
            style,
            bg,
        }
    }

    pub fn clear(&mut self) {
        let bg = self.bg;
        Rectangle::new(Point::new(0, 0), Size::new(WIDTH, HEIGHT))
            .into_styled(PrimitiveStyleBuilder::new().fill_color(bg).build())
            .draw(&mut self.framebuffer)
            .ok();
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn print(&mut self, text: &str) {
        let x = self.cursor_x;
        let y = self.cursor_y;
        let style = self.style;
        Text::new(text, Point::new(x, y), style)
            .draw(&mut self.framebuffer)
            .ok();
        self.cursor_x = x + (text.len() as u32 * FONT_W) as i32;
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        self.cursor_y += FONT_H as i32;
        self.cursor_x = 0;
        if self.cursor_y >= HEIGHT as i32 {
            self.scroll();
        }
    }

    pub fn write_at(&mut self, row: u32, col: u32, text: &str) {
        let x = (col * FONT_W) as i32;
        let y = (row * FONT_H) as i32;
        Text::new(text, Point::new(x, y), self.style)
            .draw(&mut self.framebuffer)
            .ok();
    }

    pub fn clear_line(&mut self, row: u32) {
        let y = (row * FONT_H) as i32;
        Rectangle::new(Point::new(0, y), Size::new(WIDTH, FONT_H))
            .into_styled(PrimitiveStyleBuilder::new().fill_color(self.bg).build())
            .draw(&mut self.framebuffer)
            .ok();
    }

    pub fn draw_menu(&mut self, items: &[&str], selected: usize, title: &str) {
        self.clear();
        self.println(title);
        self.println(&"─".repeat((WIDTH / FONT_W) as usize));

        for (i, item) in items.iter().enumerate() {
            let prefix = if i == selected { " >" } else { "  " };
            let line = alloc::format!("{prefix} {item}");
            self.println(&line);
        }
        self.println("");
        self.println("  D-Pad: navigate  X: select  O: back");
    }

    pub fn swap_buffers(&mut self) {
        self.framebuffer.swap_buffers();
    }

    fn scroll(&mut self) {
        self.cursor_y = (HEIGHT as i32 / 2) as i32;
    }
}
