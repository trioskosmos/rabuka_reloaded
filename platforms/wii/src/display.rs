use alloc::vec::Vec;

extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
}

pub struct Display;
impl Display {
    pub fn new() -> Self {
        Display
    }
    pub fn clear(&mut self) {
        unsafe {
            printf("\x1b[2J\x1b[;0H\0".as_ptr());
        }
    }
    pub fn print(&mut self, text: &str) {
        let v = to_c(text);
        unsafe {
            printf(v.as_ptr());
        }
    }
    pub fn println(&mut self, text: &str) {
        let s = to_c(&alloc::format!("{}\n", text));
        unsafe {
            printf(s.as_ptr());
        }
    }
    pub fn swap_buffers(&mut self) {}
    pub fn wait_vsync(&self) {
        unsafe {
            extern "C" {
                fn VIDEO_WaitVSync();
            }
            VIDEO_WaitVSync();
        }
    }
    pub fn draw_menu(&mut self, items: &[&str], sel: usize, title: &str) {
        self.clear();
        self.println(title);
        self.println("-------------------");
        for (i, item) in items.iter().enumerate() {
            let p = if i == sel { " >" } else { "  " };
            self.println(&alloc::format!("{p} {item}"));
        }
    }
}
fn to_c(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().collect();
    v.push(0);
    v
}
