#![no_std]

// ── PS1 heap: psx-sdk-rs provides a global allocator (linked_list_allocator
//    over the data cache scratchpad) via sys_heap!. Use KB — the MB arm of the
//    macro is broken. Game working set is tiny; code ~1MB + heap fits 2MB RAM.
psx::sys_heap!(256 KB);

pub mod decks_baked;
pub mod display;
pub mod input;
