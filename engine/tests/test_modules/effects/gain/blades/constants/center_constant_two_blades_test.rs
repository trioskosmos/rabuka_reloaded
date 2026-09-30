//! `PL!SP-bp4-003-R` 嵐千砂都 — {{jyouji.png|常時}}{{center.png|センター}}ブレード×2
//!
//! The printed amount is **exactly two**, so the assertion says so.
//!
//! This file previously read `assert!(get_blade_modifier(m) >= 2, "center constant grants
//! blade")` — and the test name said "at least two". A parser or engine defect granting 3, 4
//! or 5 blades passed it, which is the one thing this card's text rules out.
//!
//! It also only ever staged the member in the centre, so the position gate itself was never
//! observed: a 常時 with the {{center.png|センター}} marker but no gate also passes "centre
//! grants two blades". Both positions are now asserted.
//!
//! Note `center_constant_blade_bonus_test.rs` asserts the same card's effect with the same
//! `>= 2`; whichever of the two survives a future tidy-up, the exact value should go with it.

use crate::helpers::*;

const NATSUMI: &str = "PL!SP-bp4-003-R";

#[test]
fn center_constant_grants_exactly_two_blades_and_nothing_off_centre() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m = game.id(NATSUMI);
    game.assert_card_identity(m, NATSUMI);

    // Centre — the effect applies, and applies for exactly its printed amount.
    game.state.player1.stage.stage = [-1, m, -1];
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(m),
        2,
        "ブレード×2 — the 常時 grants EXACTLY two. A value of 3+ means the amount is not read \
         from the printed text (got {})",
        game.state.mods.get_blade_modifier(m)
    );

    // Left — the whole effect is gated on {{center.png|センター}}, so off-centre is zero.
    game.state.player1.stage.stage = [m, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(m),
        0,
        "左サイド — {{center.png|センター}} gates the entire 常時, so a left-side member gains \
         nothing (got {})",
        game.state.mods.get_blade_modifier(m)
    );

    // Right — likewise, so a gate keyed to the wrong area cannot pass.
    game.state.player1.stage.stage = [-1, -1, m];
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(m),
        0,
        "右サイド — same, the effect is centre-only (got {})",
        game.state.mods.get_blade_modifier(m)
    );

    // And once emptied, still nothing: the 常時 is not granted at all off-stage.
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(m),
        0,
        "not on stage at all → no blades (got {})",
        game.state.mods.get_blade_modifier(m)
    );
}
