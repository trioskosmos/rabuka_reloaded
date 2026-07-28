const KEY_A: u32 = 1 << 0;
const KEY_B: u32 = 1 << 1;
const KEY_SELECT: u32 = 1 << 2;
const KEY_START: u32 = 1 << 3;
const KEY_RIGHT: u32 = 1 << 4;
const KEY_LEFT: u32 = 1 << 5;
const KEY_UP: u32 = 1 << 6;
const KEY_DOWN: u32 = 1 << 7;
const KEY_R: u32 = 1 << 8;
const KEY_L: u32 = 1 << 9;
const KEY_X: u32 = 1 << 10;
const KEY_Y: u32 = 1 << 11;

extern "C" {
    fn nds_scan_keys();
    fn nds_key_held() -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    Start,
    Select,
    L,
    R,
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
        unsafe { nds_scan_keys() }
        self.curr = unsafe { nds_key_held() };
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let mask = button_to_mask(btn);
        (self.curr & mask) != 0 && (self.prev & mask) == 0
    }
}
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
        Button::X => KEY_X,
        Button::Y => KEY_Y,
        Button::Start => KEY_START,
        Button::Select => KEY_SELECT,
        Button::L => KEY_L,
        Button::R => KEY_R,
    }
}
