/// Comprehensive tests for live success/failure mechanics (Rules §8.3, §8.4, QA entries)
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
fn advance_to_live_victory(game: &mut TestGame) {
    for _ in 0..3 {
        game.pass();
    }
}

/// Insufficient hearts -> live fails, no LiveSuccess (Rules 8.3.16, Q35).
#[test]
fn live_fails_with_insufficient_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    advance_to_live_victory(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    // One more pass finalizes victory determination (winner placement).
    game.pass();

    // Q35: the failed live card must end in the WAITROOM — never the
    // success zone. A bare "!has_pending_choice()" cannot distinguish this
    // from any other terminal state.
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "Q35: failed live card goes to the waitroom"
    );
    assert!(
        game.state.player1.success_live_card_zone.cards.is_empty(),
        "Q35: insufficient hearts → nothing enters the success zone"
    );
}

/// Both players have lives -> both scores compared, each may have LiveSuccess.
#[test]
fn both_players_live_score_compared() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_live = game.id("PL!-sd1-019-SD");
    let p2_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player2.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(p1_live);
    game.state.player2.hand.cards.push(p2_live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(p1_live);
    advance_to_live_start(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    advance_to_live_victory(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    // One more pass to finalize LiveVictoryDetermination (move winner to success zone)
    game.pass();
    let p1_success = game.state.player1.success_live_card_zone.cards.len();
    let p2_success = game.state.player2.success_live_card_zone.cards.len();
    // Exactly one player wins (identical live cards, same score → P1 wins as first attacker)
    assert_eq!(
        p1_success, 1,
        "P1 (first attacker) wins with score-1 live; P1 success zone has 1 card"
    );
    assert_eq!(
        p2_success, 0,
        "P2 loses: success zone should be empty (P2 live card stays in live zone)"
    );
}

/// Edge: score-0 live succeeds when hearts met (Q147).
#[test]
fn q147_empty_need_heart_live_succeeds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    advance_to_live_victory(&mut game);
    // Drain the live's own ライブ成功時 ability chain (look_and_select etc.)
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    // One more pass finalizes winner placement.
    game.pass();
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.as_slice(),
        &[live][..],
        "Q147: score-0 live placed on success"
    );
}

/// Both players have lives -> both scores compared, each may have LiveSuccess.
#[test]
fn both_players_have_live_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_live = game.id("PL!-sd1-019-SD");
    let p2_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player2.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(p1_live);
    game.state.player2.hand.cards.push(p2_live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(p1_live);
    // Transition to SecondAttacker so P2 can set theirs too — without this,
    // P2 never performs and the scenario is a walkover, not a tie.
    game.pass();
    game.set_live_card(p2_live);
    advance_to_live_victory(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    advance_to_live_victory(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    game.pass();
    // Rule 8.4.6.2: equal scores → BOTH win and BOTH place (neither has
    // ≥2 cards in their success zone yet). An OR here would mask one side
    // silently failing to place.
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.as_slice(),
        &[p1_live][..],
        "tie: P1 places their winning live"
    );
    assert_eq!(
        game.state.player2.success_live_card_zone.cards.as_slice(),
        &[p2_live][..],
        "tie: P2 also wins and places"
    );
}

/// P1 has live card, P2 doesn't -> P1's score is automatically higher (8.4.3.2, Q47)
#[test]
fn one_player_live_auto_higher_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    advance_to_live_victory(&mut game);
    assert!(
        game.has_pending_choice(),
        "Q47: P1 with live card succeeds even if P2 has none"
    );
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    // One more pass finalizes winner placement.
    game.pass();

    // Q47 walkover: P2 never performed, so P1 places unopposed.
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.as_slice(),
        &[live][..],
        "Q47: P1's live reaches the success zone"
    );
    assert!(
        game.state.player2.success_live_card_zone.cards.is_empty()
            && game.state.player2.live_card_zone.cards.is_empty(),
        "Q47: P2 has nothing in play"
    );
}

/// No live card set -> no yell, no LiveSuccess (Q32).
#[test]
fn no_live_card_no_yell_no_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.clear();
    game.state.player2.hand.cards.clear();
    for _ in 0..50 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    for _ in 0..20 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    advance_to_live_victory(&mut game);
    assert!(
        game.state.player1.main_deck.cards.len() >= 45,
        "Q32: Deck should not lose >5 cards from yell (no live card set)"
    );
    assert!(
        game.state.player1.success_live_card_zone.cards.is_empty()
            && game.state.player2.success_live_card_zone.cards.is_empty(),
        "Q32: no live performed → nobody places"
    );
}

/// Multiple live cards: if any fails hearts, all fail (8.3.16, Q35).
/// BOTH cards are in the zone; the shared heart pool cannot satisfy both.
#[test]
fn any_card_fails_hearts_all_fail() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(live_a);
    game.state.player1.hand.cards.push(live_b);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_a);
    // Second live card joins the zone (same as having set 2 cards).
    game.state.player1.live_card_zone.cards.push(live_b);
    advance_to_live_start(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    advance_to_live_victory(&mut game);
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    game.pass();

    // 8.3.16: one failure fails ALL — nothing places, both cards to waitroom.
    assert!(
        game.state.player1.success_live_card_zone.cards.is_empty(),
        "Q35: no placement when any live card failed"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a)
            && game.state.player1.waitroom.cards.contains(&live_b),
        "Q35: ALL live cards go to the waitroom"
    );
}

