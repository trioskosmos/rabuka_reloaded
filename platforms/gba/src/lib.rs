#![no_std]

extern crate alloc;

/// The engine resolves its `#[global_allocator]` slot per build shape and
/// deliberately leaves it to the port under `no_std` (see engine/src/lib.rs).
/// agb 0.25 already satisfies that itself: `agb::agb_alloc` declares
/// `#[global_allocator] static GLOBAL_ALLOC: BlockAllocator`, carving EWRAM
/// from `data_end`. `agb::InternalAllocator` is only a
/// `core::alloc::Allocator`, not a `GlobalAlloc`, so declaring it here was both
/// a trait error and a second global allocator in the link.

mod board;
mod card_art_gen;
pub mod deck_builder;
pub mod diag;
mod display;
mod font_tiles_gen;
mod overlay;
#[cfg(feature = "diag-shot")]
pub mod shot;
pub mod sram;
mod texticons_gen;

pub mod decks_baked;
pub mod gba_ui;
pub mod input;
pub mod lang;
pub mod link;
pub mod menu;
pub mod screens;
pub mod ui;
