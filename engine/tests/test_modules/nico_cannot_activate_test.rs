/// Tests for 矢澤にこ (PL!-pb1-009-R) — 登場: cannot_activate_by_effect restriction.
///
/// ab#0: 登場 → Put 1 opponent member (original blade ≤ 1) to wait.
/// ab#1: 登場 → This turn, members on both stages cannot be made active by effects.
///
/// The cannot_activate_by_effect restriction blocks:
///   1. 起動 ability usage (via handle_use_ability in actions.rs)
///   2. change_state effects that would set a member to "active" (via the
///      check added in execute_change_state in state.rs)
///
/// This test proves #2: a card effect that tries to activate a member
/// (e.g. PL!-pb1-012-R Minami Kotori: 登場 → activate up to 1 Printemps
/// member) is silently blocked when the restriction is active.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// POSITIVE: Nico's cannot_activate_by_effect blocks Kotori's activation effect.
///
/// Flow:
///   1. Deploy a Printemps member to stage, manually set it to wait.
///   2. Deploy Nico — her ab#1 applies cannot_activate_by_effect restriction.
///      ab#0 finds no opponent targets (empty opponent stage) and skips.
///   3. Deploy Kotori — her ab#0 tries to activate the wait Printemps member,
///      but execute_change_state finds the restriction active and skips.
///   4. Verify the Printemps member is still in wait state.
#[test]
fn nico_blocks_kotori_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD"); // Printemps, blade 1, cost 4
    let nico = game.id("PL!-pb1-009-R"); // cost 4
    let kotori = game.id("PL!-pb1-012-R"); // Printemps, cost 2

    // Energy: filler(4) + nico(4) + kotori(2) = 10, give 15.
    game.give_energy(15);

    // ---- Step 1: Deploy Printemps filler ----
    game.add_to_hand(filler);
    game.play_to_stage(filler, MemberArea::LeftSide);
    assert!(
        game.state.mods.get_orientation_modifier(filler).is_none(),
        "Filler should start active (no modifier)"
    );

    // Manually put filler in wait (Nico's ab#0 targets opponent stage).
    game.state.mods.add_orientation_modifier(filler, "wait");
    assert_eq!(
        game.state.mods.get_orientation_modifier(filler).cloned(),
        Some("wait".to_string()),
        "Filler should be in wait after manual setting"
    );

    // ---- Step 2: Deploy Nico ----
    game.add_to_hand(nico);
    game.play_to_stage(nico, MemberArea::Center);
    game.drain_auto_ability_choices();

    // ab#0 fires but opponent stage is empty → no target → skips silently.
    // ab#1 fires → cannot_activate_by_effect restriction is stored.

    // Verify restriction is active.
    assert!(
        game.state
            .cannot_activate_members
            .contains(&"p1".to_string()),
        "cannot_activate_members should contain player1's ID"
    );

    // ---- Step 3: Deploy Kotori ----
    game.add_to_hand(kotori);
    game.play_to_stage(kotori, MemberArea::RightSide);

    // Kotori ab#0: choice to activate up to 1 Printemps member.
    assert!(game.has_pending_choice(), "Kotori ab#0 should prompt");
    // The filler (Printemps, in wait) is the only valid target.
    game.select_indices(&[0]);

    // The cannot_activate_by_effect restriction should have blocked the
    // state change. Filler should still be in wait.
    assert_eq!(
        game.state.mods.get_orientation_modifier(filler).cloned(),
        Some("wait".to_string()),
        "Filler should remain in wait — cannot_activate_by_effect blocked Kotori's activation"
    );
}

/// NEGATIVE: Without Nico's restriction, Kotori's activation works normally.
#[test]
fn kotori_activate_works_without_restriction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD"); // Printemps, blade 1, cost 4
    let kotori = game.id("PL!-pb1-012-R"); // Printemps, cost 2

    // Energy: filler(4) + kotori(2) = 6, give 10.
    game.give_energy(10);

    // Deploy filler to stage.
    game.add_to_hand(filler);
    game.play_to_stage(filler, MemberArea::LeftSide);

    // Manually put filler in wait.
    game.state.mods.add_orientation_modifier(filler, "wait");
    assert_eq!(
        game.state.mods.get_orientation_modifier(filler).cloned(),
        Some("wait".to_string()),
        "Filler should be in wait after manual setting"
    );

    // Deploy Kotori.
    game.add_to_hand(kotori);
    game.play_to_stage(kotori, MemberArea::Center);

    // Kotori ab#0: choice to activate Printemps member.
    assert!(game.has_pending_choice(), "Kotori ab#0 should prompt");
    game.select_indices(&[0]);

    // No restriction → activation should succeed.
    // Filler is now active (no wait modifier).
    let ori = game.state.mods.get_orientation_modifier(filler).cloned();
    assert!(
        ori != Some("wait".to_string()),
        "Filler should no longer be in wait (activation succeeded). Got: {:?}",
        ori
    );
}
