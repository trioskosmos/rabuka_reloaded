//! R3 slice 2 characterization: the prohibition_effects layer.
//!
//! Restrictions are currently stored as strings and matched by substring
//! (`is_action_prohibited` does `e.contains(action)`). Before this layer can
//! be typed, pin its observable lifecycle:
//!
//! - a conditional cannot_live registers only while its gate holds
//! - an unconditional cannot_place registers from a live card in play
//! - clearing/recompute must remove stale prohibitions

use crate::helpers::*;

// ====================================================================
// PL!SP-bp1-001-R (常時):
// 「自分のステージにほかのメンバーがいない場合、自分はライブできない。」
// ====================================================================

#[test]
fn kanon_alone_on_stage_prohibits_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp1-001-R");
    game.state.player1.stage.stage[0] = me;

    game.state.recalculate_constants();

    assert!(
        game.state.is_action_prohibited("cannot_live"),
        "alone on stage -> live prohibited"
    );
}

#[test]
fn kanon_with_teammate_live_allowed_again() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp1-001-R");
    let mate = game.new_id("PL!HS-bp5-004-R");
    game.state.player1.stage.stage = [me, mate, -1];

    game.state.recalculate_constants();

    assert!(
        !game.state.is_action_prohibited("cannot_live"),
        "teammate present -> gate fails -> no prohibition"
    );
}

#[test]
fn removing_teammate_reprohibits_without_stale_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp1-001-R");
    let mate = game.new_id("PL!HS-bp5-004-R");
    game.state.player1.stage.stage = [me, mate, -1];
    game.state.recalculate_constants();
    assert!(!game.state.is_action_prohibited("cannot_live"));

    // Teammate leaves; recalculation must re-register the prohibition.
    game.state.player1.stage.stage[1] = -1;
    game.state.recalculate_constants();

    assert!(
        game.state.is_action_prohibited("cannot_live"),
        "stale allow-state must not survive recomputation"
    );
}

// ====================================================================
// NOTE on cannot_place (PL!S-bp2-024-L): that restriction does NOT register
// through recalculate_constants (live-card-zone constants are not scanned).
// It is enforced at placement time via GameState::can_place_card_in_zone
// checking the card's own printed Restriction ability — already covered end-
// to-end by kagayaiteru_q125_cannot_place_in_success_zone. Do NOT add a
// recalc-based characterization for it here.
// ====================================================================
