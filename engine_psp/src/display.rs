use embedded_graphics::{
    geometry::Size,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use psp::sys;

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 272;
pub const FONT_H: u32 = 10;
pub const FONT_W: u32 = 6;
pub const COLS: u32 = WIDTH / FONT_W;
pub const ROWS: u32 = HEIGHT / FONT_H;

pub struct Display {
    front: &'static mut [u32],
    back: &'static mut [u32],
    cursor_x: i32,
    cursor_y: i32,
    style: MonoTextStyle<'static, Rgb888>,
    bg: Rgb888,
}

impl Display {
    pub fn new() -> Self {
        let vram_base = unsafe { sys::sceGeEdramGetAddr() as u32 };
        let buf_size = (BUF_WIDTH * HEIGHT) as usize;
        let front_buf = unsafe {
            core::slice::from_raw_parts_mut((0x4000_0000 | vram_base) as *mut u32, buf_size)
        };
        let back_offset = vram_base + (BUF_WIDTH * HEIGHT * 4);
        let back_buf = unsafe {
            core::slice::from_raw_parts_mut((0x4000_0000 | back_offset) as *mut u32, buf_size)
        };

        unsafe {
            sys::sceDisplaySetMode(sys::DisplayMode::Lcd, 480, 272);
            sys::sceDisplaySetFrameBuf(
                front_buf.as_ptr() as *const u8,
                BUF_WIDTH as usize,
                sys::DisplayPixelFormat::Psm8888,
                sys::DisplaySetBufSync::NextFrame,
            );
        }

        let bg = Rgb888::new(0, 0, 0);
        let style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(255, 255, 255));
        let mut disp = Display {
            front: front_buf,
            back: back_buf,
            cursor_x: 0,
            cursor_y: 0,
            style,
            bg,
        };
        disp.clear();
        disp.swap_buffers();
        disp.clear();
        disp
    }

    pub fn clear(&mut self) {
        for pixel in self.back.iter_mut() {
            *pixel = 0;
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn print(&mut self, text: &str) {
        let x = self.cursor_x;
        let y = self.cursor_y;
        let style = self.style;
        Text::new(text, Point::new(x, y), style)
            .draw(&mut BackBuffer {
                pixels: self.back,
                stride: BUF_WIDTH,
            })
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
            .draw(&mut BackBuffer {
                pixels: self.back,
                stride: BUF_WIDTH,
            })
            .ok();
    }

    pub fn clear_line(&mut self, row: u32) {
        let y = (row * FONT_H) as i32;
        Rectangle::new(Point::new(0, y), Size::new(WIDTH, FONT_H))
            .into_styled(PrimitiveStyleBuilder::new().fill_color(self.bg).build())
            .draw(&mut BackBuffer {
                pixels: self.back,
                stride: BUF_WIDTH,
            })
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
        unsafe {
            sys::sceDisplaySetFrameBuf(
                self.back.as_ptr() as *const u8,
                BUF_WIDTH as usize,
                sys::DisplayPixelFormat::Psm8888,
                sys::DisplaySetBufSync::NextFrame,
            );
        }
        core::mem::swap(&mut self.front, &mut self.back);
    }

    fn scroll(&mut self) {
        self.cursor_y = (HEIGHT as i32 / 2) as i32;
    }
}

const BUF_WIDTH: u32 = 512;

struct BackBuffer {
    pixels: *mut [u32],
    stride: u32,
}

impl BackBuffer {
    fn write_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < WIDTH && y < HEIGHT {
            let idx = (y * self.stride + x) as isize;
            unsafe {
                *(*self.pixels).as_mut_ptr().offset(idx) = color;
            }
        }
    }
}

impl DrawTarget for BackBuffer {
    type Error = core::convert::Infallible;
    type Color = Rgb888;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for p in pixels.into_iter() {
            let Pixel(coord, color) = p;
            if coord.x >= 0 && coord.y >= 0 {
                self.write_pixel(
                    coord.x as u32,
                    coord.y as u32,
                    (color.r() as u32) | ((color.g() as u32) << 8) | ((color.b() as u32) << 16),
                );
            }
        }
        Ok(())
    }
}

impl OriginDimensions for BackBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}
