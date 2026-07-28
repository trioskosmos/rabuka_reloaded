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
}
fn to_c(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().collect();
    v.push(0);
    v
}
