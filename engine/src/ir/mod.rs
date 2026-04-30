//! Intermediate Representation for parsed abilities.
//!
//! This module defines typed Rust enums for effects, conditions, and costs,
//! converting from the flat JSON produced by parser.py.
//! The goal is zero runtime Japanese text matching in the engine.

pub mod effect;
pub mod condition;
pub mod cost;
pub mod zone;
pub mod filter;

pub use effect::{Effect, Target, Duration, Resource, Count, StateChange, zone_from_str};
pub use condition::Condition;
pub use zone::Zone;
