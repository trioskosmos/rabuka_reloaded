#![no_std]

// ── PS1 heap: psx-sdk-rs provides a global allocator (linked_list_allocator
//    over a static .bss buffer) via sys_heap!. Use KB — the MB arm of the macro
//    is broken. The card blob is baked (~12KB), so the heap only holds the
//    match working set. Host-side bounded-heap test of the exact load+match
//    path peaks at ~187KB with 64-bit pointers (PS1's 32-bit pointers use less);
//    192KB covers it with margin and still leaves ~22KB between heap and stack.
psx::sys_heap!(192 KB);

pub mod decks_baked;
pub mod decks_card_blob;
pub mod display;
pub mod input;
