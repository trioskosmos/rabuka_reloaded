#![recursion_limit = "512"]

pub mod card;
pub mod constants;
pub mod zones;
pub mod player;
pub mod game_state;
pub mod turn;
pub mod card_loader;
pub mod deck_builder;
pub mod deck_parser;
pub mod web_server;
pub mod bot;
pub mod game_setup;
pub mod ability;
pub mod ability_resolver;
pub mod check_timing;
pub mod cheer_system;
pub mod selection_system;
pub mod card_matching;

// #[cfg(test)]
// mod real_gameplay_test;
// #[cfg(test)]
// mod simple_gameplay_test;
// #[cfg(test)]
// mod working_gameplay_test;
// #[cfg(test)]
// mod actual_gameplay_test;
// #[cfg(test)]
// mod real_ability_test;
// #[cfg(test)]
// mod working_ability_test;
