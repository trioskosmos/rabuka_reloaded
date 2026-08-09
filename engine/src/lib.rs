#![recursion_limit = "512"]
#![cfg_attr(feature = "no_std", no_std)]

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
#[cfg(feature = "alloc_tracker")]
#[global_allocator]
static ALLOC: alloc_counter::CountingAllocator = alloc_counter::CountingAllocator;

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
pub use game::deck_parser;
pub use game::display;
pub use game::game_setup;
#[cfg(feature = "server")]
pub use game::web_server;

// Effect/condition system
pub mod ability;
pub mod ability_queue;
#[cfg(not(feature = "no_std"))]
pub mod qa_test_suite;
pub mod rng;
#[cfg(not(feature = "no_std"))]
pub mod timer;
pub mod triggers;
pub mod turn;
