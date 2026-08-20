/// Tests SELF CONTROL!! (PL!S-bp5-022-L) live_start ability granting blade
/// to members who moved areas this turn, combined with 鹿角聖良's
/// position_change activation ability.
///
/// SELF CONTROL!! ab#0 (LiveStart):
///   ライブ終了時まで、自分のステージにいる、
///   このターン中にエリアを移動したメンバーはブレードを得る。
///   → gain_resource blade count=1 timing_condition="moved_this_turn" duration="live_end"
///
/// 鹿角聖良 (PL!S-bp5-111-R) ab#0 (Activation):
///   E：このメンバーを『Aqours』か『SaintSnow』のメンバーがいるエリアにポジションチェンジする。
///   → position_change to area with Aqours/SaintSnow member
///
/// Scenario:
///   1. Place Seira A (center), Seira B (right), filler (left) on stage.
///   2. Activate Seira A's kidou: pay 1 energy, position_change to right (swap with Seira B).
///   3. Both Seira A and Seira B have now moved areas.
///   4. Advance to LiveStart with SELF CONTROL!! as live card.
///   5. SELF CONTROL!! fires: only Seira A and Seira B get blade; filler does not.
use crate::helpers::*;
use crate::test_modules::bp7_wait_immunity_helpers::*;

/// 聖良's ab#1 (area-move trigger) wait is blocked by wait-immunity.
#[test]
fn seira_area_move_wait_blocked_by_immunity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Player2 protects their 果南 (blade 2).
    let p2_kanan = p2_establish_wait_immunity(&mut game);

    // Player1: 聖良 at center, filler at right (destination for position change).
    let seira = game.id("PL!S-bp5-111-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = seira;
    game.state.player1.stage.stage[2] = filler;
    game.give_energy(10);

    // Activate 聖良 ab#0 (position change) → 聖良 moves areas → ab#1 fires and
    // would wait the opponent's blade≤2 member (果南, blade 2).
    game.activate_ability(seira);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let t = game.pending_choice_type();
        match t.as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            Some("SelectPosition") | Some("SelectTarget") => game.select_option(1),
            _ => game.select_indices(&[0]),
        }
    }

    assert!(
        !is_waited(&game, p2_kanan),
        "聖良's area-move wait must be blocked by wait-immunity"
    );
}

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state
            .player1
            .energy_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }
}

fn advance_to_live_start(game: &mut TestGame, live_card: i16) {
    game.add_to_hand(live_card);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass(); // LiveCardSetP2
    game.pass(); // LiveStart
    while game.has_pending_choice() {
        game.drain_auto_ability_choices();
    }
}

#[test]
fn position_change_triggered_grant_blade_to_moved_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);

    let self_control = game.id("PL!S-bp5-022-L");
    let seira_a = game.id("PL!S-bp5-111-R");
    let seira_b = game.id("PL!S-bp5-111-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(10);

    // Place members on stage: filler left, seira_a center, seira_b right
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = seira_a;
    game.state.player1.stage.stage[2] = seira_b;

    // Sanity: stage layout before swap
    assert_eq!(game.state.player1.stage.stage[0], filler, "filler at left");
    assert_eq!(
        game.state.player1.stage.stage[1], seira_a,
        "seira_a at center"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], seira_b,
        "seira_b at right"
    );

    // Activate Seira A's kidou ability (ab#0: position_change)
    game.activate_ability(seira_a);

    // Should prompt for destination position choice
    assert!(
        game.has_pending_choice(),
        "Expected position choice after activating Seira A's ability"
    );

    // Find the "right" position option (where seira_b is sitting)
    let actions = game.generated_actions();
    let right_idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == "right")
        })
        .expect("Should have a 'right' position option");

    // Select right (swap seira_a → right, seira_b → center)
    game.select_generated(right_idx);

    // Drain any auto-ability choices (Seira's jidou etc.)
    game.drain_auto_ability_choices();

    // Verify: swap happened
    assert_eq!(
        game.state.player1.stage.stage[1], seira_b,
        "seira_b should now be at center after swap"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], seira_a,
        "seira_a should now be at right after swap"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], filler,
        "filler should still be at left"
    );

    // Verify: movement was recorded for seira_a and seira_b but NOT filler
    assert!(
        game.state.has_card_moved_this_turn(seira_a),
        "seira_a should be recorded as moved"
    );
    assert!(
        game.state.has_card_moved_this_turn(seira_b),
        "seira_b should be recorded as moved"
    );
    assert!(
        !game.state.has_card_moved_this_turn(filler),
        "filler should NOT be recorded as moved"
    );

    // Verify turn_area_movements has both seira cards
    let area_moved_ids: std::collections::HashSet<i16> = game
        .state
        .turn_area_movements
        .iter()
        .map(|m| m.moved_card_id)
        .collect();
    assert!(
        area_moved_ids.contains(&seira_a),
        "seira_a in turn_area_movements"
    );
    assert!(
        area_moved_ids.contains(&seira_b),
        "seira_b in turn_area_movements"
    );
    assert!(
        !area_moved_ids.contains(&filler),
        "filler NOT in turn_area_movements"
    );

    // Advance to LiveStart — SELF CONTROL!! fires and grants blade
    advance_to_live_start(&mut game, self_control);

    // Verify: only seira_a and seira_b got blade; filler did not
    assert_eq!(
        game.state.mods.get_blade_modifier(seira_a),
        1,
        "Seira A (moved) should get 1 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(seira_b),
        1,
        "Seira B (moved) should get 1 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(filler),
        0,
        "Filler (unmoved) should get 0 blade"
    );
}
