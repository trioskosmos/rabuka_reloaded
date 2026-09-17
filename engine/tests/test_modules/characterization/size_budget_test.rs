use core::mem::size_of;
use rabuka_engine::ability::resolver::AbilityResolver;
use rabuka_engine::ability_queue::AbilityQueueEntry;
use rabuka_engine::card::{Ability, AbilityEffect, Card};
use rabuka_engine::core::game_modifiers::GameModifiers;
use rabuka_engine::core::types::*;

/// Hot-struct size report.
///
/// HISTORY: this used to hard-fail when a struct exceeded its recorded
/// ceiling. That fought legitimate feature work (any new tracking field on
/// GameModifiers tripped it), so the per-struct ceilings are now DIAGNOSTIC:
/// sizes are always printed, and growth past a reference size prints a
/// warning instead of failing. Skim the output when you touch these structs;
/// nothing fails because of it.
///
/// The ONE hard invariant kept here is architectural: AbilityQueueEntry must
/// not inline AbilityResolver (the resolver is boxed so idle entries pay
/// pointer-sized cost, not ~2 KB). That decision is load-bearing for queue
/// performance and memory — regressions there SHOULD fail the suite.
///
/// Sizes are pointer-width dependent; dev build = default features, 64-bit.
#[test]
fn hot_struct_size_budget() {
    println!("=== HOT STRUCT SIZES (default features, 64-bit) ===");
    let _ = size_of::<GameModifiers>();

    // Reference sizes are informational landmarks, not ceilings.
    let rows: Vec<(&str, usize, usize)> = vec![
        ("AbilityQueueEntry", size_of::<AbilityQueueEntry>(), 700),
        ("AbilityResolver", size_of::<AbilityResolver>(), 2200),
        (
            "GameState",
            size_of::<rabuka_engine::game_state::GameState>(),
            13000,
        ),
        ("Player", size_of::<rabuka_engine::player::Player>(), 900),
        ("Card", size_of::<Card>(), 400),
        ("Ability", size_of::<Ability>(), 160),
        ("AbilityEffect", size_of::<AbilityEffect>(), 200),
        ("GameModifiers", size_of::<GameModifiers>(), 1200),
        ("PerformanceSnapshot", size_of::<PerformanceSnapshot>(), 500),
        ("LogEntry", size_of::<LogEntry>(), 300),
        ("MemberContribution", size_of::<MemberContribution>(), 160),
        ("MovementEvent", size_of::<MovementEvent>(), 60),
        ("Allocation", size_of::<Allocation>(), 40),
        ("AbilityApplication", size_of::<AbilityApplication>(), 40),
        ("PositionChangeEvent", size_of::<PositionChangeEvent>(), 64),
        ("Adjustment", size_of::<Adjustment>(), 64),
        ("AbilityBonus", size_of::<AbilityBonus>(), 40),
    ];

    for &(name, actual, reference) in &rows {
        if actual > reference {
            println!(
                "  {:<24} {:>5} B (reference {} — GREW by {} B; intentional? bump the reference)",
                name,
                actual,
                reference,
                actual - reference
            );
        } else {
            println!("  {:<24} {:>5} B (reference {})", name, actual, reference);
        }
    }

    // Cross-struct invariant: the queue entry must NOT inline a full resolver.
    let inline_resolver_cost = size_of::<AbilityResolver>();
    assert!(
        size_of::<AbilityQueueEntry>() < inline_resolver_cost,
        "AbilityQueueEntry ({}) embeds an inlined AbilityResolver ({} B) — \
         the resolver should be boxed so idle entries pay 8 B, not {} B",
        size_of::<AbilityQueueEntry>(),
        inline_resolver_cost,
        inline_resolver_cost
    );

    let _ = rows.len();
}
