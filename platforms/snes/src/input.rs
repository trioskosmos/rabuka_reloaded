use crate::sneslib;

#[derive(Clone, Copy, PartialEq)]
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

pub struct Input;

impl Input {
    pub fn new() -> Self {
        Input
    }

    pub fn poll(&mut self) {
        // pad_keys is updated by the crt0 NMI handler; nothing to do here.
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let mask = mask(btn);
        sneslib::pressed(0, mask)
    }

    pub fn held(&self, btn: Button) -> bool {
        sneslib::held(0, mask(btn))
    }
}

fn mask(btn: Button) -> u16 {
    match btn {
        Button::Up => sneslib::KEY_UP,
        Button::Down => sneslib::KEY_DOWN,
        Button::Left => sneslib::KEY_LEFT,
        Button::Right => sneslib::KEY_RIGHT,
        Button::A => sneslib::KEY_A,
        Button::B => sneslib::KEY_B,
        Button::Start => sneslib::KEY_START,
        Button::Select => sneslib::KEY_SELECT,
    }
}
