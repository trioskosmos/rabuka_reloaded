#![no_std]
#![no_main]

extern crate alloc;

use core::option::Option;

#[panic_handler]
fn panic(_: &core::panic::PanickInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn main() {}

// Test: can we use core::f32 module?
pub fn test() {
    let _x = f32::consts::PI;
}
