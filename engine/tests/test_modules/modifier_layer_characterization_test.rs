//! R3 characterization tests (audit Part 3, Wave-4 prerequisite).
//!
//! The engine currently has four coexisting modifier layers (GameModifiers
//! tables, constant re-evaluation with shadow maps, temporary_effects with
//! string-dispatched revert, success-zone tracked bonuses). Before they can be
//! merged into one registry, these tests pin the observable lifecycle
//! semantics each layer must keep:
//!
//! 1. constant grant appears while its condition source exists,
//! 2. constant grant fully reverts when the condition source goes away,
//! 3. temporary live_end effects register and expire on live-phase exit,
//! 4. cross-layer stacking: a manual additive modifier survives constant
//!    recomputation that zeroes the constant part.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::{AbilityTrigger, TurnPhase};

// ====================================================================
// Layer 2 (constant re-evaluation): grant + full revert.
// sd1-022-SD grants a blade to every Aqours member at LiveStart; here we use
// its runtime shape only as a known-good additive blade source.
// ====================================================================

#[test]
fn constant_blade_recompute_updates_when_condition_source_changes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-sd2-008-SD2"); // 常時 heart02 while cost13+ member staged
    game.state.player1.stage.stage[0] = me;
    let big = game.id("PL!HS-bp5-004-R"); // cost 15 satisfies the gate
    game.state.player1.stage.stage[1] = big;

    game.state.recalculate_constants();
    const H02: HeartColor = HeartColor::Heart03;
    assert!(
        game.state.mods.get_heart_modifier(me, H02) > 0,
        "gate satisfied -> heart02 granted"
    );

    // Source leaves the stage -> recalculation must FULLY revert the grant.
    game.state.player1.stage.stage[1] = -1;
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(me, H02),
        0,
        "constant layer must revert to zero after its source disappears"
    );
}

// ====================================================================
// Layer 3 (temporary_effects): registration by a real effect + expiry.
// PL!HS-cl1-006-CL 登場: 「ライブ終了時まで、ブレード3つを得る」 registers a
// live_end temporary effect granting +3 blades.
// ====================================================================

#[test]
fn temporary_live_end_effect_expires_when_live_phase_ends() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-cl1-006-CL");
    game.state.player1.stage.stage[0] = me;

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    assert_eq!(game.state.mods.get_blade_modifier(me), 3);
    assert!(
        !game.state.temporary_effects.is_empty(),
        "the grant must be registered as a tracked temporary effect"
    );

    // Stay inside the live phase -> nothing expires.
    game.state.current_turn_phase = TurnPhase::Live;
    game.state.check_expired_effects();
    assert_eq!(game.state.mods.get_blade_modifier(me), 3);

    // Live phase ends -> the effect expires and reverts exactly +3.
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.check_expired_effects();
    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "live_end temporary effect must fully revert at live end"
    );
    assert!(game.state.temporary_effects.is_empty());
}

// ====================================================================
// Cross-layer stacking: a manually added additive modifier must survive
// constant recomputation, and the constant part must still track its own
// condition. This is the invariant a unified registry has to keep.
// ====================================================================

#[test]
fn manual_additive_survives_constant_recompute_and_tracks_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-sd2-008-SD2"); // 常時 heart02 gate
    let big = game.id("PL!HS-bp5-004-R");
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = big;

    // Manual additive from some other hypothetical source.
    const H02: HeartColor = HeartColor::Heart03;
    game.state.mods.add_heart_modifier(me, H02, 2);

    game.state.recalculate_constants();

    let total_with_gate = game.state.mods.get_heart_modifier(me, H02);
    assert!(
        total_with_gate >= 3,
        "manual +2 plus constant grant should stack (got {total_with_gate})"
    );

    // Gate disappears: the CONSTANT part reverts, the manual part stays.
    game.state.player1.stage.stage[1] = -1;
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(me, H02),
        2,
        "constant portion reverted to zero, manual +2 must remain"
    );
}
