extern "C" {
    fn PAD_ScanPads();
    fn PAD_ButtonsHeld(port: i32) -> u16;
}

const PAD_UP: u16 = 0x0008;
const PAD_DOWN: u16 = 0x0004;
const PAD_LEFT: u16 = 0x0001;
const PAD_RIGHT: u16 = 0x0002;
const PAD_A: u16 = 0x0100;
const PAD_B: u16 = 0x0200;
const PAD_X: u16 = 0x0400;
const PAD_Y: u16 = 0x0800;
const PAD_START: u16 = 0x1000;

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
}

pub struct Input {
    prev: u16,
    curr: u16,
}

impl Input {
    pub fn new() -> Self {
        Input { prev: 0, curr: 0 }
    }

    pub fn poll(&mut self) {
        unsafe {
            PAD_ScanPads();
        }
        self.prev = self.curr;
        self.curr = unsafe { PAD_ButtonsHeld(0) };
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let mask = button_mask(btn);
        self.prev & mask == 0 && self.curr & mask != 0
    }

    pub fn is_held(&self, btn: Button) -> bool {
        let mask = button_mask(btn);
        self.curr & mask != 0
    }
}

fn button_mask(btn: Button) -> u16 {
    match btn {
        Button::Up => PAD_UP,
        Button::Down => PAD_DOWN,
        Button::Left => PAD_LEFT,
        Button::Right => PAD_RIGHT,
        Button::A => PAD_A,
        Button::B => PAD_B,
        Button::X => PAD_X,
        Button::Y => PAD_Y,
        Button::Start => PAD_START,
    }
}
