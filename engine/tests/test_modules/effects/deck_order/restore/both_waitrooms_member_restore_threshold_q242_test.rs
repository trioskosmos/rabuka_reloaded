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

/// 百生吟子 ab#0, both halves of 「自身と相手はそれぞれ」 with the threshold NOT met.
///
/// Was `assert!(waitroom.len() < w1)` — a count, so it passed for the wrong
/// reasons: a card leaving the discard into the hand, a single card moving
/// twice, or the OPPONENT's members moving to the wrong deck all satisfy it.
/// The printed effect is directional and positional, so both are pinned:
///
/// - each player's MEMBERS leave THEIR OWN waitroom for THE BOTTOM of THEIR
///   OWN deck — checked card by card, in both directions, with a sentinel
///   proving the destination is the bottom and not just "the deck";
/// - 3 + 3 = 6 is below the 20-card threshold, so neither the live retrieval
///   nor the blade+2 half fires.
#[test]
fn debut_both_waitrooms_member_restore_shrinks_own_waitroom() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!HS-pb1-012-R");
    g.assert_card_identity(c, "PL!HS-pb1-012-R");
    // Distinct prints, not five copies of one: the claim is about WHICH cards
    // moved, and one id in five slots cannot say that.
    let prints = [
        "PL!N-bp1-001-R",
        "PL!N-bp1-002-R\u{ff0b}",
        "PL!N-bp1-003-R\u{ff0b}",
    ];
    let mut p1_members = Vec::new();
    let mut p2_members = Vec::new();
    for card_no in prints {
        let p1_card = g.id(card_no);
        let p2_card = g.new_id(card_no);
        g.assert_card_type(p1_card, "member_card", "the shuffled cards are members");
        g.assert_card_type(p2_card, "member_card", "the shuffled cards are members");
        g.state.player1.waitroom.cards.push(p1_card);
        g.state.player2.waitroom.cards.push(p2_card);
        p1_members.push(p1_card);
        p2_members.push(p2_card);
    }
    // A live in P1's waitroom: it is not a member, so the shuffle must ignore
    // it, and the retrieval half must still not fire (6 < 20).
    let live = g.id("PL!-sd1-019-SD");
    g.state.player1.waitroom.cards.push(live);
    // Sentinels already in each deck, so "moved to the BOTTOM" is observable.
    let p1_sentinel = g.id("PL!-sd1-010-SD");
    let p2_sentinel = g.new_id("PL!-sd1-010-SD");
    g.state.player1.main_deck.cards.push(p1_sentinel);
    g.state.player2.main_deck.cards.push(p2_sentinel);

    let p1_waitroom_before = g.state.player1.waitroom.cards.len();
    let p2_waitroom_before = g.state.player2.waitroom.cards.len();
    play_q242_debut(&mut g, c);

    assert_eq!(
        g.state.player1.waitroom.cards.len(),
        p1_waitroom_before - p1_members.len(),
        "P1 loses exactly its own 3 members (the live stays: it is not a member)"
    );
    assert_eq!(
        g.state.player2.waitroom.cards.len(),
        p2_waitroom_before - p2_members.len(),
        "P2 loses exactly its own 3 members"
    );
    for &card in &p1_members {
        assert!(
            g.state.player1.main_deck.cards.contains(&card),
            "a P1 member must reach P1's deck"
        );
        assert!(
            !g.state.player2.main_deck.cards.contains(&card),
            "a P1 member must NOT cross into P2's deck"
        );
    }
    for &card in &p2_members {
        assert!(
            g.state.player2.main_deck.cards.contains(&card),
            "a P2 member must reach P2's deck"
        );
        assert!(
            !g.state.player1.main_deck.cards.contains(&card),
            "a P2 member must NOT cross into P1's deck"
        );
    }
    // 「自身のデッキの下に置く」 — under, not merely inside. The sentinel was
    // already in the deck, so it must end up ABOVE every shuffled member.
    let p1_deck = &g.state.player1.main_deck.cards;
    let sentinel_at = p1_deck
        .iter()
        .position(|&id| id == p1_sentinel)
        .expect("the P1 sentinel must still be in P1's deck");
    let lowest_member = p1_members
        .iter()
        .filter_map(|&id| p1_deck.iter().position(|&x| x == id))
        .min()
        .expect("P1 members must be in P1's deck");
    assert!(
        sentinel_at < lowest_member,
        "members are shuffled UNDER the existing deck: sentinel at {sentinel_at}, \
         lowest member at {lowest_member} of {} cards",
        p1_deck.len()
    );
    // Below the 20-card threshold, so the second half of the sentence does not
    // fire — this fixture is the only place the sub-threshold case is pinned.
    assert!(
        !g.state.player1.hand.cards.contains(&live),
        "6 cards total is below the 20 threshold, so no live is retrieved"
    );
    assert_eq!(
        g.state.mods.get_blade_modifier(c),
        0,
        "6 cards total is below the 20 threshold, so no blade+2"
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
fn p_plus_twenty_restored_members_gain_two_blades_without_recoverable_live_q242() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-pb1-012-P＋");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, -1, -1];

    for _ in 0..10 {
        game.state.player1.waitroom.cards.push(member);
        game.state.player2.waitroom.cards.push(member);
    }

    let hand_before = game.state.player1.hand.cards.len();
    fill_both_main_decks(&mut game, filler);
    let blade_before = game.state.mods.get_blade_modifier(card);

    trigger_member_restore_debut_and_check_blade_gain(&mut game, card);

    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        blade_before + 2
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