/// Winner takes at most 1 card from live zone to success zone (8.4.7, Q83)
fn assert_two_live_placement(selected: usize) {
    let mut game = TestGame::new(load_real_database());
    let deck: Vec<i16> = (0..30).map(|_| game.id("PL!-sd1-010-SD")).collect();
    let opponent_deck: Vec<i16> = (0..10).map(|_| game.id("PL!-sd1-010-SD")).collect();
    game.state.player1.main_deck.cards = deck.into();
    game.state.player2.main_deck.cards = opponent_deck.into();
    let rin = game.id("PL!-sd1-014-SD");
    let hanayo = game.id("PL!-sd1-017-SD");
    let lives = [game.id("PL!HS-bp1-019-L"), game.id("PL!HS-bp1-019-L")];
    for card in [rin, hanayo, lives[0], lives[1]] {
        game.add_to_hand(card);
    }
    game.give_energy(18);
    game.play_to_stage(rin, rabuka_engine::zones::MemberArea::Center);
    game.play_to_stage(hanayo, rabuka_engine::zones::MemberArea::LeftSide);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(lives[0]);
    game.set_live_card(lives[1]);
    assert_eq!(game.state.player1.live_card_zone.cards.as_slice(), &lives);
    assert!(game.state.player1.hand.cards.is_empty());
    advance_to_live_start(&mut game);
    let deck_before = game.state.player1.main_deck.cards.to_vec();
    let hand_before = game.state.player1.hand.cards.clone();
    for _ in 0..3 {
        assert!(!game.has_pending_choice());
        game.pass();
    }
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectLiveSuccess"));
    assert_eq!(game.state.player1.live_card_zone.cards.as_slice(), &lives);
    game.select_indices(&[selected]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.success_live_card_zone.cards.as_slice(), &[lives[selected]]);
    assert!(game.state.player1.live_card_zone.cards.is_empty());
    assert_eq!(game.state.player1.hand.cards, hand_before);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &deck_before[6..]);
    let mut expected_waitroom = deck_before[..6].to_vec();
    expected_waitroom.push(lives[1 - selected]);
    let mut actual_waitroom = game.state.player1.waitroom.cards.to_vec();
    expected_waitroom.sort_unstable();
    actual_waitroom.sort_unstable();
    assert_eq!(actual_waitroom, expected_waitroom);
    assert!(game.state.player2.success_live_card_zone.cards.is_empty());
    assert!(game.state.player2.waitroom.cards.is_empty());
}

#[test]
fn winner_takes_one_to_success_zone() {
    assert_two_live_placement(0);
}

/// Two live cards, choose the SECOND one for success zone.
#[test]
fn two_live_cards_choose_second() {
    assert_two_live_placement(1);
}

/// Daydream Mermaid (PL!N-bp4-030-L): Live success — conditional_alternative choice.
/// No nijigasaki in success zone → pick 1 option, then end.
#[test]
fn daydream_mermaid_no_niji_in_success_pick_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!N-bp4-030-L");
    // Need heart05=1, heart06=3, heart0=4
    // PL!S-bp2-015-PR: heart04=1, heart05=1
    // PL!-sd1-003-SD: heart01=1, heart03=2, heart06=2
    // PL!-sd1-001-SD: heart01=1, heart03=2, heart06=1
    let h05 = game.id("PL!S-bp2-015-PR");
    let h06a = game.id("PL!-sd1-003-SD");
    let h06b = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [h05, h06a, h06b];
    game.state.player1.hand.cards.push(live);

    // Put a member in waitroom for the "recover" option
    let recover_target = game.new_id("PL!-sd1-001-SD");
    game.state.player1.waitroom.cards.push(recover_target);

    // Energy deck for the "place energy" option
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(h05);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(h05);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.state.player1.live_card_zone.cards.push(live);
    game.pass();
    game.pass();
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);

    game.pass();
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        let t = game.pending_choice_type();
        if t.as_deref() == Some("SelectTarget") {
            break;
        }
        game.select_indices(&[]);
    }

    assert!(
        game.has_pending_choice(),
        "Should have option pick (no niji)"
    );

    game.select_option(1); // Pick option 1 (recover)

    // Sub-choice: pick the recover target from the waiting room.
    assert!(game.has_pending_choice(), "recover target selection expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard"
    );
    game.select_indices(&[0]);

    assert!(
        !game.has_pending_choice(),
        "No re-prompt after 1 pick (count=1)"
    );
    assert!(
        game.state.player1.hand.cards.contains(&recover_target),
        "Recovered member in hand"
    );
}

