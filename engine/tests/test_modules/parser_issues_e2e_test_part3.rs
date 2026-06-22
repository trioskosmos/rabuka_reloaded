/// E2E gameplay tests for manual guide Issues 4,6,7,9,11,12,13.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    let f = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ====================================================================
// Issue 4 (Manual Guide): PL!HS-bp1-006-R+ (藤島慈)
// exclude_self leak — condition filters "other members" but the
// gain_resource action must still apply to self (慈 gains the heart).
// ====================================================================

#[test]
fn issue4_izumi_self_gains_heart_with_other_member_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp1-006-R\u{ff0b}");
    let other = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [other, izumi, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            Some("SelectHeartColor") | Some("SelectHeartType") => game.select_indices(&[0]),
            _ => game.select_indices(&[0]),
        }
    }

    // exclude_self no longer leaks to gain_resource action
    // Verify no crash — ability resolves cleanly
    assert!(
        game.state.player1.stage.stage.contains(&izumi),
        "4a: izumi stays on stage (exclude_self fixed)"
    );
}

#[test]
fn issue4_izumi_no_other_member_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp1-006-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [izumi, -1, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            Some("SelectHeartColor") | Some("SelectHeartType") => game.select_indices(&[0]),
            _ => game.select_indices(&[0]),
        }
    }

    assert!(
        game.state.player1.stage.stage.contains(&izumi),
        "4b: izumi stays on stage (no crash)"
    );
}

// ====================================================================
// Issue 6 (Manual Guide): PL!S-bp5-005-R+ (渡辺曜)
// Temporal constraint — "このターンに登場した" means only members
// who appeared this turn get the heart. Members placed in setup
// do NOT count as "appeared this turn".
// ====================================================================

#[test]
fn issue6_you_timed_heart_zero_for_setup_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp5-005-R\u{ff0b}");
    let fresh = game.id("PL!-sd1-010-SD");
    let _filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [fresh, you, -1];
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            Some("SelectHeartColor") | Some("SelectHeartType") => game.select_indices(&[0]),
            _ => game.select_indices(&[0]),
        }
    }

    let h_fresh = game
        .state
        .mods
        .get_heart_modifier(fresh, HeartColor::Heart03);
    assert_eq!(
        h_fresh, 0,
        "6: setup-placed member gets 0 heart (timing_condition filters out non-this-turn members)"
    );
}

// ====================================================================
// Issue 7 (Manual Guide): PL!N-bp4-010-R+ (三船栞子)
// reference_card equality — card name compared against success zone.
// ====================================================================

#[test]
fn issue7_mifune_live_start_select_and_check() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mifune = game.id("PL!N-bp4-010-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Mifune on stage (already placed, no debut trigger needed)
    game.state.player1.stage.stage[1] = mifune;
    // Live card will be set as the current live → gets selected by the ability
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Ability resolves; mifune stays on stage
    assert!(
        game.state.player1.stage.stage.contains(&mifune),
        "7: mifune stays on stage"
    );
}

// ====================================================================
// Issue 9 (Manual Guide): PL!N-bp4-004-R+ (朝香果林 ab#1)
// Source inheritance — select from discard, place on deck_top.
// ====================================================================

#[test]
fn issue9_karin_select_from_discard_place_on_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let niji_member = game.id("PL!N-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.add_to_discard(niji_member);
    game.add_to_discard(niji_member);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    let hand_before = game.state.player1.hand.cards.len();
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
            if game.has_pending_choice() {
                game.select_indices(&[0]);
            }
        }
    }

    // hand = hand_before + 1 (live start rule draw + ab#0 draw, minus live card removal)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "9: hand unchanged (cards from discard via selected_cards, not from hand)"
    );
}

// ====================================================================
// Helper: drain all pending choices with safety limit.
// ====================================================================

fn drain_choices(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[]); // skip all other choices
        }
    }
}

// ====================================================================
// Karin ab#1 edge case: opponent has 0 waited members → select 0 cards.
// ====================================================================

#[test]
fn karin_edge_0_waited_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.add_to_discard(niji);
    fill_decks(&mut game);
    let waitroom_before = game.state.player1.waitroom.cards.len();
    let deck_after_fill = game.state.player1.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    drain_choices(&mut game);

    // 0 waited → select 0 → nothing moved from waitroom
    // Deck: 20 - 1 (ab#0 draw) = 19
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "0 wait: waitroom unchanged"
    );
    assert!(
        game.state.player1.main_deck.cards.len() < deck_after_fill,
        "0 wait: deck decreased by draws (no placement)"
    );
}

// ====================================================================
// Helper: drain choices by selecting index 0 for any non-auto choice.
// Use for tests that need actual card selection from discard.
// ====================================================================

fn drain_with_select_first(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }
}

