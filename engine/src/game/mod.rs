//! Game shell: setup, persistence, menus and display around the engine.
//!
//! Everything here sits OUTSIDE the rules. It prepares a game, presents it, or
//! persists it; it does not decide anything about how a card resolves.
//!
//! - `deck_builder` / `deck_ordering` / `deck_parser`: deck formats.
//! - `game_setup`: deck loading, game creation, mulligan, and
//!   `execute_action` — the UI/test entry point into main-phase actions.
//! - `match_runner`: full-game loops (bot arena, replays).
//! - `display` / `language` / `menu` / `platform_ui`: rendering and prompts.
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
pub mod match_runner;
pub mod menu;
pub mod platform_ui;
pub mod sav;
#[cfg(feature = "server")]
pub mod web_server;