#[test]
fn daydream_mermaid_q191_niji_in_success_pick_both() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!N-bp4-030-L");
    let h05 = game.id("PL!S-bp2-015-PR");
    let h06a = game.id("PL!-sd1-003-SD");
    let h06b = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [h05, h06a, h06b];

    // Put a nijigasaki card in success zone so condition is met
    let niji_live = game.id("PL!N-bp1-025-L");
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(niji_live);

    let recover_target = game.new_id("PL!-sd1-001-SD");
    game.state.player1.waitroom.cards.push(recover_target);

    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(h05);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(h05);
    }

    for _ in 0..5 {
        game.pass();
    }
    // Manually set the live card
    game.state.player1.live_card_zone.cards.push(live);
    game.pass();
    game.pass();
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);

    game.pass();
    game.pass();
    game.pass();

    // Niji in success zone → choice should use any_number re-prompt
    while game.has_pending_choice() {
        let t = game.pending_choice_type();
        if t.as_deref() == Some("SelectTarget") {
            break;
        }
        game.select_indices(&[]);
    }

    assert!(
        game.has_pending_choice(),
        "Should have option pick (niji in success)"
    );
    game.select_option(1); // Pick option 1 (recover)

    // Sub-choice: pick a card from waitroom ("Select 1 card(s) from the waiting room")
    assert!(game.has_pending_choice(), "recover target selection expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard"
    );
    game.select_indices(&[0]);

    // any_number re-prompt: should have remaining option (energy)
    assert!(
        game.has_pending_choice(),
        "Re-prompt after picking 1 option with any_number"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget"
    );
    game.select_option(0); // Pick the remaining option (energy)

    // Observed: the energy option moves the top energy_deck card to the energy
    // zone directly — no card-selection prompt follows the second option pick.
    assert!(
        !game.has_pending_choice(),
        "No re-prompt after all options consumed"
    );

    assert!(
        game.state.player1.hand.cards.contains(&recover_target),
        "Recovered member is in hand"
    );
    assert!(
        game.state.player1.energy_zone.active_count() > 0
            || !game.state.player1.energy_zone.cards.is_empty(),
        "Energy card was placed"
    );
}

fn assert_shared_heart_pool(live_count: usize) {
    let mut game = TestGame::new(load_real_database());
    let deck: Vec<i16> = (0..20).map(|_| game.id("PL!-sd1-014-SD")).collect();
    let opponent_deck: Vec<i16> = (0..10).map(|_| game.id("PL!-sd1-014-SD")).collect();
    game.state.player1.main_deck.cards = deck.into();
    game.state.player2.main_deck.cards = opponent_deck.into();
    let energy = game.id("LL-E-001-SD");
    game.state.player2.energy_deck.cards.push(energy);
    let rin = game.id("PL!-sd1-014-SD");
    let lives: Vec<i16> = (0..live_count).map(|_| game.id("PL!HS-bp1-019-L")).collect();
    game.add_to_hand(rin);
    for &live in &lives {
        game.add_to_hand(live);
    }
    game.give_energy(9);
    game.play_to_stage(rin, rabuka_engine::zones::MemberArea::Center);
    advance_to_live_card_set_p1(&mut game);
    for &live in &lives {
        game.set_live_card(live);
    }
    assert_eq!(game.state.player1.live_card_zone.cards.as_slice(), lives.as_slice());
    assert!(game.state.player1.hand.cards.is_empty());
    advance_to_live_start(&mut game);
    let deck_before = game.state.player1.main_deck.cards.to_vec();
    let hand_before = game.state.player1.hand.cards.clone();
    let opponent_deck_before = game.state.player2.main_deck.cards.clone();
    let opponent_hand_before = game.state.player2.hand.cards.clone();
    for _ in 0..4 {
        assert!(!game.has_pending_choice());
        game.pass();
    }
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.live_card_zone.cards.is_empty());
    assert_eq!(game.state.player1.hand.cards, hand_before);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &deck_before[3..]);
    let mut expected_waitroom = deck_before[..3].to_vec();
    if live_count == 1 {
        assert_eq!(game.state.player1.success_live_card_zone.cards.as_slice(), lives.as_slice());
    } else {
        assert!(game.state.player1.success_live_card_zone.cards.is_empty());
        expected_waitroom.extend_from_slice(&lives);
    }
    let mut actual_waitroom = game.state.player1.waitroom.cards.to_vec();
    actual_waitroom.sort_unstable();
    expected_waitroom.sort_unstable();
    assert_eq!(actual_waitroom, expected_waitroom);
    assert_eq!(game.state.player2.main_deck.cards, opponent_deck_before);
    assert_eq!(game.state.player2.hand.cards, opponent_hand_before);
    assert!(game.state.player2.waitroom.cards.is_empty());
    assert!(game.state.player2.success_live_card_zone.cards.is_empty());
}

#[test]
fn shared_pool_four_hearts_satisfy_one_live() {
    assert_shared_heart_pool(1);
}

#[test]
fn shared_pool_depletion_all_fail() {
    assert_shared_heart_pool(2);
}

#[test]
fn three_live_cards_any_fails_all_fail() {
    assert_shared_heart_pool(3);
}
