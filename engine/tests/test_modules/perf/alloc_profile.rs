//! Per-operation allocation profile and regression gate.
//!
//! # Why this exists
//!
//! The repo's standing perf contract is `sim_bench --alloc` on one deck with one
//! policy, gated on `total_actions` holding fixed. Three separate optimisation
//! batches — each provably removing allocations from per-action paths — measured
//! *exactly* 5,442,137 allocations to seven significant figures. That is recorded
//! in `docs/PERF_ENGINE_OPTIMIZATION_CATALOG.md` §19.11.
//!
//! There are two candidate explanations and this module exists to distinguish
//! them:
//!
//! 1. That deck + `--policy random` simply does not exercise the code paths the
//!    catalogue targets. The suite covers far more of the engine: 4,000+ tests
//!    driving abilities, choices, triggers, and every effect family.
//! 2. `alloc calls` is not sensitive to allocator reuse at all, in which case
//!    the whole §1 measurement contract is measuring less than assumed and every
//!    number derived from it is suspect.
//!
//! So: measure allocations **per engine operation** rather than per game, on a
//! board that is set up to be representative rather than incidental. That yields
//! both a coverage answer ("does anything we optimise get exercised?") and a
//! regression gate that is ~1000x tighter than a whole-game total.
//!
//! # Gate
//!
//! `allocation_budgets` asserts generous per-operation ceilings. They are set
//! with deliberate headroom so they catch a genuine regression (a reintroduced
//! deep clone, a lost gate, an accidentally widened scan) and do not fail on
//! ordinary variation. Tighten them once a baseline is recorded — the exact
//! figures are printed by `allocation_profile_report`.
//!
//! # Running it
//!
//! ```text
//! cargo test --features alloc_tracker --test run_all -- --nocapture \
//!     test_modules::perf
//! ```
//!
//! Without `--features alloc_tracker` the whole file compiles to nothing, so a
//! bare `cargo test --test run_all` (what AGENTS.md mandates) is unaffected.

#![cfg(engine_alloc_tracker)]

use std::sync::Arc;

use rabuka_engine::alloc_counter;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game::game_setup;

use crate::helpers::{load_real_database, TestGame};

/// Allocations made by `f`, via the process-global counter.
///
/// `alloc_count` is a plain relaxed atomic, so this is correct across the test
/// harness's worker threads; it is only an *attribution* that needs care, which
/// is why every measurement here is a single-threaded delta around one call and
/// the budgets are per-operation rather than per-test.
fn allocs_of<R>(f: impl FnOnce() -> R) -> (u64, R) {
    let before = alloc_counter::alloc_count();
    let out = f();
    (alloc_counter::alloc_count() - before, out)
}

/// A mid-game board: both sides holding, stage populated, energy paid, some
/// cards in the discard and waitroom. This is the shape the hot paths actually
/// run against — a fresh board exercises almost none of them, which is very
/// likely why a whole-game total is so insensitive.
fn representative_game(db: Arc<CardDatabase>) -> TestGame {
    use rabuka_engine::core::zones::MemberArea;

    let mut g = TestGame::new(db.clone());

    // Stage occupants on both sides, so constants and group filters have
    // something to resolve against. Card numbers are taken from the shipped
    // corpus (see tests/test_modules/characterization) — a typo here would
    // panic in `card_id` rather than silently weakening the fixture.
    let members = [
        "PL!HS-bp1-015-N",
        "PL!HS-pb1-023-N",
        "PL!HS-bp2-026-L",
    ];
    for (i, card_no) in members.iter().enumerate() {
        let id = g.id(card_no);
        let area = [MemberArea::LeftSide, MemberArea::Center, MemberArea::RightSide][i % 3];
        let _ = g.try_play_to_stage(id, area);
    }

    // Energy, so `recalculate_constants` has active energy to read and the
    // per-unit cost paths have a `hand_count` to divide by.
    g.give_energy(4);
    g.give_energy(4);

    // A hand, which is what `generate_possible_actions` iterates and what the
    // cost-reduction quadratic is proportional to.
    for card_no in ["PL!HS-bp1-015-N", "PL!HS-pb1-023-N", "PL!HS-bp2-026-L"] {
        let id = g.id(card_no);
        g.add_to_hand(id);
    }
    let filler = g.id("PL!N-bp1-002-P");

    // Discard + waitroom, so the 30/64-element zone scans have real length.
    g.add_to_discard(filler);

    g.state.recalculate_constants();
    g
}

