use psp::sys::*;

#[derive(Debug, Clone, Copy, PartialEq)]
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
    LTrigger,
    RTrigger,
}

pub struct Input {
    prev: CtrlButtons,
    curr: CtrlButtons,
    initialized: bool,
}

impl Input {
    pub fn new() -> Self {
        Input {
            prev: CtrlButtons::empty(),
            curr: CtrlButtons::empty(),
            initialized: false,
        }
    }

    pub fn poll(&mut self) {
        self.prev = self.curr;
        let mut pad = core::mem::MaybeUninit::<SceCtrlData>::uninit();
        unsafe {
            if !self.initialized {
                sceCtrlSetSamplingCycle(0);
                sceCtrlSetSamplingMode(CtrlMode::Analog);
                self.initialized = true;
            }
            sceCtrlPeekBufferPositive(pad.as_mut_ptr(), 1);
            self.curr = pad.assume_init().buttons;
        }
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let mask = button_to_mask(btn);
        !self.prev.contains(mask) && self.curr.contains(mask)
    }
}
    }
}

fn button_to_mask(btn: Button) -> CtrlButtons {
    match btn {
        Button::Up => CtrlButtons::UP,
        Button::Down => CtrlButtons::DOWN,
        Button::Left => CtrlButtons::LEFT,
        Button::Right => CtrlButtons::RIGHT,
        Button::Cross => CtrlButtons::CROSS,
        Button::Circle => CtrlButtons::CIRCLE,
        Button::Square => CtrlButtons::SQUARE,
        Button::Triangle => CtrlButtons::TRIANGLE,
        Button::Start => CtrlButtons::START,
        Button::Select => CtrlButtons::SELECT,
        Button::LTrigger => CtrlButtons::LTRIGGER,
        Button::RTrigger => CtrlButtons::RTRIGGER,
    }
}
