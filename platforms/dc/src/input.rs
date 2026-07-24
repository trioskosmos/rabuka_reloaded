use core::ffi::c_int;
use core::ffi::c_void;

#[repr(C)]
struct MapleDevice {
    _opaque: [u8; 0],
}

#[repr(C)]
struct ContState {
    buttons: u32,
    _rest: [u8; 64],
}

extern "C" {
    fn maple_enum_dev(port: c_int, unit: c_int) -> *mut MapleDevice;
    fn maple_dev_status(dev: *mut MapleDevice) -> *mut c_void;
    fn thd_sleep(ms: c_int);
}

const CONT_DPAD_UP: u32 = 1 << 4;
const CONT_DPAD_DOWN: u32 = 1 << 5;
const CONT_DPAD_LEFT: u32 = 1 << 6;
const CONT_DPAD_RIGHT: u32 = 1 << 7;
const CONT_A: u32 = 1 << 2;
const CONT_B: u32 = 1 << 1;
const CONT_X: u32 = 1 << 10;
const CONT_Y: u32 = 1 << 9;
const CONT_START: u32 = 1 << 3;

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
    dev: *mut MapleDevice,
    prev_buttons: u32,
    curr_buttons: u32,
}

impl Input {
    pub fn new() -> Self {
        unsafe {
            thd_sleep(100);
        }
        let dev = unsafe { maple_enum_dev(0, 0) };
        Input {
            dev,
            prev_buttons: 0,
            curr_buttons: 0,
        }
    }

    pub fn poll(&mut self) {
        self.prev_buttons = self.curr_buttons;
        self.curr_buttons = if self.dev.is_null() {
            0
        } else {
            let state = unsafe { maple_dev_status(self.dev) };
            if state.is_null() {
                0
            } else {
                unsafe { (*(state as *const ContState)).buttons }
            }
        };
    }

    pub fn just_pressed(&self, btn: Button) -> bool {
        let mask = button_mask(btn);
        self.prev_buttons & mask == 0 && self.curr_buttons & mask != 0
    }

    pub fn is_held(&self, btn: Button) -> bool {
        let mask = button_mask(btn);
        self.curr_buttons & mask != 0
    }

    pub fn any_just_pressed(&self) -> Option<Button> {
        const ALL: &[Button] = &[
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
            Button::A,
            Button::B,
            Button::X,
            Button::Y,
            Button::Start,
        ];
        for btn in ALL {
            if self.just_pressed(*btn) {
                return Some(*btn);
            }
        }
        None
    }
}

fn button_mask(btn: Button) -> u32 {
    match btn {
        Button::Up => CONT_DPAD_UP,
        Button::Down => CONT_DPAD_DOWN,
        Button::Left => CONT_DPAD_LEFT,
        Button::Right => CONT_DPAD_RIGHT,
        Button::A => CONT_A,
        Button::B => CONT_B,
        Button::X => CONT_X,
        Button::Y => CONT_Y,
        Button::Start => CONT_START,
    }
}
