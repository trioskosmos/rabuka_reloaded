#![recursion_limit = "512"]

// Core data types — re-exported at crate root so all existing imports still work
pub mod core;
pub use core::card;
pub use core::card_loader;
pub use core::config;
pub use core::constants;
pub use core::game_state;
pub use core::mod_map;
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
pub mod bot;
pub mod ability;
pub mod ability_resolver;
pub mod ability_queue;
pub mod triggers;