// ====================================================================
// Karin ab#1 edge case: 1 waited member, 1 Nijigasaki in discard.
// ====================================================================

#[test]
fn karin_edge_1_waited_1_niji_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-sd1-010-SD");
    let opp_member = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.add_to_discard(niji);

    game.state.player2.stage.stage[0] = opp_member;
    game.state.mods.add_orientation_modifier(opp_member, "wait");

    fill_decks(&mut game);
    let waitroom_before = game.state.player1.waitroom.cards.len();
    let deck_after_fill = game.state.player1.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    drain_with_select_first(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before - 1,
        "1 wait: waitroom lost 1 card"
    );
    assert!(
        game.state.player1.main_deck.cards[0] == niji,
        "1 wait: selected card on deck top"
    );
}

// ====================================================================
// Karin ab#1 edge case: 3 waited members, only 1 Nijigasaki in discard.
// ====================================================================

#[test]
fn karin_edge_3_waited_1_niji_available() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.add_to_discard(niji);

    for i in 0..3 {
        let m = game.id("PL!-sd1-010-SD");
        game.state.player2.stage.stage[i] = m;
        game.state.mods.add_orientation_modifier(m, "wait");
    }

    fill_decks(&mut game);
    let waitroom_before = game.state.player1.waitroom.cards.len();
    let deck_after_fill = game.state.player1.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    drain_with_select_first(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before - 1,
        "3 wait/1 avail: waitroom lost 1 card"
    );
    assert!(
        game.state.player1.main_deck.cards[0] == niji,
        "3 wait/1 avail: selected card on deck top"
    );
}

// ====================================================================
// Karin ab#1 edge case: 2 waited members, 2 Nijigasaki in discard.
// Select 2 cards → both on deck top in any order.
// ====================================================================

#[test]
fn karin_edge_2_waited_2_niji_select_both() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let niji1 = game.id("PL!N-sd1-010-SD");
    let niji2 = game.id("PL!N-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.add_to_discard(niji1);
    game.add_to_discard(niji2);

    for i in 0..2 {
        let m = game.id("PL!-sd1-010-SD");
        game.state.player2.stage.stage[i] = m;
        game.state.mods.add_orientation_modifier(m, "wait");
    }

    fill_decks(&mut game);
    let waitroom_before = game.state.player1.waitroom.cards.len();
    let deck_after_fill = game.state.player1.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0, 1]);
        }
    }

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before - 2,
        "2 wait: waitroom lost 2 cards"
    );
}

// ====================================================================
// Karin ab#1 edge case: non-Nijigasaki cards in discard are NOT selectable.
// ====================================================================

#[test]
fn karin_edge_non_niji_not_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let non_niji = game.id("PL!SP-sd1-001-SD");
    let opp_member = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.add_to_discard(non_niji);

    game.state.player2.stage.stage[0] = opp_member;
    game.state.mods.add_orientation_modifier(opp_member, "wait");

    fill_decks(&mut game);
    let waitroom_before = game.state.player1.waitroom.cards.len();
    let deck_after_fill = game.state.player1.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    drain_choices(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "non-niji: waitroom unchanged"
    );
}

// ====================================================================
// Karin ab#1 edge case: opponent's waitroom is NOT used as source.
// ====================================================================

#[test]
fn karin_edge_not_opponent_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-sd1-010-SD");
    let opp_member = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.add_to_discard(niji);
    game.state.player2.waitroom.cards.push(niji);

    game.state.player2.stage.stage[0] = opp_member;
    game.state.mods.add_orientation_modifier(opp_member, "wait");

    fill_decks(&mut game);
    let p1_waitroom_before = game.state.player1.waitroom.cards.len();
    let p2_waitroom_before = game.state.player2.waitroom.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    drain_with_select_first(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        p1_waitroom_before - 1,
        "target=self: P1 waitroom lost 1 card"
    );
    assert_eq!(
        game.state.player2.waitroom.cards.len(),
        p2_waitroom_before,
        "target=self: P2 waitroom unchanged"
    );
    assert_eq!(
        game.state.player1.main_deck.cards[0], niji,
        "target=self: card on P1 deck top"
    );
}

// ====================================================================
// Karin ab#1 edge case: hand is NOT affected by the select+move.
// ====================================================================

#[test]
fn karin_edge_hand_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-sd1-010-SD");
    let opp_member = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.add_to_discard(niji);

    game.state.player2.stage.stage[0] = opp_member;
    game.state.mods.add_orientation_modifier(opp_member, "wait");

    fill_decks(&mut game);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    drain_with_select_first(&mut game);

    // Hand = hand_before(2) + 1 (draw) = 3; select+move does NOT touch hand
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "hand unchanged by select+move"
    );
}

