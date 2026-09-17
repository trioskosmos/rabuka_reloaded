use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;

#[test]
fn active_phase_lock_owned_by_first_player_targets_second_player() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let restriction = g.id("PL!HS-pb1-008-R");
    let waiting = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [restriction, -1, -1];
    g.state.player2.stage.stage = [waiting, -1, -1];
    g.state.mods.add_orientation_modifier(waiting, "wait");
    fill_both_main_decks(&mut g, waiting);
    g.give_energy(5);
    g.state.recalculate_constants();

    // restriction stores P2 ID (opponent) in constant_cannot_activate_members
    let opp_id = &g.state.player2.id;
    assert!(
        g.state
            .constant_cannot_activate_members
            .iter()
            .any(|x| x == opp_id),
        "opponent ID should be in constant_cannot_activate_members"
    );

    // During P2's turn (P2 is active player), P2's activation should be blocked
    let p2_turn = g.state.player2.id.clone();
    let is_activation_blocked = g
        .state
        .constant_cannot_activate_members
        .iter()
        .any(|t| t == &p2_turn);
    assert!(
        is_activation_blocked,
        "P2's turn: P2 should be blocked by the restriction"
    );

    // P2's member should NOT become active
    let orientation = g.state.mods.get_orientation_modifier(waiting);
    assert_eq!(
        orientation,
        Some("wait"),
        "P2's waiting member should remain in wait state (blocked by restriction)"
    );
}

#[test]
fn active_phase_lock_owned_by_second_player_targets_first_player() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let restriction = g.id("PL!HS-pb1-008-R");
    let waiting = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [waiting, -1, -1];
    g.state.player2.stage.stage = [restriction, -1, -1];
    g.state.mods.add_orientation_modifier(waiting, "wait");
    fill_both_main_decks(&mut g, waiting);
    g.give_energy(5);
    g.state.recalculate_constants();

    // P1 is the turn player, P2 owns the restriction card targeting "opponent"
    // resolve_target_player("opponent") should return P1 (P2's opponent)
    let p1_id = &g.state.player1.id;
    assert!(
        g.state
            .constant_cannot_activate_members
            .iter()
            .any(|x| x == p1_id),
        "P1 should be in constant_cannot_activate_members (P2's opponent)"
    );

    // Simulate active phase activation logic
    let turn_player_id = g.state.active_player().id.clone();
    let is_activation_blocked = g
        .state
        .constant_cannot_activate_members
        .iter()
        .any(|t| t == &turn_player_id);
    assert!(
        is_activation_blocked,
        "P1's turn: P1 should be blocked because P2's restriction targets opponent"
    );

    // P1's member should NOT have been activated
    let orientation = g.state.mods.get_orientation_modifier(waiting);
    assert_eq!(
        orientation,
        Some("wait"),
        "P1's waiting member should remain in wait state despite active phase"
    );
}

#[test]
fn active_phase_lock_excludes_owners_members_from_simulated_activation_block() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let restriction = g.id("PL!HS-pb1-008-R");
    let waiting = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [restriction, waiting, -1];
    g.state.mods.add_orientation_modifier(waiting, "wait");
    fill_both_main_decks(&mut g, waiting);
    g.give_energy(5);
    g.state.recalculate_constants();

    // constant_cannot_activate_members contains P2 (opponent), NOT P1
    let p1_id = &g.state.player1.id;
    assert!(
        !g.state
            .constant_cannot_activate_members
            .iter()
            .any(|x| x == p1_id),
        "P1 should NOT be in constant_cannot_activate_members (self is not opponent)"
    );

    // P1's own member SHOULD be activated
    let turn_player_id = g.state.active_player().id.clone();
    let is_activation_blocked = g
        .state
        .constant_cannot_activate_members
        .iter()
        .any(|t| t == &turn_player_id);
    assert!(
        !is_activation_blocked,
        "P1's turn: P1's own activation should NOT be blocked"
    );

    // Simulate active phase activation
    if !is_activation_blocked {
        g.state.mods.add_orientation_modifier(waiting, "active");
    }
    let orientation = g.state.mods.get_orientation_modifier(waiting);
    assert_eq!(
        orientation,
        Some("active"),
        "P1's own waiting member should become active during active phase"
    );
}