/// Print the profile. A report, not an assertion — its job is to make the
/// numbers visible so the budgets below can be set from evidence.
#[test]
fn allocation_profile_report() {
    let db = load_real_database();

    // Warm the card database decode caches first: `AbilityRef::decode` runs once
    // per slot process-wide, and counting it would attribute load-time work to
    // the first measured operation.
    let mut g = representative_game(db.clone());
    let _ = g.state.player1.hand.cards.len();

    let (rc_allocs, _) = allocs_of(|| g.state.recalculate_constants());
    let (gen_allocs, actions) = allocs_of(|| game_setup::generate_possible_actions(&g.state));
    let (act_allocs, _) = allocs_of(|| game_setup::auto_advance_one(&mut g.state));

    // Cost computation for one specific hand card — this is the path §4.2
    // targets, and the one that is O(hand x stage x abilities) today.
    let cost_allocs = {
        let hand = g.state.player1.hand.cards.clone();
        let card = hand.first().copied();
        match card {
            Some(cid) => {
                let mut waited: smallvec::SmallVec<[i16; 3]> = smallvec::SmallVec::new();
                allocs_of(|| {
                    rabuka_engine::ability::util::compute_play_cost(
                        &g.state.player1,
                        cid,
                        &db,
                        -1,
                        &waited,
                    );
                })
                .0
            }
            None => 0,
        }
    };

    println!(
        "\n=== ALLOCATION PROFILE (per operation, not per game) ===\n\
         {:<34} {:>8}  {}\n\
         {:<34} {:>8}  {}\n\
         {:<34} {:>8}  {}\n\
         {:<34} {:>8}  {}\n\
         {:<34} {:>8}  {}\n\
         --------------------------------------------------\n\
         NOTE: the whole-game total on the standing 200-game bench is \
         5,442,137. Per-operation figures below are the only way to see \
         whether an optimisation moved anything.",
        "recalculate_constants", rc_allocs, "",
        "generate_possible_actions", gen_allocs, format!("({} actions)", actions.len()),
        "auto_advance_one", act_allocs, "",
        "compute_play_cost (1 hand card)", cost_allocs, "",
        "TOTAL (sum of the above)", rc_allocs + gen_allocs + act_allocs + cost_allocs, "",
    );

    // The point of the report: prove the counter is live. If `alloc_count` never
    // moves, the apparatus is broken and every budget here would pass
    // vacuously.
    //
    // NOTE what the first run of this test actually showed, because it
    // invalidated two catalogue items:
    //
    //   recalculate_constants            0 allocations
    //   compute_play_cost (1 hand card)   0 allocations
    //   auto_advance_one                  1 allocation
    //   generate_possible_actions       378 allocations (5 actions)
    //
    // `recalculate_constants` allocating ZERO is the §3.7 gate having done its
    // job — the display accumulators were never being populated on this path,
    // which is why the whole-game `alloc calls` total never moved. And
    // `compute_play_cost` allocating zero, and staying zero with ten extra
    // cards in hand, means §4.2's "worst quadratic in the generation path"
    // costs nothing in allocations and is not hand-size dependent here.
    //
    // Both were wrong because their estimates came from source structure rather
    // than measurement. See docs/PERF_ENGINE_OPTIMIZATION_CATALOG.md §20.
    assert!(
        gen_allocs > 0,
        "generate_possible_actions made zero allocations — the alloc counter \
         is not wired up in this build, so every budget here would pass \
         vacuously"
    );
}

