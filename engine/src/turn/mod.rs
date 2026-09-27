//! Turn engine: phase progression, live performance/scoring, and the
//! main-phase action entry points the UI and tests drive.
//!
//! - `phases`: phase machine + yell/live windows (`TurnEngine` impl).
//! - `live`: yell reveal, heart allocation, success determination, scoring.
//! - `actions`: main-phase actions (play member, use ability, set live…).
//! - `triggers`: trigger scanning → ability queue entries.
//! Ability execution itself lives in `crate::ability` (`AbilityResolver`).
pub mod actions;
pub mod live;
#[cfg(test)]
mod mulligan_tests;
pub mod phases;
pub mod triggers;

/// Unit struct carrying the turn-engine `impl` blocks (see submodules).
/// Stateless by design: all game state lives in `GameState`.
pub struct TurnEngine;
