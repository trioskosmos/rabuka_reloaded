//! Ability system: parsing-adjacent types plus the execution engine.
//!
//! Map — how the pieces connect:
//! - `types` / `enums`: shared vocabulary (`Choice`, `ChoiceResult`,
//!   `ExecutionContext`, `ActionType`, `Zone`). Start here.
//! - `resolver`: `AbilityResolver::resolve_ability` — the per-ability
//!   pipeline (gates → cost → `execute_effect` → choice bookkeeping).
//! - `queue`: the pending-ability queue. The mailbox between this module and
//!   `crate::turn` — trigger scans post entries, the turn engine drains them.
//!   It was a crate-root `ability_queue.rs`, which read as a peer of `ability`
//!   when it is really a part of it.
//! - `effects`: `execute_effect` — guards + the `ActionType` dispatch match.
//!   Every file here is named for the `action` wire string in
//!   `cards/abilities.json` that it implements, and holds exactly that
//!   action's code: `deck.rs` is `draw_card`, `resource.rs` is
//!   `gain_resource`, `move/` is `move_cards`, and so on. `mod.rs` is what
//!   runs before dispatch; `dispatch.rs` is the table.
//! - `cost`: cost payment (`pay_cost` → per-action payers).
//! - `compound`: `Sequential` / `Conditional*` multi-step execution.
//! - `choice`: the answering half of a choice — what a player picks. The
//!   asking half is `effects/choice.rs`.
//! - `condition`: gating predicates (`ConditionContext::evaluate_condition`).
//! - `util`: shared filters, zone helpers, selection primitives.
//!   Domain submodules live in `util/`; game-wide label tables live in
//!   `describe`.
//! - `vm` / `abilities_gen` / `ability_store`: bytecode decoding + store.
//! - `describe`: EN/JA rendering of effects, costs and zones.
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
pub mod gates;
// `log` is NOT gated out on `no_std`: `ability/log.rs` carries its own
// `#[cfg(feature = "no_std")] mod inner` with no-op verdict recording, and its
// `impl AbilityResolver` block is called from the action dispatch table
// (`effects/dispatch.rs`, `effects/move/mod.rs`), which every target compiles.
// Gating the module left those two call sites with no method to resolve to.
pub mod log;
pub mod queue;
pub mod resolver;
pub mod types;
pub mod util;

pub mod abilities_gen;
#[cfg(feature = "bytecode_abilities")]
pub mod vm;
