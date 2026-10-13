#![recursion_limit = "512"]
#![cfg_attr(feature = "no_std", no_std)]
// Cast hygiene (audit QUEUE B). The population is now zero: every narrowing
// goes through the `CountCast` trait in core::constants, which saturates at
// the top and floors at zero for signed -> unsigned, and the only `as` casts
// left in the crate are the ones inside that trait's own f64 impl, where a
// clamp on the preceding line proves the range. These three are DENIED rather
// than warned, because there is no backlog left to document and a new `as`
// should fail the build rather than join a list nobody reads.
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::cast_sign_loss)]
#![deny(clippy::cast_possible_wrap)]

// `too_many_arguments` is allowed per function, not crate-wide, and the
// functions carrying it all look the same way: the parameter list IS the
// domain data, not an accident. `execute_gain_ability` takes the effect plus
// the player, the zone, the card, the source, the ability index and the
// activation context because a card effect needs all of them at once, and
// bundling them into a struct would move the call site rather than clarify
// it. The generated decoders went the other way, via their `*Locals`
// accumulators, and that is the shape to use when a parameter list is
// genuinely incidental rather than definitional. See
// docs/CODE_AUDIT_2026-08-23.md.

// `#[macro_use]` is load-bearing: it is what puts `format!`/`vec!` in scope
// crate-wide under `no_std`, where there is no `std` prelude to supply them.
// Do NOT ungate this declaration to make an ungated `use alloc::…` resolve —
// `alloc` is in the extern prelude in both configurations, so the `use` works
// either way, and ungating the declaration puts `alloc`'s no-op `println!`
// into the macro_use prelude where it would shadow the real one on std builds.
// `alloc` is in the sysroot on every target, so the declaration is
// unconditional — modules below name `alloc::borrow::Cow` and friends
// directly, and gating the declaration off under std left those paths
// unresolvable in the host/test build. `#[macro_use]` stays no_std-only:
// std's prelude already exports vec!/format!, and importing alloc's copies
// as well would make every one of those macro calls ambiguous.
#[cfg_attr(feature = "no_std", macro_use)]
extern crate alloc;

// PSP stubs for std macros
#[cfg(feature = "no_std")]
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {};
}
#[cfg(feature = "no_std")]
#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => {};
}

// Platform compat: maps std types to no_std equivalents for PSP
// Public because the relocated unit tests (tests/run_all.rs) name these
// types directly; they were `pub(crate)` only because those tests used to
// live inside the crate.
pub mod compat;
pub use compat::{Arc, Box, HashMap, HashSet, OnceLock, VecDeque};
#[cfg(feature = "serde_support")]
pub use compat::BTreeMap;

#[cfg(feature = "alloc_tracker")]
pub mod alloc_counter;

#[cfg(feature = "fast_alloc")]
pub mod pool_alloc;

/// One global allocator, three build shapes.
///
/// The `alloc_tracker` and `fast_alloc` features each want to own this slot, so
/// they are resolved here rather than in two places: a crate may only declare a
/// single `#[global_allocator]`, and letting both features attach one would fail
/// to compile. When both are on, the counter wraps the pool, which is the
/// combination that matters — it means the allocation report measures the
/// allocator the engine actually ships with, not a slower stand-in that exists
/// only while profiling.
#[cfg(all(feature = "fast_alloc", feature = "alloc_tracker"))]
#[global_allocator]
static ALLOC: alloc_counter::CountingAllocator = alloc_counter::CountingAllocator;

#[cfg(all(feature = "fast_alloc", not(feature = "alloc_tracker")))]
#[global_allocator]
static ALLOC: pool_alloc::PoolAllocator = pool_alloc::PoolAllocator;

#[cfg(all(not(feature = "fast_alloc"), feature = "alloc_tracker"))]
#[global_allocator]
static ALLOC: alloc_counter::CountingAllocator = alloc_counter::CountingAllocator;

// std builds have no platform to satisfy, so the system allocator is the
// default. `no_std` targets have no `std` at all: the port owns the slot and
// declares its own `#[global_allocator]` (PicolibcAlloc on DS, the vendored
// sce heap on PSP, a bump heap on the rest) — the same split the other
// allocator arms above already use. GBA is the exception: agb 0.25 declares
// its own `BlockAllocator` inside `agb::agb_alloc`, so the port adds nothing.
#[cfg(all(
    not(feature = "no_std"),
    not(any(feature = "fast_alloc", feature = "alloc_tracker"))
))]
#[global_allocator]
static ALLOC: std::alloc::System = std::alloc::System;

// ── Module map ──
//
// The engine has four layers. Reading order follows the data, not the alphabet,
// so a change to how a card resolves can be traced without jumping around.
//
//   core   — game data and pure rules: card definitions, zones, player state,
//            the modifier tables, and the stat pipeline.
//   ability— WHAT an effect means: conditions, effects, and the resolver that
//            runs them.
//   turn   — WHEN things happen: the phase machine, live scoring, and trigger
//            scanning. This layer calls into `ability`; never the reverse.
//   game   — what the outside world sees: setup, deck handling, and the JSON
//            display shapes the UI and server serialise.
//
// Three modules sit outside that flow. `triggers` (the trigger vocabulary) and
// `ability::queue` (the pending-ability mailbox) are both shared between
// `ability` and `turn`, and are grouped with the layers they bridge. The
// std-only helpers are last.

// ── core: game data and pure rules ──
//
// Re-exported at the crate root so the many `use rabuka_engine::card::…`
// imports across the tree keep working.
pub mod core;
pub use core::card;
pub use core::card_loader;
pub use core::constants;
pub use core::game_state;
pub use core::player;
pub use core::types;
pub use core::zones;

// ── ability / turn: what effects mean, and when they run ──
pub mod ability;
// The trigger vocabulary — the `起動` / `常時` strings and `TriggerKind`. What a
// trigger IS. The code that decides WHEN one fires is `turn::trigger_scan`.
// The pending-ability queue lives in `ability::queue`; it is the mailbox
// between the two layers above, not a third layer of its own.
pub mod triggers;
pub mod turn;

// ── game: the outside world's view ──
pub mod game;
pub use game::deck_builder;
pub use game::deck_ordering;
pub use game::deck_parser;
pub use game::display;
pub use game::game_setup;
#[cfg(feature = "server")]
pub use game::web_server;

// ── Baked card data ──
//
// Per-deck compact card data baked from web_ui/decks/*.txt (see
// tools/bake_deck_cards.py). load_two_decks() decodes only the two selected
// decks' cards from these blobs.
pub mod decks_cards_gen;

// ── Presentation helpers ──
//
// Choice renderer for card selection menus.
pub mod choice_renderer;

// ── std-only support (excluded on no_std targets) ──
#[cfg(not(feature = "no_std"))]
pub mod bot;
#[cfg(not(feature = "no_std"))]
pub mod bin_common;
#[cfg(not(feature = "no_std"))]
pub mod timer;
#[cfg(all(not(feature = "no_std"), feature = "serde_support"))]
pub mod replay;

pub mod rng;
