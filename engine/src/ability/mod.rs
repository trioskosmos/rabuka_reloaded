//! Ability system: parsing-adjacent types plus the execution engine.
//!
//! Map — how the pieces connect:
//! - `types` / `enums`: shared vocabulary (`Choice`, `ChoiceResult`,
//!   `ExecutionContext`, `ActionType`, `Zone`). Start here.
//! - `resolver`: `AbilityResolver::resolve_ability` — the per-ability
//!   pipeline (gates → cost → `execute_effect` → choice bookkeeping).
//! - `effects`: `execute_effect` — guards + the `ActionType` dispatch match.
//! - `cost`: cost payment (`pay_cost` → per-action payers).
//! - `compound`: `Sequential` / `Conditional*` multi-step execution.
//! - `choice` + `look` + `move_cards`: player-choice and card-movement
//!   executors. Choices pause execution (`pending_choice`); answers resume
//!   via `resume_pending_actions` (choice.rs) / turn/actions.rs.
//! - `condition`: gating predicates (`ConditionContext::evaluate_condition`).
//! - `util`: shared filters, zone helpers, selection primitives.
//!   Domain submodules live in `util/` (`labels`, `hearts`, `duration`,
//!   `selection`); game-wide label tables live in `describe`.
//! - `vm` / `abilities_gen` / `ability_store`: bytecode decoding + store.
//! - `describe`: EN/JA rendering of effects, costs and zones.
//! - `condition/` + `effects/`: per-kind condition/executor submodules.
//! - `move_cards/`: placement / selection / transport submodules.
//! - `debug` / `log`: diagnostics and structured verdicts (no game logic).
pub mod ability_store;
pub mod choice;
pub mod compound;
pub mod condition;
pub mod cost;
pub mod debug;
pub mod describe;
pub mod dynamic_count;
pub mod effects;
pub mod enums;
#[cfg(not(feature = "no_std"))]
pub mod log;
pub mod look;
pub mod move_cards;
pub mod resolver;
pub mod types;
pub mod util;

pub mod abilities_gen;
#[cfg(feature = "bytecode_abilities")]
pub mod vm;
