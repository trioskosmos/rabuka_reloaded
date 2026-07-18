use core::ffi::c_uint;

const MAPLE_FUNC_CONTROLLER: c_uint = 0x801;

#[repr(C)]
struct ContState {
    buttons: c_uint,
    rtrn: c_uint,
    ltrig: u16,
    rtrig: u16,
    joyx: u16,
    joyy: u16,
    joy2x: u16,
    joy2y: u16,
}

#[repr(C)]
enum MapleDeviceClass {
    None,
    Maple,
    Drum,
    Keyboard,
    Mouse,
    Lightgun,
    Media,
}

#[repr(C)]
struct MapleDevice {
    dev_class: MapleDeviceClass,
    port: u32,
    unit: u32,
    info: *mut core::ffi::c_void,
    frame: u32,
    func: c_uint,
    _private: [u32; 3],
}

extern "C" {
    fn maple_dev_attach(port: c_uint, func: c_uint) -> *mut MapleDevice;
    fn maple_dev_status(dev: *mut MapleDevice) -> *mut core::ffi::c_void;
}

const CONT_DPAD_UP: c_uint = 0x0001;
const CONT_DPAD_DOWN: c_uint = 0x0002;
const CONT_DPAD_LEFT: c_uint = 0x0004;
const CONT_DPAD_RIGHT: c_uint = 0x0008;
const CONT_A: c_uint = 0x0100;
const CONT_B: c_uint = 0x0200;
const CONT_X: c_uint = 0x0400;
const CONT_Y: c_uint = 0x0800;
const CONT_START: c_uint = 0x1000;

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
    prev_buttons: c_uint,
    curr_buttons: c_uint,
}

impl Input {
    pub fn new() -> Self {
        let dev = unsafe { maple_dev_attach(0, MAPLE_FUNC_CONTROLLER) };
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

    #[cfg(not(feature = "dc"))]
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

fn button_mask(btn: Button) -> c_uint {
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
