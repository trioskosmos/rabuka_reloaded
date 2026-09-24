use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;
use rabuka_engine::zones::MemberArea;

fn play_q242_debut(game: &mut TestGame, card_id: i16) {
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(13);
    game.state.player1.hand.cards.push(card_id);
    game.play_to_stage(card_id, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

#[test]
fn debut_both_waitrooms_member_restore_shrinks_own_waitroom() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!HS-pb1-012-R");
    let m = g.id("PL!-sd1-001-SD");
    for _ in 0..5 {
        g.state.player1.waitroom.cards.push(m);
        g.state.player2.waitroom.cards.push(m);
    }
    let w1 = g.state.player1.waitroom.cards.len();
    play_q242_debut(&mut g, c);
    assert!(
        g.state.player1.waitroom.cards.len() < w1,
        "P1 waitroom shrank"
    );
}

fn trigger_member_restore_debut_and_check_blade_gain(game: &mut TestGame, card_id: i16) -> bool {
    play_q242_debut(game, card_id);
    let blade = game.state.mods.get_blade_modifier(card_id);
    blade > 0
}

#[test]
fn twenty_restored_members_recover_live_and_gain_two_blades_q242() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD"); // member card
    let live = game.id("PL!-sd1-019-SD"); // live card
    let f = game.id("PL!-sd1-010-SD"); // filler
    game.state.player1.stage.stage = [-1, -1, -1];

    // 10 member cards each in both players' discards = 20 total (hits threshold)
    for _ in 0..10 {
        game.state.player1.waitroom.cards.push(m);
        game.state.player2.waitroom.cards.push(m);
    }
    // Live card in P1's discard for retrieval
    game.state.player1.waitroom.cards.push(live);
    let w1_before = game.state.player1.waitroom.cards.len();

    fill_both_main_decks(&mut game, f);
    let blade_before = game.state.mods.get_blade_modifier(c);

    trigger_member_restore_debut_and_check_blade_gain(&mut game, c);

    // Live card was retrieved from discard to hand
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Q242: Live card should be retrieved to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "Q242: Live card should no longer be in discard"
    );
    // P1's waitroom shrank (member cards shuffled under + live card retrieved)
    assert!(
        game.state.player1.waitroom.cards.len() < w1_before,
        "Q242: P1 waitroom should shrink"
    );

    // Blade+2 gained
    let blade_after = game.state.mods.get_blade_modifier(c);
    assert_eq!(
        blade_after,
        blade_before + 2,
        "Q242: Should gain blade+2 from debut (happy path)"
    );
}

#[test]
fn twenty_restored_members_gain_two_blades_without_recoverable_live_q242() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, -1, -1];

    // 10 member cards each = 20 total (meets threshold)
    for _ in 0..10 {
        game.state.player1.waitroom.cards.push(m);
        game.state.player2.waitroom.cards.push(m);
    }
    // NO live card in discard!
    let hand_before = game.state.player1.hand.cards.len();

    fill_both_main_decks(&mut game, f);
    let blade_before = game.state.mods.get_blade_modifier(c);

    trigger_member_restore_debut_and_check_blade_gain(&mut game, c);

    // No live card retrieved (hand unchanged from the retrieve effect)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Q242: Hand should not grow when no live card in discard"
    );

    // Blade+2 STILL gained (Q242: yes, you can)
    let blade_after = game.state.mods.get_blade_modifier(c);
    assert_eq!(
        blade_after,
        blade_before + 2,
        "Q242: Should gain blade+2 EVEN with no live card in discard"
    );
}

#[test]
fn exactly_twenty_restored_members_meet_recovery_and_blade_threshold_q242() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, -1, -1];

    // Exactly 10 each = 20 total
    for _ in 0..10 {
        game.state.player1.waitroom.cards.push(m);
        game.state.player2.waitroom.cards.push(m);
    }
    game.state.player1.waitroom.cards.push(live);
    fill_both_main_decks(&mut game, f);

    trigger_member_restore_debut_and_check_blade_gain(&mut game, c);

    let blade = game.state.mods.get_blade_modifier(c);
    assert!(
        blade >= 2,
        "Q242: Exactly 20 cards moved → threshold met, blade+2 should apply"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Q242: Live card retrieved at exactly 20 threshold"
    );
}

