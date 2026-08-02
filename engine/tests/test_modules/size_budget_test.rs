use core::mem::size_of;
use rabuka_engine::ability::resolver::AbilityResolver;
use rabuka_engine::ability_queue::AbilityQueueEntry;
use rabuka_engine::card::{Ability, AbilityEffect, Card};
use rabuka_engine::core::game_modifiers::GameModifiers;
use rabuka_engine::core::types::*;

/// Permanent size-budget regression test.
///
/// Guards the per-instance memory footprint of the hot runtime structs on the
/// dev (default-features, 64-bit) build. These budgets are NOT exact numbers
/// to chase — they are ceilings meant to catch accidental regressions (e.g. a
/// field widened back to usize, or a struct re-bloated by an inlined member).
///
/// Sizes are pointer-width dependent; adjust the ceilings if the platform
/// baseline legitimately changes (e.g. 32-bit targets shrink usize).
#[test]
fn hot_struct_size_budget() {
    println!("=== HOT STRUCT SIZES (default features, 64-bit) ===");
    let _ = size_of::<GameModifiers>();

    let mut rows: Vec<(&str, usize, usize)> = vec![
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

    // The boxing of AbilityResolver in the queue entry is the headline win:
    // the entry must be well under what an inlined 1904B resolver would imply.
    for &(name, actual, budget) in &rows {
        println!("  {:<24} {:>5} B (budget {})", name, actual, budget);
        assert!(
            actual <= budget,
            "{} is {} B, over the {} B budget — check for a size regression",
            name,
            actual,
            budget
        );
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
