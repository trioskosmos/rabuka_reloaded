//! A cost modifier's expiry must undo exactly what was applied — including the sign.
//!
//! `execute_modify_cost` registers a revert with the value it applied, and expiry calls
//! `remove_cost_modifier(card_id, amount)`, which does `additive = additive - amount`.
//! Undoing an applied `d` therefore means passing `d` back unchanged.
//!
//! The registration used to store `d.abs()`. Because `remove_cost_modifier` clamps the
//! result with `.max(0)`, that reads as correct whenever nothing else has touched the
//! same card: a reduction of 2 applies `additive = -2`, and expiry computes
//! `-2 - 2 = -4`, which clamps to 0 — the right answer by accident.
//!
//! It is wrong as soon as a competing modifier exists, which is the interesting case:
//!
//! ```text
//! apply  -2  (a cost reduction)     additive = -2
//! apply  +3  (something else)       additive =  1
//! expire  -2  → correct: 1 - (-2) = 3
//!             → with |d|: 1 - 2 = -1, clamped to 0   ← the increase vanishes
//! ```
//!
//! The competing half is not hypothetical. PL!HS-bp5-005-AR registers a *negative*
//! delta whenever the chosen member's printed cost is below the deployed card's
//! (「このメンバーのコストは、選んだメンバーが元々持つコストより1低い値になる」), so the
//! reduction path is reachable in the corpus — it simply has not met a collision yet.
//!
//! These tests pin the arithmetic, because the collision that exposes it cannot be
//! built from the current card pool: no ability reduces and raises the same card's cost
//! at once.

use crate::helpers::*;

/// Undoing a signed delta is what restores a competing modifier intact.
#[test]
fn expiring_a_reduction_leaves_a_competing_increase_intact() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-005-AR");

    // A cost reduction: 「コストは…より1低い値になる」 applies a negative delta.
    game.state.mods.add_cost_modifier(card, -2);
    // An unrelated increase on the same card, from a different source.
    game.state.mods.add_cost_modifier(card, 3);
    assert_eq!(
        game.state.mods.get_cost_modifier(card),
        1,
        "a -2 reduction and a +3 increase net to +1"
    );

    // The reduction expires. Passing the signed value back is what undoes it.
    game.state.mods.remove_cost_modifier(card, -2);
    assert_eq!(
        game.state.mods.get_cost_modifier(card),
        3,
        "the reduction is gone and the +3 increase is untouched"
    );

    // What the old registration would have done. Reverting by MAGNITUDE moves the
    // surviving modifier the wrong way and the +3 increase does not come back — the
    // exact value it lands on depends on the residue, which is the point: there is no
    // magnitude that undoes a reduction correctly, because the amount to undo is
    // signed.
    game.state.mods.add_cost_modifier(card, -2);
    game.state.mods.remove_cost_modifier(card, 2);
    assert_ne!(
        game.state.mods.get_cost_modifier(card),
        3,
        "reverting by MAGNITUDE instead of by signed value does not restore the +3 \
         increase — this is the arithmetic the fix removed"
    );
}

/// The mirror case: expiring an increase must not manufacture a reduction.
#[test]
fn expiring_an_increase_leaves_a_competing_reduction_intact() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-005-AR");

    game.state.mods.add_cost_modifier(card, 6);
    game.state.mods.add_cost_modifier(card, -2);
    assert_eq!(game.state.mods.get_cost_modifier(card), 4);

    game.state.mods.remove_cost_modifier(card, 6);
    assert_eq!(
        game.state.mods.get_cost_modifier(card),
        -2,
        "the +6 increase is gone; the -2 reduction is exactly as it was"
    );
}

/// A lone modifier still reverts cleanly in both directions.
///
/// The `.max(0)` clamp is what let the sign bug survive: these are the cases where
/// passing the magnitude happens to give the right answer, so they must keep passing
/// when the signed value is passed instead.
#[test]
fn a_lone_modifier_reverts_to_nothing_in_both_directions() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    for (applied, label) in [(-2i16, "reduction"), (6i16, "increase")] {
        let card = game.id("PL!HS-bp5-005-AR");
        game.state.mods.add_cost_modifier(card, applied);
        assert_eq!(
            game.state.mods.get_cost_modifier(card),
            applied,
            "{label} applies as {applied}"
        );
        game.state.mods.remove_cost_modifier(card, applied);
        assert_eq!(
            game.state.mods.get_cost_modifier(card),
            0,
            "{label} reverts to no modifier at all, and the entry is dropped"
        );
    }
}
