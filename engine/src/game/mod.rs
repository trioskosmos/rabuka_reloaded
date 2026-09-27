//! Game shell: setup, persistence, menus and display around the engine.
//!
//! - `game_setup`: deck loading, game creation, mulligan, `execute_action`
//!   (the UI/test entry point into main-phase actions).
//! - `match_runner`: full-game loops (bot arena, replays).
//! - `deck_builder` / `deck_ordering` / `deck_parser`: deck formats.
//! - `display` / `menu` / `platform_ui` / `language`: rendering + prompts.
//! - `sav`: save files. `link`: link battles. `web_server` (feature-gated).
//!
//! Rules live in `crate::turn`; card data in `crate::core`.
pub mod deck_builder;
pub mod deck_ordering;
pub mod deck_parser;
pub mod display;
pub mod game_setup;
pub mod language;
pub mod link;
pub mod platform_ui;
pub mod menu;
pub mod match_runner;
pub mod sav;
#[cfg(feature = "server")]
pub mod web_server;
