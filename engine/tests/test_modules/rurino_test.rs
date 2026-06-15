/// Tests for PL!HS-bp6-003-R 大沢瑠璃乃 (Rurino Osawa) — Debut ability:
///
/// 登場：自分のステージにいるウェイト状態の「みらくらぱーく！」のメンバー1人を
/// アクティブにしてもよい。そうした場合、自分の控え室から「みらくらぱーく！」の
/// ライブカードを1枚手札に加える。
///
/// Key behavior: the optional activation should NOT be offered when no wait
/// "みらくらぱーく！" members exist on stage (unpayable option). When the
/// option IS offered and the player pays, the subsequent move_cards fires.
/// When the player skips, it doesn't.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Helper: set up Rurino + a cheap Mirakura Park member + a Mirakura Park live
/// in discard. Returns (rurino, mirakura_member, live_card_id).
fn setup_rurino(game: &mut TestGame) -> (i16, i16, i16) {
    let rurino = game.id("PL!HS-bp6-003-R");
    let mirakura_member = game.id("PL!HS-bp6-011-R"); // cost 2, unit=みらくらぱーく！
    let live_card = game.id("PL!HS-PR-012-PR"); // live, unit=みらくらぱーく！

    // Energy: rurino(11) + mirakura_member(2) = 13, give 15.
    game.give_energy(15);

    // Put Mirakura Park live card in discard for the second action.
    game.state.player1.waitroom.cards.push(live_card);

    // Put the cheap member on stage in wait state.
    game.state.player1.stage.stage[0] = mirakura_member;
    game.state
        .mods
        .add_orientation_modifier(mirakura_member, "wait");

    // Put Rurino in hand.
    game.add_to_hand(rurino);

    (rurino, mirakura_member, live_card)
}

/// POSITIVE: Wait Mirakura Park member exists → optional offered → pay →
/// member activated → live card moved from discard to hand.
#[test]
fn rurino_activates_wait_member_and_adds_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (rurino, mirakura_member, live_card) = setup_rurino(&mut game);

    // Verify preconditions
    assert_eq!(
        game.state
            .mods
            .get_orientation_modifier(mirakura_member)
            .cloned(),
        Some("wait".to_string()),
        "Member should start in wait"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_card),
        "Live card should be in discard"
    );

    // Deploy Rurino
    game.play_to_stage(rurino, MemberArea::Center);

    // Optional change_state prompt: "pay or skip"
    assert!(
        game.has_pending_choice(),
        "Should have optional activation prompt"
    );
    game.select_option(1); // pay

    // After the change_state resolves, the member should be active.
    let ori = game
        .state
        .mods
        .get_orientation_modifier(mirakura_member)
        .cloned();
    assert!(
        ori != Some("wait".to_string()),
        "Member should no longer be in wait (was activated). Got: {:?}",
        ori
    );

    // The "そうした場合" (conditional) action should have fired:
    // live card moved from discard to hand.
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_card),
        "Live card should be removed from discard"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live_card),
        "Live card should be in hand"
    );
}

/// NEGATIVE: No wait Mirakura Park member → optional NOT offered →
/// live card stays in discard.
#[test]
fn rurino_no_wait_member_skips_entire_sequence() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp6-003-R");
    let live_card = game.id("PL!HS-PR-012-PR");
    let mirakura_member = game.id("PL!HS-bp6-011-R");

    game.give_energy(15);
    game.state.player1.waitroom.cards.push(live_card);

    // Put member on stage in ACTIVE state (not wait).
    game.state.player1.stage.stage[0] = mirakura_member;

    game.add_to_hand(rurino);
    game.play_to_stage(rurino, MemberArea::Center);

    // The optional activation should NOT be offered (no wait targets).
    // The entire conditional sequence is skipped.
    assert!(
        !game.has_pending_choice(),
        "Should NOT have optional prompt when no wait member exists"
    );

    // Live card should still be in discard — second action didn't fire.
    assert!(
        game.state.player1.waitroom.cards.contains(&live_card),
        "Live card should remain in discard (sequence was skipped)"
    );
}

/// NEGATIVE: Wait member exists but player skips → live card stays in discard.
#[test]
fn rurino_skip_optional_leaves_live_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (rurino, _mirakura_member, live_card) = setup_rurino(&mut game);

    game.play_to_stage(rurino, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Should have optional activation prompt"
    );
    game.select_option(0); // skip

    // Live card should still be in discard — second action didn't fire.
    assert!(
        game.state.player1.waitroom.cards.contains(&live_card),
        "Live card should remain in discard (skipped)"
    );
}

/// POSITIVE: Handle empty discard — live card not found but activation still
/// works (the move_cards for live card simply finds nothing).
#[test]
fn rurino_empty_discard_no_crash() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp6-003-R");
    let mirakura_member = game.id("PL!HS-bp6-011-R");

    game.give_energy(15);
    // No live card in discard — second action will find nothing.
    game.state.player1.stage.stage[0] = mirakura_member;
    game.state
        .mods
        .add_orientation_modifier(mirakura_member, "wait");
    game.add_to_hand(rurino);

    game.play_to_stage(rurino, MemberArea::Center);
    assert!(game.has_pending_choice(), "Should have optional prompt");
    game.select_option(1); // pay

    // Activation succeeded, the conditional move_cards runs but finds nothing.
    // No crash.
    let ori = game
        .state
        .mods
        .get_orientation_modifier(mirakura_member)
        .cloned();
    assert!(
        ori != Some("wait".to_string()),
        "Member should be activated"
    );
}

/// Tests for PL!HS-bp5-003-R＋ — LiveStart ability:
/// (ab#1) 手札を1枚控え室に置いてもよい: ライブ終了時まで、
/// これにより控え室に置いたカードと同じグループ名を持つメンバー1人は、heart01を得る。
#[test]
fn rurino_bp5_live_start_gains_heart_from_discarded_group() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp5-003-R\u{ff0b}");
    let mirakura_member = game.id("PL!HS-bp6-011-R"); // みらくらぱーく！
    let cost_card = game.id("PL!HS-bp6-011-R"); // same group for discard
    let filler = game.id("PL!-sd1-010-SD");

    // Set up stage: Rurino + one other みらくらぱーく member
    game.state.player1.stage.stage = [rurino, mirakura_member, filler];
    // Hand card with matching group name
    game.state.player1.hand.cards.push(cost_card);
    // Live card in hand for set_live_card
    let live_card = game.id("PL!HS-PR-012-PR"); // みらくらぱーく！ live
    game.state.player1.hand.cards.push(live_card);
    // Fill deck
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Energy for live card cost
    game.give_energy(10);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    // Advance to trigger LiveStart abilities
    game.pass();
    game.pass();

    // LiveStart triggers — optional cost prompt (discard 1 from hand)
    assert!(
        game.has_pending_choice(),
        "Should have optional cost prompt"
    );
    game.select_option(1); // pay

    // The effect auto-selects the only eligible member and applies heart01.
    // Verify heart01 modifier was applied to Rurino (same group as discarded card)
    assert!(
        game.state.mods.heart_modifiers.contains_key(&rurino),
        "Rurino should have a heart modifier"
    );
    if let Some(heart_mod) = game.state.mods.heart_modifiers.get(&rurino) {
        let h1 = heart_mod.get(&rabuka_engine::card::HeartColor::Heart01);
        assert!(h1.is_some(), "Heart01 should be present");
        assert!(h1.unwrap().total() >= 1, "Heart01 should be >= 1");
    }
    assert!(!game.has_pending_choice(), "No pending choices after setup");
}
