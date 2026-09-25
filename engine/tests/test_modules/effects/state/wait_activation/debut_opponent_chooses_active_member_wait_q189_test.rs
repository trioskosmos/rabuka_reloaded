use crate::helpers::*;
use crate::test_modules::support::bp7_wait_immunity_helpers::*;
use rabuka_engine::zones::MemberArea;

/// にこ's debut wait is blocked by 松浦果南's wait-immunity on the opponent's member.
#[test]
fn debut_opponent_wait_is_blocked_by_wait_immunity() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Player2 protects their 果南 (Aqours, blade 2).
    let p2_kanan = p2_establish_wait_immunity(&mut game);
    assert!(
        game.state
            .wait_immune_members
            .iter()
            .any(|(m, o)| *m == p2_kanan && o == "p2"),
        "player2's 果南 should be protected"
    );

    // Player1 plays にこ → would wait an opponent (player2) active member.
    let nico = game.id("PL!-bp4-009-R");
    game.add_to_hand(nico);
    game.give_energy(10);
    game.play_to_stage(nico, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !is_waited(&game, p2_kanan),
        "にこ's opponent wait must be blocked by wait-immunity"
    );
}

/// PL!-bp4-009-R (矢澤にこ) Q189: Debut — opponent chooses 1 of their own active members to wait.
#[test]
fn debut_opponent_waits_only_active_member_q189() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let nico = game.id("PL!-bp4-009-R");
    let p2_member = game.id("PL!-sd1-010-SD");

    // Opponent has an active member on stage
    game.state.player2.stage.stage[0] = p2_member;
    game.add_to_hand(nico);
    game.give_energy(10);
    game.play_to_stage(nico, MemberArea::Center);

    // Debut fires: opponent waits 1 of their own active members
    // (with only 1 eligible target, the effect auto-resolves)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify the member is now in wait state (stays on stage)
    assert!(
        game.state.player2.stage.stage.contains(&p2_member),
        "Opponent member should still be on stage"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(p2_member),
        Some("wait"),
        "Opponent member should be in wait state"
    );
}

/// PL!-bp4-009-R (矢澤にこ): Multiple opponent members — forces a choice.
#[test]
fn debut_opponent_chooses_which_active_member_to_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let nico = game.id("PL!-bp4-009-R");
    let p2_member_a = game.id("PL!-sd1-010-SD");
    let p2_member_b = game.id("PL!-sd1-013-SD");

    // Opponent has 2 active members on stage
    game.state.player2.stage.stage = [p2_member_a, p2_member_b, -1];
    game.add_to_hand(nico);
    game.give_energy(10);
    game.play_to_stage(nico, MemberArea::Center);

    // Debut fires: opponent must choose which member to wait
    assert!(
        game.has_pending_choice(),
        "Opponent must choose which member to wait with 2+ eligible"
    );
    let entry = game.state.ability_queue.current_entry();
    assert_eq!(
        entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Wait-member choice should be routed to opponent"
    );

    // Opponent selects p2_member_b (index 1)
    game.select_indices(&[1]);

    // Verify: p2_member_b is waited, p2_member_a stays active
    assert_eq!(
        game.state.mods.get_orientation_modifier(p2_member_b),
        Some("wait"),
        "p2_member_b should be in wait state"
    );
    assert!(
        game.state
            .mods
            .get_orientation_modifier(p2_member_a)
            .is_none()
            || game.state.mods.get_orientation_modifier(p2_member_a) == Some("active"),
        "p2_member_a should stay active (not waited)"
    );
}
