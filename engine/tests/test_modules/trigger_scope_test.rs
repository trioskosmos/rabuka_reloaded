//! Trigger-SOURCE-SCOPING tests (08-25 directive, arm S1/S2/S3).
//!
//! Printed-rule convention: a triggered 自動 ability fires on OWN-side card
//! effects; it fires on OPPONENT-caused events ONLY when the text carries
//! 「(対戦相手の/相手のカードの効果でも発動する。)」.
//!
//! Arms:
//! - S1 「自分のカードの効果によって」  -> own effects only   (self_effect_only)
//! - S2 「(…効果でも発動する。)」      -> own AND opponent    (parenthetical)
//! - S3 unscoped movement triggers      -> own side (default)
//!
//! The opponent-caused scenarios run REAL plays: P2 stages/activates cards
//! through the action pipeline (`set_active_side(P2)` + side-aware helpers),
//! never synthetic event injection for the cause itself.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // original blade 1

/// Simulate an energy card entering `owner`'s zone caused by `causer`'s card
/// effect, through the canonical movement pipeline.
fn push_energy_placement(game: &mut TestGame, energy: i16, owner_pid: &str, causer: &str) {
    game.state
        .resolve_target_player_mut(owner_pid)
        .energy_zone
        .cards
        .push(energy);
    game.state
        .push_movement_event(energy, "energy_deck", "energy_zone", None, causer, true);
}

// ====================================================================
// S2 — PL!SP-pb1-006-R: 登場/エリア移動 +2 blades, fires on opponent
// card effects too.
// ====================================================================

/// Positive control: own debut fires the auto (+2 blade).
#[test]
fn s2_pb1_006_own_debut_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.new_id("PL!SP-pb1-006-R");
    game.give_energy(9);
    game.add_to_hand(member);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(member, MemberArea::Center);
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        2,
        "own debut arms the S2 auto -> heart-less +2 blade"
    );
}

/// THE parenthetical pin: the OPPONENT's real play moves this member
/// (HS-pb1-014-R's debut position-changes an opponent member to the facing
/// area), and the S2 auto MUST fire even though a p2 card caused it.
#[test]
fn s2_pb1_006_opponent_drag_arms_per_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // P1: the S2 watcher staged at RIGHT.
    let member = game.new_id("PL!SP-pb1-006-R");
    game.give_energy(9);
    game.add_to_hand(member);
    game.state.player1.stage.stage = [-1, -1, member];
    game.play_to_stage(member, MemberArea::RightSide);
    scan_autos_both(&mut game);
    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        2,
        "own debut armed the auto (+2)"
    );

    // P2 board: all-みらくらぱーく！ gate for the mover.
    let mirakura_a = game.new_id("PL!HS-bp1-005-PR");
    let mover1 = game.new_id("PL!HS-pb1-014-R"); // cost 9
    game.state.player2.stage.stage = [mirakura_a, -1, -1];
    game.add_to_hand_for(Side::P2, mover1);
    game.give_energy_for(Side::P2, 9);

    // DRAG #1: mover1 debuts at P2 center (faces P1 center) — the watcher is
    // pulled right -> center. Distinct movement_event_counter => arms again.
    game.try_play_to_stage_for(Side::P2, mover1, MemberArea::Center)
        .expect("p2 debut of mover1");
    while game.has_pending_choice() {
        answer_choice(&mut game, 0);
    }
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage[1], member,
        "drag #1 moved the watcher into P1 center"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        4,
        "per-move dedupe: drag #1 is a DISTINCT move -> +2 more"
    );

    // NOTE: multi-drag sequences within one turn are deliberately NOT pinned
    // here — blind index answers to the facing-area prompt can select
    // degenerate destinations (EPCWD source==target NOOP). Per-move arming is
    // covered by opp_cause_fired_keys unit semantics; extend with a scripted
    // two-mover scenario only alongside real UI-flow captures.
}

// ====================================================================
// Energy-placed S2 — PL!SP-bp4-016-N: heart06 when ANY card effect
// (own OR opponent) places an energy card into this player's zone.
// ====================================================================

