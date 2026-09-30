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
    L,
    R,
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
        // SELECT+L toggles the card-art kill switch. This is the only place the
        // port polls input every frame regardless of which screen the engine
        // has put us on, so the toggle works in a match without the port having
        // to know anything about the engine's loop. See `diag::ART_ENABLED`.
        if self.ctrl.is_just_pressed(GbaButton::L) && self.ctrl.is_pressed(GbaButton::Select) {
            crate::diag::toggle_art();
        }
    }

    /// True while SELECT is held — the modifier for the diagnostic hotkeys.
    pub fn select_held(&self) -> bool {
        self.ctrl.is_pressed(GbaButton::Select)
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        self.ctrl.is_just_pressed(to_gba(btn))
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
        Button::L => GbaButton::L,
        Button::R => GbaButton::R,
    }
}