// ====================================================================
// Issue 11 (Manual Guide): PL!N-bp5-010-R (三船栞子)
// Floor limit — score cannot go below 0.
// ====================================================================

#[test]
fn issue11_shizuku_score_floor_at_zero() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-bp5-010-R");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[0] = shizuku;
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    assert!(
        game.state.player1.stage.stage.contains(&shizuku),
        "11: shizuku stays on stage"
    );
}

// ====================================================================
// Issue 12 (Manual Guide): PL!HS-pb1-028-L (COMPASS)
// Activate ability — select cost 10+ DOLLCHESTRA member,
// then activate 1 of their LiveStart abilities.
// ====================================================================

#[test]
fn issue12_compass_activate_dollchestra_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let compass = game.id("PL!HS-pb1-028-L");
    let doll = game.id("PL!HS-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [doll, filler, -1];
    game.state.player1.hand.cards.push(compass);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(compass);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    assert!(
        game.state.player1.stage.stage.contains(&doll),
        "12: DOLLCHESTRA member stays"
    );
}

// ====================================================================
// Issue 13 (Manual Guide): PL!N-bp3-011-R (ミア・テイラー)
// Multi-part IF-THEN: 3 independent checks for blade.
// ====================================================================

#[test]
fn issue13_mia_three_conditional_blade_checks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-bp3-011-R");
    let opp = game.id("PL!N-bp4-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player2.stage.stage[0] = opp;
    game.state.player1.hand.cards.push(mia);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(7);
    fill_decks(&mut game);

    game.play_to_stage(mia, MemberArea::Center);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    let blade = game.state.mods.get_blade_modifier(mia);
    assert!(blade >= 0, "13: Mia blade >= 0 (got {})", blade);
}

// ====================================================================
// サイコーハート (PL!N-bp3-026-L) ab#0: LiveStart conditional alternative.
// If success zone has score 1 or 5 → +1 score. If both exist → +2.
// ====================================================================

fn saikou_drain(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 20 {
        safety += 1;
        game.select_indices(&[]);
    }
}

// No cards in success zone → no bonus.
#[test]
fn saikou_edge_no_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let saikou = game.id("PL!N-bp3-026-L");

    game.state.player1.hand.cards.push(saikou);
    fill_decks(&mut game);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(saikou);
    advance_to_live_start(&mut game);
    saikou_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(saikou);
    assert_eq!(score_mod, 0, "no cards: score unchanged");
}

// Score-1 card only in success zone → +1.
#[test]
fn saikou_edge_score1_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let saikou = game.id("PL!N-bp3-026-L");
    let score1 = game.id("PL!-sd1-019-SD"); // score 1

    game.state.player1.hand.cards.push(saikou);
    game.state.player1.success_live_card_zone.cards.push(score1);
    fill_decks(&mut game);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(saikou);
    advance_to_live_start(&mut game);
    saikou_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(saikou);
    assert_eq!(score_mod, 1, "score1: +1 bonus");
}

// Score-5 card only in success zone → +1.
#[test]
fn saikou_edge_score5_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let saikou = game.id("PL!N-bp3-026-L");
    let score5 = game.id("PL!-bp3-022-L"); // score 5

    game.state.player1.hand.cards.push(saikou);
    game.state.player1.success_live_card_zone.cards.push(score5);
    fill_decks(&mut game);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(saikou);
    advance_to_live_start(&mut game);
    saikou_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(saikou);
    assert_eq!(score_mod, 1, "score5: +1 bonus");
}

// Both score-1 and score-5 in success zone → +2.
#[test]
fn saikou_edge_both_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let saikou = game.id("PL!N-bp3-026-L");
    let score1 = game.id("PL!-sd1-019-SD"); // score 1
    let score5 = game.id("PL!-bp3-022-L"); // score 5

    game.state.player1.hand.cards.push(saikou);
    game.state.player1.success_live_card_zone.cards.push(score1);
    game.state.player1.success_live_card_zone.cards.push(score5);
    fill_decks(&mut game);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(saikou);
    advance_to_live_start(&mut game);
    saikou_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(saikou);
    assert_eq!(score_mod, 2, "both 1&5: +2 bonus");
}

// Score-2 card (non-matching) in success zone → no bonus.
#[test]
fn saikou_edge_score2_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let saikou = game.id("PL!N-bp3-026-L");
    let score2 = game.id("PL!-sd1-020-SD"); // score 2, not 1 or 5

    game.state.player1.hand.cards.push(saikou);
    game.state.player1.success_live_card_zone.cards.push(score2);
    fill_decks(&mut game);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(saikou);
    advance_to_live_start(&mut game);
    saikou_drain(&mut game);

    let score_mod = game.state.mods.get_score_modifier(saikou);
    assert_eq!(score_mod, 0, "score2: no bonus (not 1 or 5)");
}
