//! Genesis 3-button pad poll via I/O ports 0xA10003 etc.
//! Stub that compiles and returns no-press; real poll is ~20 lines of
//! read TH/TR dance.

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

pub struct Input {
    prev: PadState,
}

impl Input {
    pub fn new() -> Self {
        Self { prev: PadState::default() }
    }

    /// Poll Joypad 1 at $A10003. Stub returns not pressed so game still advances via timeout.
    pub fn poll(&mut self) -> PadState {
        // Real impl:
        // unsafe { write_volatile(0xA10009, 0x40); let a = read(...); write 0x00; let b = read(...); decode }
        let cur = PadState::default();
        self.prev = cur;
        cur
    }
}
