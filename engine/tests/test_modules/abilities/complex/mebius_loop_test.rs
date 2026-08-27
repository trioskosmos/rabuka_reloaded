/// Tests for 逃走迷走メビウスループ (PL!S-pb1-022-L) — ライブ成功時:
/// restriction preventing both players from placing in the success live card
/// zone (when total scores are equal at winner determination).
///
/// Card text: ライブ成功時: このターン、ライブに勝利するプレイヤーを決定するとき、
/// 自分と相手のライブの合計スコアが同じ場合、ライブ終了時まで、自分と相手は
/// 成功ライブカード置き場にカードを置くことができない。
///
/// Q36 (2025-08-04): ライブ成功時 fires after both players' performance
/// phase, at the start of the live victory determination phase, BEFORE
/// the winning player is determined.
///
/// Same ability is on `PL!S-pb1-022-L＋`.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_victory(game: &mut TestGame) {
    for _ in 0..3 {
        game.pass();
    }
}

/// Build a stage member that produces lots of heart04 so the mebius live
/// (need_heart: heart04×3 + heart0×3) can succeed. PL!HS-bp2-001-R has
/// heart04×3 per member. Its 起動 ability won't fire unless we call
/// activate_ability, so it's safe to use as a passive stage filler.
fn fill_p1_stage_with_heart04(game: &mut TestGame) {
    let m1 = game.id("PL!HS-bp2-001-R");
    let m2 = game.id("PL!HS-bp2-001-R");
    let m3 = game.id("PL!HS-bp2-001-R");
    game.state.player1.stage.stage = [m1, m2, m3];
}

fn fill_p2_stage_with_heart04(game: &mut TestGame) {
    let m1 = game.id("PL!HS-bp2-001-R");
    let m2 = game.id("PL!HS-bp2-001-R");
    let m3 = game.id("PL!HS-bp2-001-R");
    game.state.player2.stage.stage = [m1, m2, m3];
}

/// POSITIVE: Both players set mebius loop. Both stages have plenty of
/// heart04. Both lives succeed. With tied scores (both mebius = score 2),
/// the cannot_place restriction fires, and NEITHER mebius card goes to
/// either player's success live card zone — they go to waitroom instead.
#[test]
fn mebius_blocks_both_success_zones_on_tied_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mebius_p1 = game.id("PL!S-pb1-022-L");
    let mebius_p2 = game.id("PL!S-pb1-022-L");
    let filler = game.id("PL!-sd1-010-SD");

    fill_p1_stage_with_heart04(&mut game);
    fill_p2_stage_with_heart04(&mut game);

    game.state.player1.hand.cards.push(mebius_p1);
    game.state.player2.hand.cards.push(mebius_p2);

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    // P1 sets mebius (first attacker).
    game.set_live_card(mebius_p1);
    // Transition phase to LiveCardSetSecondAttacker.
    game.pass();
    // P2 sets mebius (second attacker).
    game.set_live_card(mebius_p2);

    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // One more pass finalizes victory determination (winner placement).
    game.pass();

    // Both lives must have SUCCEEDED (stage supplies 9 heart04 vs need
    // heart04×3+heart0×3) with EQUAL totals. Proven success + equal scores +
    // non-placement together isolate the cannot-place restriction as the
    // blocker — the transient prohibition itself is already cleaned up by
    // live end and cannot be inspected here.
    let mut p1_snap: Option<(bool, u8)> = None;
    let mut p2_snap: Option<(bool, u8)> = None;
    for snap in &game.state.performance_snapshots {
        let entry = (snap.success, snap.total_score);
        if snap.player_id == game.state.player1.id {
            p1_snap = Some(entry);
        } else {
            p2_snap = Some(entry);
        }
        assert!(
            snap.success,
            "[{}] live must have succeeded; otherwise non-placement \
             proves nothing about the restriction",
            snap.player_id
        );
    }
    assert_eq!(
        p1_snap.map(|(_, s)| s),
        p2_snap.map(|(_, s)| s),
        "totals must be tied for the restriction's condition"
    );

    // With both mebius' restrictions fired, neither card should be in
    // either player's success zone.
    assert!(
        !game
            .state
            .player1
            .success_live_card_zone
            .cards
            .contains(&mebius_p1),
        "Mebius restriction should prevent P1's mebius from success zone"
    );
    assert!(
        !game
            .state
            .player2
            .success_live_card_zone
            .cards
            .contains(&mebius_p2),
        "Mebius restriction should prevent P2's mebius from success zone"
    );
    // Each player's own mebius ends in their OWN waitroom (Rule 8.4 cleanup):
    // exact per-player placement, no cross-player ambiguity.
    let p1_waitroom = &game.state.player1.waitroom.cards;
    let p2_waitroom = &game.state.player2.waitroom.cards;
    assert!(
        p1_waitroom.contains(&mebius_p1) && !p2_waitroom.contains(&mebius_p1),
        "P1's mebius should be in P1's OWN waitroom (p1={:?}, p2={:?})",
        p1_waitroom,
        p2_waitroom
    );
    assert!(
        p2_waitroom.contains(&mebius_p2) && !p1_waitroom.contains(&mebius_p2),
        "P2's mebius should be in P2's OWN waitroom (p1={:?}, p2={:?})",
        p1_waitroom,
        p2_waitroom
    );
}

/// NEGATIVE: P1 sets mebius, P2 sets a different (non-restriction) live
/// card. P2's live has a different score so totals are NOT tied. The
/// condition (scores equal) should FAIL, the restriction should NOT fire,
/// and P2's live card should go to P2's success zone normally.
#[test]
fn mebius_does_not_fire_on_untied_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mebius_p1 = game.id("PL!S-pb1-022-L");
    // P2 sets a live whose requirements (heart06×5 + heart0×7) are impossible
    // for this board, so its life FAILS: totals are 2 vs 0 — genuinely
    // untied. (The previous "score-1" picks all carried エール score-icon
    // riders that tied the game at 2-2.)
    let normal_live_p2 = game.id("PL!N-bp1-028-L");
    let filler = game.id("PL!-sd1-010-SD");

    fill_p1_stage_with_heart04(&mut game);
    fill_p2_stage_with_heart04(&mut game);

    game.state.player1.hand.cards.push(mebius_p1);
    game.state.player2.hand.cards.push(normal_live_p2);

    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(mebius_p1);
    game.pass();
    game.set_live_card(normal_live_p2);

    // Transition through both performance phases.
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // One more pass finalizes victory determination (winner placement),
    // matching live_success_rules_test::both_players_live_score_compared.
    game.pass();

    // Untied totals (P1's successful mebius 2 vs P2's failed live 0): the
    // 「合計スコアが同じ場合」condition FAILS, so the cannot-place restriction
    // must not fire and P1 places normally as the winner.
    let p1_success = game.state.player1.success_live_card_zone.cards.clone();
    let p2_success = game.state.player2.success_live_card_zone.cards.clone();
    assert_eq!(
        p1_success.as_slice(),
        &[mebius_p1][..],
        "untied 2-vs-0: P1's winning mebius reaches its own success zone"
    );
    assert!(
        !p2_success.contains(&normal_live_p2),
        "untied: P2's failed live places nothing"
    );
    assert!(
        game.state.player2.waitroom.cards.contains(&normal_live_p2),
        "P2's failed live card ends in the waitroom"
    );
}
