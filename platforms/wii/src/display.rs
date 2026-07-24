use core::ffi::c_char;

extern "C" {
    fn display_init();
    fn display_clear();
    fn display_print(s: *const c_char);
    fn display_swap();
    fn VIDEO_WaitVSync();
}
pub struct Display;
impl Display {
    pub fn new() -> Self {
        unsafe {
            display_init();
        }
        Display
    }
    pub fn clear(&mut self) {
        unsafe {
            display_clear();
        }
    }
    pub fn print(&mut self, text: &str) {
        let mut b: alloc::vec::Vec<u8> = text.bytes().collect();
        b.push(0);
        unsafe {
            display_print(b.as_ptr() as *const c_char);
        }
    }
    pub fn println(&mut self, text: &str) {
        self.print(text);
    }
    pub fn swap_buffers(&mut self) {
        unsafe {
            display_swap();
        }
    }
    pub fn wait_vsync(&self) {
        unsafe {
            VIDEO_WaitVSync();
        }
    }
    pub fn draw_menu(&mut self, items: &[&str], selected: usize, title: &str) {
        self.clear();
        self.println(title);
        self.println("-------------------");
        for (i, item) in items.iter().enumerate() {
            let p = if i == selected { " >" } else { "  " };
            self.println(&alloc::format!("{p} {item}"));
        }
    }
}
