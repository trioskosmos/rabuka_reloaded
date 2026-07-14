#![recursion_limit = "512"]
#[cfg(feature = "alloc_tracker")]
pub mod alloc_counter;
#[cfg(feature = "alloc_tracker")]
#[global_allocator]
static ALLOC: alloc_counter::CountingAllocator = alloc_counter::CountingAllocator;

// Core data types  Ere-exported at crate root so all existing imports still work
pub mod core;
pub use core::card;
pub use core::card_loader;
pub use core::constants;
pub use core::game_state;
pub use core::player;
pub use core::types;
pub use core::zones;

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
pub mod qa_test_suite;
pub mod rng;
pub mod triggers;
pub mod timer;
pub mod turn;
