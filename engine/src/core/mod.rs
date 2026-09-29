// `core::pool` (a hand-rolled `Pool<T>` + `make_pool_box!` for `EkBox`) used
// to live here. It was dead once `EkBox` became `Arc<EffectKind>` in card.rs —
// see the comment there for why the pool was the slower of the two designs —
// and nothing referenced it. Removed rather than left declared.

pub mod card;
pub mod card_loader;
pub mod constants;
pub mod game_modifiers;
pub mod game_state;
pub mod player;
// Not gated on `no_std`: the module selects core::sync::atomic or
// std::sync::atomic itself, and the hot paths that read these gates
// (game_state log materialization, turn::phases, turn::live, the resolver)
// are compiled on every target. Gating the declaration off under `no_std`
// left those call sites pointing at a module that did not exist, which is
// what broke every no_std port (gba/ds/psp/wii/dc/jaguar/wasm/cdi/genesis).
pub mod runtime_flags;
pub mod stats_pipeline;
pub mod types;
pub mod zones;

/// Compact card binary format for RAM-constrained targets (GBA, DS).
#[cfg(feature = "compact_card_data")]
pub mod card_binary;
#[cfg(feature = "compact_card_data")]
pub mod cards_gen;
