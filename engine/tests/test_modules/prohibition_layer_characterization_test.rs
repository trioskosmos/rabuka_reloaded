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
// PL!S-bp2-024-L (常時): 「このカードは成功ライブカード置き場に置くことができない。」
//
// Enforced at PLACEMENT TIME via GameState::can_place_card_in_zone scanning
// the card's own printed Restriction ability — NOT via prohibition_effects
// registration (live-card-zone constants are not scanned by recalc).
// End-to-end flow covered by kagayaiteru_q125_cannot_place_in_success_zone.
// Here we pin the validation PRIMITIVE itself, incl. the
// LiveCardZone <-> SuccessLiveZone interchangeability rule.
// ====================================================================

#[test]
fn bp2024_cannot_place_blocks_both_live_zones() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let live = game.id("PL!S-bp2-024-L");

    assert!(
        !game
            .state
            .can_place_card_in_zone(live, "success_live_card_zone", "p1"),
        "printed cannot_place must block success-zone placement"
    );
    assert!(
        !game.state.can_place_card_in_zone(live, "live_card_zone", "p1"),
        "LiveCardZone <-> SuccessLiveZone are interchangeable for cannot_place"
    );
}

#[test]
fn bp2024_positive_control_normal_live_card_is_placeable() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let normal = game.id("PL!N-bp1-025-L"); // 虹ヶ咲 live card, no restriction

    assert!(
        game.state
            .can_place_card_in_zone(normal, "success_live_card_zone", "p1"),
        "a live card without the restriction must be placeable"
    );
}
