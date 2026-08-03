use crate::ffi;

pub const KEY_A: u32 = 1 << 0;
pub const KEY_B: u32 = 1 << 1;
pub const KEY_SELECT: u32 = 1 << 2;
pub const KEY_START: u32 = 1 << 3;
pub const KEY_RIGHT: u32 = 1 << 4;
pub const KEY_LEFT: u32 = 1 << 5;
pub const KEY_UP: u32 = 1 << 6;
pub const KEY_DOWN: u32 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

pub struct Input {
    prev: u32,
    curr: u32,
}

impl Input {
    pub fn new() -> Self {
        Input { prev: 0, curr: 0 }
    }

    pub fn poll(&mut self) {
        self.prev = self.curr;
        unsafe { ffi::nds_scan_keys() }
        self.curr = unsafe { ffi::nds_keys_held() as u32 };
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let mask = button_to_mask(btn);
        (self.curr & mask) != 0 && (self.prev & mask) == 0
    }

    pub fn held(&self, btn: Button) -> bool {
        (self.curr & button_to_mask(btn)) != 0
    }
}

fn button_to_mask(btn: Button) -> u32 {
    match btn {
        Button::Up => KEY_UP,
        Button::Down => KEY_DOWN,
        Button::Left => KEY_LEFT,
        Button::Right => KEY_RIGHT,
        Button::A => KEY_A,
        Button::B => KEY_B,
        Button::Start => KEY_START,
        Button::Select => KEY_SELECT,
    }
}
