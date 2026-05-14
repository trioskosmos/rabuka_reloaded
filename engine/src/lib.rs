#![recursion_limit = "512"]

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
pub use game::web_server;

// Effect/condition system

pub mod turn;
pub mod ability;
pub mod ability_queue;
pub mod qa_test_suite;
pub mod triggers;

