/// Tests for PL!HS-bp2-007-R+/P/P+/SEC (百生 吟子) ab#1 — LiveStart:
///   手札を1枚控え室に置いてもよい：これにより控え室に置いたカードが
///   メンバーカードの場合、控え室に置いたカードと同じ名前を持つメンバー1人は、
///   ライブ終了時まで、heart04 + ブレードを得る。
///
/// Parsed ability:
///   trigger: ライブ開始時 (LiveStart)
///   cost: optional move_cards(hand → discard, count=1)
///   condition: location_condition(card_type=member_card, target=self, location=discard)
///   effect: sequential [gain_resource(blade,1,live_end), gain_resource(heart,heart04,1,live_end)]
///           with target_count=1, same_name=true
///
/// heart04 = +1 heart of color heart04 (the 4th color = yellow).
/// The "04" is the color index, NOT the count.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
}

fn get_heart04(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart04)
}

fn get_blade(game: &TestGame, card_id: i16) -> i32 {
    game.state.mods.get_blade_modifier(card_id)
}

fn get_heart_modifier(game: &TestGame, card_id: i16, color: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card_id, color)
}

// =========================================================================
// 1. Happy path: discard member → same-name member gains heart04 + blade
// =========================================================================
#[test]
fn discard_member_gives_heart04_and_blade_to_same_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(game.state.player1.waitroom.cards.contains(&nahone_hand));
    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 2. Skip cost: choosing not to discard → no effect
// =========================================================================
#[test]
fn skip_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    let _ = game.try_select_indices(&[]);

    assert!(
        game.state.player1.hand.cards.contains(&nahone_hand),
        "Card should remain in hand when cost is skipped"
    );
    assert_eq!(get_heart04(&game, nahone_stage), 0);
    assert_eq!(get_blade(&game, nahone_stage), 0);
}

// =========================================================================
// 3. Discard non-member → condition fails, no buff
// =========================================================================
#[test]
fn discard_non_member_no_buff() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let live_discard = game.new_id("PL!-sd1-020-SD");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live_discard);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(game.state.player1.waitroom.cards.contains(&live_discard));
    assert_eq!(get_heart04(&game, nahone_stage), 0);
    assert_eq!(get_blade(&game, nahone_stage), 0);
}

// =========================================================================
// 4. Empty hand → ability auto-skips
// =========================================================================
#[test]
fn empty_hand_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(!game.has_pending_choice());
    assert_eq!(get_heart04(&game, nahone_stage), 0);
    assert_eq!(get_blade(&game, nahone_stage), 0);
}

// =========================================================================
// 5. Discard member but no same-name on stage → the only same-name is the
//    activating card itself (on stage), so it receives the buff.
//    This verifies same_name filter uses the discarded card's name, not the
//    activating card's name.
// =========================================================================
#[test]
fn discard_member_different_name_than_activating() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let other_member = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(other_member);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(game.state.player1.waitroom.cards.contains(&other_member));
    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 6. Activating card itself is same-name → it receives the buff
// =========================================================================
#[test]
fn discard_same_name_as_activating_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 7. Multiple same-name → only 1 gets buffed
// =========================================================================
#[test]
fn multiple_same_name_only_one_buffed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone1 = game.id("PL!HS-bp2-007-R+");
    let nahone2 = game.new_id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [nahone1, nahone2, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let heart1 = get_heart04(&game, nahone1);
    let heart2 = get_heart04(&game, nahone2);
    assert_eq!(
        heart1 + heart2,
        1,
        "Exactly 1 member should get +1 heart04 (total {}+{}={})",
        heart1,
        heart2,
        heart1 + heart2
    );

    let blade1 = get_blade(&game, nahone1);
    let blade2 = get_blade(&game, nahone2);
    assert!(
        (blade1 >= 1 && blade2 == 0) || (blade1 == 0 && blade2 >= 1),
        "Exactly 1 member should get blade: blade1={} blade2={}",
        blade1,
        blade2
    );
}

// =========================================================================
// 8. Discarded card ends up in waitroom
// =========================================================================
#[test]
fn discarded_card_goes_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(!game.state.player1.hand.cards.contains(&nahone_hand));
    assert!(game.state.player1.waitroom.cards.contains(&nahone_hand));
}

