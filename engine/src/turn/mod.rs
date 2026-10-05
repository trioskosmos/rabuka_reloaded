//! Turn engine: phase progression, live performance/scoring, and the
//! main-phase action entry points the UI and tests drive.
//!
//! - `phases`: phase machine + yell/live windows (`TurnEngine` impl).
//! - `live`: yell reveal, heart allocation, success determination, scoring.
//! - `actions`: main-phase actions (play member, use ability, set live…).
//! - `trigger_scan`: WHEN a trigger fires — scanning the board and queuing
//!   abilities. Not what a trigger *is*: the trigger vocabulary (the `起動` /
//!   `常時` strings and `TriggerKind`) is `crate::triggers`. The two were both
//!   called `triggers`, which made `use crate::triggers::*` ambiguous at every
//!   site that needed both.
//!
//! Ability execution itself lives in `crate::ability` (`AbilityResolver`).
pub mod actions;
pub mod live;
pub mod phases;
pub mod trigger_scan;

/// Unit struct carrying the turn-engine `impl` blocks (see submodules).
/// Stateless by design: all game state lives in `GameState`.
pub struct TurnEngine;
