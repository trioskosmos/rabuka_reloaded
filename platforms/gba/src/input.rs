use agb::input::{Button as GbaButton, ButtonController};

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

pub struct Input {
    ctrl: ButtonController,
}

impl Input {
    pub fn new() -> Self {
        Input {
            ctrl: ButtonController::new(),
        }
    }

    pub fn poll(&mut self) {
        self.ctrl.update();
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        self.ctrl.is_just_pressed(to_gba(btn))
    }

    pub fn held(&self, btn: Button) -> bool {
        self.ctrl.is_pressed(to_gba(btn))
    }
}

fn to_gba(btn: Button) -> GbaButton {
    match btn {
        Button::Up => GbaButton::Up,
        Button::Down => GbaButton::Down,
        Button::Left => GbaButton::Left,
        Button::Right => GbaButton::Right,
        Button::A => GbaButton::A,
        Button::B => GbaButton::B,
        Button::Select => GbaButton::Select,
        Button::Start => GbaButton::Start,
    }
}