// =========================================================================
// 9. heart04 only, no other colors
// =========================================================================
#[test]
fn heart04_only_no_other_colors() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert_eq!(
        get_heart_modifier(&game, nahone_stage, HeartColor::Heart00),
        0
    );
    assert_eq!(
        get_heart_modifier(&game, nahone_stage, HeartColor::Heart01),
        0
    );
    assert_eq!(
        get_heart_modifier(&game, nahone_stage, HeartColor::Heart02),
        0
    );
    assert_eq!(
        get_heart_modifier(&game, nahone_stage, HeartColor::Heart03),
        0
    );
    assert_eq!(
        get_heart_modifier(&game, nahone_stage, HeartColor::Heart05),
        0
    );
    assert_eq!(
        get_heart_modifier(&game, nahone_stage, HeartColor::Heart06),
        0
    );
}

// =========================================================================
// 10. P variant works
// =========================================================================
#[test]
fn p_variant_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-P");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 11. P+ variant works
// =========================================================================
#[test]
fn p_plus_variant_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-P＋");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 12. SEC variant works
// =========================================================================
#[test]
fn sec_variant_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-SEC");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 13. Discard from non-contiguous hand position
// =========================================================================
#[test]
fn discard_from_non_contiguous_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(nahone_hand);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[1]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(game.state.player1.waitroom.cards.contains(&nahone_hand));
    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 14. Only live card in hand → auto-skip
// =========================================================================
#[test]
fn only_live_card_in_hand_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(!game.has_pending_choice());
    assert_eq!(get_heart04(&game, nahone_stage), 0);
    assert_eq!(get_blade(&game, nahone_stage), 0);
}

// =========================================================================
// 15. Nahone at left position works
// =========================================================================
#[test]
fn nahone_at_left_position() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [nahone_stage, -1, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 16. Nahone at right position works
// =========================================================================
#[test]
fn nahone_at_right_position() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, -1, nahone_stage];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}

// =========================================================================
// 17. Mixed hand: discard member, filler stays in hand
// =========================================================================
#[test]
fn hand_mixed_cards_discard_member_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-020-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[1]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(game.state.player1.waitroom.cards.contains(&nahone_hand));
    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
    assert!(
        game.state.player1.hand.cards.contains(&filler),
        "Non-discarded card should remain in hand"
    );
}

// =========================================================================
// 18. Discard different-name member → condition passes but no same-name target
// =========================================================================
#[test]
fn discard_different_name_member_no_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let other_member = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(other_member);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(game.state.player1.waitroom.cards.contains(&other_member));
    assert_eq!(
        get_heart04(&game, nahone_stage),
        0,
        "No heart04 when discarded card has different name"
    );
    assert_eq!(
        get_blade(&game, nahone_stage),
        0,
        "No blade when discarded card has different name"
    );
}

// =========================================================================
// 19. Other same-name on stage does NOT get buffed
// =========================================================================
#[test]
fn other_same_name_not_buffed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone1 = game.id("PL!HS-bp2-007-R+");
    let nahone2 = game.new_id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [nahone1, nahone2, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let heart1 = get_heart04(&game, nahone1);
    let heart2 = get_heart04(&game, nahone2);
    let blade1 = get_blade(&game, nahone1);
    let blade2 = get_blade(&game, nahone2);

    assert!(
        (heart1 == 1 && heart2 == 0) || (heart1 == 0 && heart2 == 1),
        "Exactly one member should be buffed: heart1={} heart2={}",
        heart1,
        heart2
    );
    assert!(
        (blade1 >= 1 && blade2 == 0) || (blade1 == 0 && blade2 >= 1),
        "Exactly one member should get blade: blade1={} blade2={}",
        blade1,
        blade2
    );

    if heart1 == 1 {
        assert!(blade1 >= 1);
        assert_eq!(blade2, 0);
    } else {
        assert!(blade2 >= 1);
        assert_eq!(blade1, 0);
    }
}

// =========================================================================
// 20. Temporary effect tracked with LiveEnd duration
// =========================================================================
#[test]
fn effect_tracked_as_temporary_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nahone_stage = game.id("PL!HS-bp2-007-R+");
    game.state.player1.stage.stage = [-1, nahone_stage, -1];

    let nahone_hand = game.new_id("PL!HS-PR-007-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_card_set_p1(&mut game);

    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nahone_hand);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let has_live_end_effect = game
        .state
        .temporary_effects
        .iter()
        .any(|e| e.duration == rabuka_engine::types::Duration::LiveEnd);
    assert!(
        has_live_end_effect,
        "Effect should be tracked as temporary with LiveEnd duration"
    );

    assert_eq!(get_heart04(&game, nahone_stage), 1);
    assert!(get_blade(&game, nahone_stage) >= 1);
}
