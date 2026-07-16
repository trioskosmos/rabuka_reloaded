#![recursion_limit = "512"]
#![cfg_attr(feature = "psp", no_std)]
#![cfg_attr(feature = "psp", feature(alloc_prelude))]

#[cfg(feature = "psp")]
extern crate alloc;

// Platform compat: maps std types to no_std equivalents for PSP
pub(crate) mod compat;
pub(crate) use compat::{Arc, HashMap, HashSet, OnceLock, VecDeque};

#[cfg(feature = "alloc_tracker")]
pub mod alloc_counter;
#[cfg(feature = "alloc_tracker")]
#[global_allocator]
static ALLOC: alloc_counter::CountingAllocator = alloc_counter::CountingAllocator;

// Core data types — re-exported at crate root so all existing imports still work
pub mod core;
pub use core::card;
#[cfg(not(feature = "psp"))]
pub use core::card_loader;
pub use core::constants;
pub use core::game_state;
pub use core::player;
pub use core::types;
pub use core::zones;

// Bot AI module (excluded on PSP)
#[cfg(not(feature = "psp"))]
pub mod bot;

// Game logic modules
pub mod game;
pub use game::deck_builder;
#[cfg(not(feature = "psp"))]
pub use game::deck_parser;
pub use game::display;
pub use game::game_setup;
#[cfg(feature = "server")]
pub use game::web_server;

// Effect/condition system
pub mod ability;
pub mod ability_queue;
#[cfg(not(feature = "psp"))]
pub mod qa_test_suite;
#[cfg(not(feature = "psp"))]
pub mod timer;
pub mod rng;
pub mod triggers;
pub mod turn;
