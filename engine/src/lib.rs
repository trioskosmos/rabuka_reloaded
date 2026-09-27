#![recursion_limit = "512"]
#![cfg_attr(feature = "no_std", no_std)]
// Cast hygiene (audit QUEUE B): surface every remaining lossy cast so new
// ones can't sneak in. Warn-only; the existing population is being worked
// down via saturate_* helpers / CardId newtype — see docs/CODE_AUDIT_2026-08-23.md.
#![warn(clippy::cast_possible_truncation)]
#![warn(clippy::cast_sign_loss)]
#![warn(clippy::cast_possible_wrap)]

#[cfg(feature = "no_std")]
#[macro_use]
extern crate alloc;

// PSP stubs for std macros
#[cfg(feature = "no_std")]
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {};
}
#[cfg(feature = "no_std")]
#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => {};
}

// Platform compat: maps std types to no_std equivalents for PSP
pub(crate) mod compat;
pub(crate) use compat::{Arc, Box, HashMap, HashSet, VecDeque};
#[cfg(feature = "serde_support")]
pub(crate) use compat::BTreeMap;

#[cfg(feature = "alloc_tracker")]
pub mod alloc_counter;

#[cfg(feature = "fast_alloc")]
pub mod pool_alloc;

/// One global allocator, three build shapes.
///
/// The `alloc_tracker` and `fast_alloc` features each want to own this slot, so
/// they are resolved here rather than in two places: a crate may only declare a
/// single `#[global_allocator]`, and letting both features attach one would fail
/// to compile. When both are on, the counter wraps the pool, which is the
/// combination that matters — it means the allocation report measures the
/// allocator the engine actually ships with, not a slower stand-in that exists
/// only while profiling.
#[cfg(all(feature = "fast_alloc", feature = "alloc_tracker"))]
#[global_allocator]
static ALLOC: alloc_counter::CountingAllocator = alloc_counter::CountingAllocator;

#[cfg(all(feature = "fast_alloc", feature = "alloc_tracker"))]
type SystemAlloc = pool_alloc::PoolAllocator;

#[cfg(all(feature = "fast_alloc", not(feature = "alloc_tracker")))]
#[global_allocator]
static ALLOC: pool_alloc::PoolAllocator = pool_alloc::PoolAllocator;

#[cfg(all(not(feature = "fast_alloc"), feature = "alloc_tracker"))]
#[global_allocator]
static ALLOC: alloc_counter::CountingAllocator = alloc_counter::CountingAllocator;

#[cfg(not(any(feature = "fast_alloc", feature = "alloc_tracker")))]
#[global_allocator]
static ALLOC: std::alloc::System = std::alloc::System;

// Core data types — re-exported at crate root so all existing imports still work
pub mod core;
pub use core::card;
pub use core::card_loader;
pub use core::constants;
pub use core::game_state;
pub use core::player;
pub use core::types;
pub use core::zones;

// Per-deck compact card data baked from web_ui/decks/*.txt (see
// tools/bake_deck_cards.py). load_two_decks() decodes only the two selected
// decks' cards from these blobs.
pub mod decks_cards_gen;

// Bot AI module (excluded on PSP)
#[cfg(not(feature = "no_std"))]
pub mod bot;

// Shared orchestration helpers for the std-only engine binaries
#[cfg(not(feature = "no_std"))]
pub mod bin_common;

// Game logic modules
pub mod game;
pub use game::deck_builder;
pub use game::deck_ordering;
pub use game::deck_parser;
pub use game::display;
pub use game::game_setup;
#[cfg(feature = "server")]
pub use game::web_server;

// Choice renderer for card selection menus
pub mod choice_renderer;

// Effect/condition system
pub mod ability;
pub mod ability_queue;
pub mod rng;
#[cfg(not(feature = "no_std"))]
pub mod timer;
#[cfg(all(not(feature = "no_std"), feature = "serde_support"))]
pub mod replay;
pub mod triggers;
pub mod turn;
