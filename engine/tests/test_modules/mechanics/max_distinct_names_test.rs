//! Cross-validation for `util::max_distinct_names`.
//!
//! Lives in the integration suite because the lib is built with
//! `[lib] test = false` — unit tests inside src/ never execute.

use rabuka_engine::ability::util::{max_distinct_names, DistinctNamesResult};
use std::collections::HashSet;

/// Reference brute force — the exact semantics of the old exhaustive DFS.
fn brute_force(name_sets: &[Vec<String>]) -> DistinctNamesResult {
    let mut best = 0usize;
    let mut found_no_collision = false;
    let mut stack: Vec<(usize, HashSet<String>, bool)> = vec![(0, HashSet::default(), false)];
    while let Some((idx, seen, collided)) = stack.pop() {
        if idx == name_sets.len() {
            best = best.max(seen.len());
            if !collided {
                found_no_collision = true;
            }
            continue;
        }
        for name in &name_sets[idx] {
            let new_collided = collided || seen.contains(name.as_str());
            let mut next = seen.clone();
            next.insert(name.clone());
            stack.push((idx + 1, next, new_collided));
        }
    }
    DistinctNamesResult {
        distinct: best,
        collision: !found_no_collision,
    }
}

#[test]
fn dp_matches_brute_force() {
    // Deterministic pseudo-random cross-validation over small inputs,
    // including duplicates within a card's own name list and shared names
    // across cards (the collision-heavy cases).
    let names = ["a", "b", "c", "d"];
    let mut seed = 0x2545F4914F6CDD1Du64;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for case in 0..2000u32 {
        let n_cards = (rng() % 6) as usize + 1; // 1..=6 cards
        let name_sets: Vec<Vec<String>> = (0..n_cards)
            .map(|_| {
                let k = (rng() % 3) as usize + 1; // 1..=3 names per card
                (0..k)
                    .map(|_| names[(rng() % 4) as usize].to_string())
                    .collect()
            })
            .collect();
        let expect = brute_force(&name_sets);
        let got = max_distinct_names(&name_sets);
        assert_eq!(
            got.distinct, expect.distinct,
            "distinct mismatch in case {case}: {name_sets:?}"
        );
        assert_eq!(
            got.collision, expect.collision,
            "collision mismatch in case {case}: {name_sets:?}"
        );
    }
}

#[test]
fn degenerate_inputs() {
    // Empty input: no cards, trivially fine.
    let r = max_distinct_names(&[]);
    assert_eq!((r.distinct, r.collision), (0, false));
    // A card with zero names: no complete assignment exists — matches the
    // degenerate behavior of the old DFS (zero leaves reached).
    let r = max_distinct_names(&[vec!["a".into()], vec![]]);
    assert_eq!((r.distinct, r.collision), (0, true));
}

#[test]
fn cyclic_overlap_is_solved_exactly() {
    // card1={x,y}, card2={y,z}, card3={z,x}: picking x,y,z is collision-free.
    let sets = vec![
        vec!["x".to_string(), "y".to_string()],
        vec!["y".to_string(), "z".to_string()],
        vec!["z".to_string(), "x".to_string()],
    ];
    let r = max_distinct_names(&sets);
    assert_eq!(r.distinct, 3);
    assert!(!r.collision);

    // Pigeonhole: three cards sharing one name ⇒ every assignment collides.
    let sets = vec![vec!["a".into()], vec!["a".into()], vec!["a".into()]];
    let r = max_distinct_names(&sets);
    assert_eq!(r.distinct, 1);
    assert!(r.collision);

    // Greedy-undercount instance: first-fit would take x,y,z here too, but
    // [{b,c},{a,b},{c,a}] ordered so first-fit picks b then a... verify DP
    // finds 3 distinct regardless of adversarial ordering.
    let sets = vec![
        vec!["c".to_string(), "a".to_string()], // first-fit takes c
        vec!["c".to_string(), "b".to_string()], // first-fit stuck → collision on c
        vec!["a".to_string(), "b".to_string()],
    ];
    let r = max_distinct_names(&sets);
    assert_eq!(r.distinct, 3);
    assert!(!r.collision);
}
