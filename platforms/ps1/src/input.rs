use alloc::vec::Vec;
use psx::sys::gamepad::{Button as PsxButton, Gamepad};

#[derive(Clone, Copy, PartialEq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    Cross,
    Circle,
    Square,
    Triangle,
    Start,
    Select,
}

pub struct Input {
    gamepad: Gamepad<'static>,
    prev: Vec<PsxButton>,
    curr: Vec<PsxButton>,
}

impl Input {
    pub fn new() -> Input {
        let mut gamepad = Gamepad::new();
        let curr: Vec<PsxButton> = gamepad.poll_p1().collect();
        Input {
            gamepad,
            prev: curr.clone(),
            curr,
        }
    }

    pub fn poll(&mut self) {
        self.prev = core::mem::replace(
            &mut self.curr,
            self.gamepad.poll_p1().collect::<Vec<PsxButton>>(),
        );
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let b = to_psx(btn);
        self.curr.contains(&b) && !self.prev.contains(&b)
    }

    pub fn held(&self, btn: Button) -> bool {
        self.curr.contains(&to_psx(btn))
    }
}

fn to_psx(btn: Button) -> PsxButton {
    match btn {
        Button::Up => PsxButton::Up,
        Button::Down => PsxButton::Down,
        Button::Left => PsxButton::Left,
        Button::Right => PsxButton::Right,
        Button::Cross => PsxButton::Cross,
        Button::Circle => PsxButton::Circle,
        Button::Square => PsxButton::Square,
        Button::Triangle => PsxButton::Triangle,
        Button::Start => PsxButton::Start,
        Button::Select => PsxButton::Select,
    }
}
