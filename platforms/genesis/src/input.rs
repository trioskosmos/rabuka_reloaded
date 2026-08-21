//! Genesis joypad 1 poll via I/O ports $A10003/$A10009.
//! TH=1 read: d0=L d1=R d2=D d3=U d4=B d5=C (active low)
//! TH=0 read: d0=U d1=D d2=A d3=START      (active low)

use core::cell::Cell;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct PadState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub c: bool,
    pub start: bool,
}

const PAD_DATA: *mut u8 = 0xA10003 as *mut u8;
const PAD_DIR: *mut u8 = 0xA10009 as *mut u8;

pub struct Input {
    prev: Cell<PadState>,
    cur: Cell<PadState>,
}

impl Input {
    pub fn new() -> Self {
        unsafe {
            // bit6 (TH) as output; rest input
            core::ptr::write_volatile(PAD_DIR, 0x40);
        }
        Input {
            prev: Cell::new(PadState::default()),
            cur: Cell::new(PadState::default()),
        }
    }

    /// Latch a fresh pad read and compute edges against the previous read.
    pub fn poll(&self) {
        let p = unsafe {
            core::ptr::write_volatile(PAD_DATA, 0x40); // TH=1
            let th1 = core::ptr::read_volatile(PAD_DATA);
            core::ptr::write_volatile(PAD_DATA, 0x00); // TH=0
            let th0 = core::ptr::read_volatile(PAD_DATA);
            PadState {
                up: th0 & 0x01 == 0,
                down: th0 & 0x02 == 0,
                a: th0 & 0x04 == 0,
                start: th0 & 0x08 == 0,
                left: th1 & 0x01 == 0,
                right: th1 & 0x02 == 0,
                b: th1 & 0x10 == 0,
                c: th1 & 0x20 == 0,
            }
        };
        self.prev.set(self.cur.get());
        self.cur.set(p);
    }

    pub fn just_pressed(&self, f: impl Fn(&PadState) -> bool) -> bool {
        f(&self.cur.get()) && !f(&self.prev.get())
    }
}