#[test]
fn nineteen_restored_members_grant_neither_recovery_nor_blades_q242() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, -1, -1];

    // 9 P1 + 10 P2 = 19 total (below 20 threshold)
    for _ in 0..9 {
        game.state.player1.waitroom.cards.push(m);
    }
    for _ in 0..10 {
        game.state.player2.waitroom.cards.push(m);
    }
    game.state.player1.waitroom.cards.push(live);
    fill_both_main_decks(&mut game, f);

    trigger_member_restore_debut_and_check_blade_gain(&mut game, c);

    // Blade should NOT be gained
    let blade = game.state.mods.get_blade_modifier(c);
    assert_eq!(blade, 0, "Q242: 19 < 20 threshold → no blade+2");
    // Live card should NOT be retrieved
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "Q242: Live card not retrieved below threshold"
    );
}

#[test]
fn twenty_members_restored_from_own_waitroom_alone_grant_blades_q242() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, -1, -1];

    // 20 cards from P1 only, 0 from P2 = 20 total
    for _ in 0..20 {
        game.state.player1.waitroom.cards.push(m);
    }
    game.state.player1.waitroom.cards.push(live);
    fill_both_main_decks(&mut game, f);

    trigger_member_restore_debut_and_check_blade_gain(&mut game, c);

    let blade = game.state.mods.get_blade_modifier(c);
    assert!(
        blade >= 2,
        "Q242: P1-only 20 cards → threshold met (total=20), blade+2 applies"
    );
}

#[test]
fn q242_real_debut_moves_each_players_members_to_their_own_deck_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let q242 = game.id("PL!HS-pb1-012-R");
    let member_card_numbers = [
        "PL!N-bp1-001-R",
        "PL!N-bp1-002-R\u{ff0b}",
        "PL!N-bp1-003-R\u{ff0b}",
        "PL!N-bp1-004-R",
        "PL!N-bp1-005-R",
        "PL!N-bp1-006-R\u{ff0b}",
        "PL!N-bp1-007-R",
        "PL!N-bp1-008-R",
        "PL!N-bp1-009-R",
        "PL!N-bp1-010-R",
    ];
    let mut p1_members = Vec::new();
    let mut p2_members = Vec::new();
    for card_no in member_card_numbers {
        let p1_member = game.id(card_no);
        let p2_member = game.new_id(card_no);
        game.state.player1.waitroom.cards.push(p1_member);
        game.state.player2.waitroom.cards.push(p2_member);
        p1_members.push(p1_member);
        p2_members.push(p2_member);
    }
    let p1_live = game.id("PL!-sd1-019-SD");
    let p2_live = game.new_id("PL!-sd1-020-SD");
    let p1_nonmember = game.id("LL-E-001-SD");
    let p1_deck_sentinel = game.new_id("PL!-sd1-020-SD");
    let p2_deck_sentinel = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(p1_live);
    game.state.player1.waitroom.cards.push(p1_nonmember);
    game.state.player2.waitroom.cards.push(p2_live);
    game.state.player1.main_deck.cards.push(p1_deck_sentinel);
    game.state.player2.main_deck.cards.push(p2_deck_sentinel);

    game.give_energy(13);
    game.state.player1.hand.cards.push(q242);
    game.play_to_stage(q242, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(game.db.get_card(q242).unwrap().card_no.as_ref(), "PL!HS-pb1-012-R");
    assert_eq!(game.state.player1.stage.stage[1], q242);
    for &card in &p1_members {
        assert!(!game.state.player1.waitroom.cards.contains(&card));
        assert!(game.state.player1.main_deck.cards.contains(&card));
        assert!(!game.state.player2.main_deck.cards.contains(&card));
    }
    for &card in &p2_members {
        assert!(!game.state.player2.waitroom.cards.contains(&card));
        assert!(game.state.player2.main_deck.cards.contains(&card));
        assert!(!game.state.player1.main_deck.cards.contains(&card));
    }
    assert!(game.state.player1.main_deck.cards.contains(&p1_deck_sentinel));
    assert!(game.state.player2.main_deck.cards.contains(&p2_deck_sentinel));
    assert!(game.state.player1.waitroom.cards.contains(&p1_nonmember));
    assert!(!game.state.player1.main_deck.cards.contains(&p1_nonmember));
    assert!(game.state.player1.hand.cards.contains(&p1_live));
    assert!(!game.state.player1.waitroom.cards.contains(&p1_live));
    assert!(game.state.player2.waitroom.cards.contains(&p2_live));
    assert!(!game.state.player1.hand.cards.contains(&p2_live));
    assert_eq!(game.state.mods.get_blade_modifier(q242), 2);
}
