use core::ptr;

// Jaguar Jagpad: buttons at $F14000, D-pad at $F14002.
const JAGPAD: *mut u16 = 0x00F1_4000 as *mut u16;
const JOYDIR: *mut u16 = 0x00F1_4002 as *mut u16;

#[derive(Clone, Copy, PartialEq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    C,
    Pause,
    Option,
}

pub struct Input {
    prev: u32,
    curr: u32,
}

impl Input {
    pub fn new() -> Input {
        let curr = read_state();
        Input { prev: curr, curr }
    }

    pub fn poll(&mut self) {
        self.prev = self.curr;
        self.curr = read_state();
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let bit = button_bit(btn);
        (self.curr & bit) != 0 && (self.prev & bit) == 0
    }

    pub fn held(&self, btn: Button) -> bool {
        let bit = button_bit(btn);
        (self.curr & bit) != 0
    }
}

fn read_state() -> u32 {
    let buttons = unsafe { ptr::read_volatile(JAGPAD) };
    let dir = unsafe { ptr::read_volatile(JOYDIR) };
    let mut state = 0u32;
    if buttons & 0x0001 != 0 {
        state |= button_bit(Button::A);
    }
    if buttons & 0x0002 != 0 {
        state |= button_bit(Button::B);
    }
    if buttons & 0x0004 != 0 {
        state |= button_bit(Button::C);
    }
    if buttons & 0x0008 != 0 {
        state |= button_bit(Button::Pause);
    }
    if buttons & 0x8000 != 0 {
        state |= button_bit(Button::Option);
    }
    if dir & 0x0001 != 0 {
        state |= button_bit(Button::Up);
    }
    if dir & 0x0002 != 0 {
        state |= button_bit(Button::Down);
    }
    if dir & 0x0004 != 0 {
        state |= button_bit(Button::Left);
    }
    if dir & 0x0008 != 0 {
        state |= button_bit(Button::Right);
    }
    state
}

fn button_bit(btn: Button) -> u32 {
    match btn {
        Button::A => 1 << 0,
        Button::B => 1 << 1,
        Button::C => 1 << 2,
        Button::Pause => 1 << 3,
        Button::Option => 1 << 4,
        Button::Up => 1 << 8,
        Button::Down => 1 << 9,
        Button::Left => 1 << 10,
        Button::Right => 1 << 11,
    }
}