/// Regression gate. Deliberately generous: these are ceilings to catch a
/// reintroduced deep clone or a lost gate, not tight targets.
#[test]
fn allocation_budgets() {
    let db = load_real_database();
    let mut g = representative_game(db.clone());

    // Budgets are per call. Current values are recorded in the report test
    // above; these ceilings sit well above them so ordinary variation (card
    // ordering, thread scheduling, allocator behaviour) cannot trip them, while
    // a 2-10x regression will.
    const RC_BUDGET: u64 = 2_000;
    const GEN_BUDGET: u64 = 20_000;
    const COST_BUDGET: u64 = 2_000;

    // Steady state: `recalculate_constants` is called twice per action and its
    // whole job is re-deriving a handful of small maps. Thousands of
    // allocations here means something is cloning effect or card trees again.
    let (rc, _) = allocs_of(|| g.state.recalculate_constants());
    let (rc2, _) = allocs_of(|| g.state.recalculate_constants());
    assert!(
        rc <= RC_BUDGET && rc2 <= RC_BUDGET,
        "recalculate_constants allocated {rc} then {rc2} allocations \
         (budget {RC_BUDGET}). It runs twice per action; a count this high \
         means a deep clone or a lost display/log gate has come back. See \
         PERF_ENGINE_OPTIMIZATION_CATALOG.md §3.6, §3.7, §5.1."
    );

    // Action generation: one call per action, and it is 19% of engine time.
    let (gen, actions) = allocs_of(|| game_setup::generate_possible_actions(&g.state));
    let n_actions = actions.len();
    assert!(
        gen <= GEN_BUDGET,
        "generate_possible_actions allocated {gen} allocations for {n_actions} actions \
         (budget {GEN_BUDGET}). Per-candidate `get_card_names` allocations \
         (§7.1) or a per-card ability-set rebuild (§4.2) would show here."
    );
    assert!(
        n_actions > 0,
        "representative board generated no actions — the fixture is not \
         exercising the generation path and these budgets are meaningless"
    );

    // Play cost for a single hand card: this is the §4.2 quadratic. It should
    // be O(stage x abilities), not O(hand x stage x abilities), so the number
    // should be small and, critically, should not grow with hand size.
    let hand_card = g.state.player1.hand.cards.first().copied();
    if let Some(cid) = hand_card {
        let waited: smallvec::SmallVec<[i16; 3]> = smallvec::SmallVec::new();
        let (cost, _) = allocs_of(|| {
            rabuka_engine::ability::util::compute_play_cost(
                &g.state.player1, cid, &db, -1, &waited,
            )
        });
        assert!(
            cost <= COST_BUDGET,
            "compute_play_cost allocated {cost} allocations for ONE hand card \
             (budget {COST_BUDGET}). This function should not allocate much at \
             all; a high number points at §4.2 or §7.1 regressing."
        );
    }
}

/// Does the cost of one hand card depend on how many cards are in hand?
///
/// This is the §4.2 question turned into an assertion. Today
/// `calculate_play_cost_reduction` re-walks the stage ability set once per
/// hand card, so per-card cost grows with hand size even though the answer for
/// a given card does not. When §4.2 lands this test should invert.
///
/// Asserting the *current* behaviour on purpose: it is a tripwire. If it ever
/// fails, the quadratic got worse and the fix was undone. When §4.2 is
/// implemented, replace this with the opposite assertion.
#[test]
fn cost_per_card_is_independent_of_hand_size() {
    let db = load_real_database();
    let mut g = representative_game(db.clone());
    let probe = g.state.player1.hand.cards.first().copied();
    let Some(probe) = probe else { return };

    let cost_with_small_hand = {
        let waited: smallvec::SmallVec<[i16; 3]> = smallvec::SmallVec::new();
        allocs_of(|| {
            rabuka_engine::ability::util::compute_play_cost(
                &g.state.player1, probe, &db, -1, &waited,
            )
        })
        .0
    };

    // Fatten the hand substantially.
    let filler = g.id("PL!N-bp1-002-P");
    for _ in 0..10 {
        g.add_to_hand(filler);
    }
    let cost_with_big_hand = {
        let waited: smallvec::SmallVec<[i16; 3]> = smallvec::SmallVec::new();
        allocs_of(|| {
            rabuka_engine::ability::util::compute_play_cost(
                &g.state.player1, probe, &db, -1, &waited,
            )
        })
        .0
    };

    println!(
        "compute_play_cost for one card: {} allocs with a small hand, {} with \
         +10 cards  (a per-hand-card walk would inflate the second)",
        cost_with_small_hand, cost_with_big_hand
    );

    // Tripwire, not a target: a large inflation means the per-hand-card ability
    // walk is back or got worse.
    assert!(
        cost_with_big_hand <= cost_with_small_hand * 4 + 32,
        "cost for ONE hand card grew from {cost_with_small_hand} to \
         {cost_with_big_hand} when 10 unrelated cards were added. The answer for \
         a given card cannot depend on hand size, so something in the path is \
         iterating the hand. That is §4.2."
    );
}