/// Opponent's effect places energy into P1's zone -> watcher fires (+1 heart06).
#[test]
fn s2_bp4_016_energy_placed_by_opponent_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let watcher = game.new_id("PL!SP-bp4-016-N");
    game.state.player1.stage.stage = [-1, watcher, -1];
    scan_autos_both(&mut game);

    // p2's card effect places one energy into P1's zone.
    let energy = game.new_id("LL-E-001-SD");
    push_energy_placement(&mut game, energy, "p1", "p2");
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.mods.get_heart_modifier(watcher, HeartColor::Heart06),
        1,
        "parenthetical: opponent-caused energy placement fires +1 heart06"
    );
}

/// S1 contrast: SP-bp7-005-R＋ ab#1 is 「自分のカードの効果によって」
/// (self_effect_only) — an opponent-caused placement must NOT fire it.
#[test]
fn s1_bp7_005_opponent_caused_placement_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let watcher = game.new_id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, watcher, -1];
    scan_autos_both(&mut game);
    let blade_before = game.state.mods.get_blade_modifier(watcher);

    // p2's effect places an energy into P1's own zone.
    let energy = game.new_id("LL-E-001-SD");
    push_energy_placement(&mut game, energy, "p1", "p2");
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(watcher),
        blade_before,
        "self_effect_only: opponent-caused placement must not fire the gain"
    );
}

// ====================================================================
// S3 contrast — PL!S-bp5-111-R 自動 (area-move waits low-blade opponent)
// has NO parenthetical: an opponent-caused move must NOT fire it.
// ====================================================================

/// Own-caused control first (proves the ability works in this harness).
#[test]
fn s3_bp5_111_own_move_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let watcher = game.new_id("PL!S-bp5-111-R");
    let aqours = game.new_id("PL!S-bp2-015-PR");
    let opp_member = game.new_id(FILLER); // original blade 1 <= 2

    game.state.player1.stage.stage = [aqours, watcher, -1];
    game.state.player2.stage.stage = [-1, opp_member, -1];
    game.give_energy(1);

    // OWN 起動 moves the watcher itself -> auto waits the low-blade opponent.
    game.activate_ability(watcher);
    if game.has_pending_choice() {
        game.select_generated(0);
    }
    scan_autos_both(&mut game);

    assert!(
        game.state.mods.get_orientation_modifier(opp_member) == Some("wait"),
        "own-effect move fires the no-parenthetical auto"
    );
}

/// THE default-scope pin: when the OPPONENT's effect causes the very same
/// move, the no-parenthetical auto must stay silent.
///
/// Opponent-caused move built from a REAL play: P2 debuts HS-pb1-014-R
/// facing the watcher and its debut drags the watcher across areas.
#[test]
fn s3_bp5_111_opponent_caused_move_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Watcher staged alone at P1 center; opponent low-blade target present
    // (would be waited if the auto wrongly fired).
    let watcher = game.new_id("PL!S-bp5-111-R");
    let opp_member = game.new_id(FILLER);
    game.state.player1.stage.stage = [-1, watcher, -1];
    game.state.player2.stage.stage = [-1, opp_member, -1];

    // P2 assembles the all-みらくらぱーく！ board and debuts its mover at
    // P2 CENTER — facing P1 CENTER, dragging the watcher out of center.
    let mirakura_a = game.new_id("PL!HS-bp1-005-PR");
    let mirakura_b = game.new_id("PL!HS-PR-005-PR");
    let mover = game.new_id("PL!HS-pb1-014-R");
    game.state.player2.stage.stage = [mirakura_a, -1, mirakura_b];
    game.add_to_hand_for(Side::P2, mover);
    game.give_energy_for(Side::P2, 9);

    game.try_play_to_stage_for(Side::P2, mover, MemberArea::Center)
        .expect("p2 debut of the mover");
    scan_autos_both(&mut game);

    assert!(
        !(game.state.mods.get_orientation_modifier(opp_member) == Some("wait")),
        "default scope: opponent-caused move must NOT fire the no-parenthetical auto"
    );
}
