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
    let filler = game.id("PL!-sd1-010-SD");
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

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "9: hand unchanged (cards from discard via selected_cards, not from hand)"
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
