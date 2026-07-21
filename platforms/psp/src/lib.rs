#![no_std]

extern crate alloc;

#[cfg(feature = "psp")]
pub mod display;
#[cfg(feature = "psp")]
pub mod input;
