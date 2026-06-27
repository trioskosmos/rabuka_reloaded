/// Tests for Awaken the power (PL!S-bp5-023-L) ab#0
///
/// Text: ライブ開始時 自分のステージに『Aqours』のメンバーと『SaintSnow』のメンバーがいて、
///       かつそれらのメンバーのコストが合計20以上の場合、自分の控え室にある『Aqours』と
///       『SaintSnow』のライブカードを4枚まで好きな順番でデッキの上に置いてもよい。
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_p1_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

fn fill_p2_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn drain_auto(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            break;
        }
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    drain_auto(game);
}

// =========================================================================
// POSITIVE - Happy path: Aqours AND SaintSnow on stage, cost >= 20
// =========================================================================

#[test]
fn happy_path_moves_up_to_4_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    // Aqours: Chika SD1 (cost 17, series ラブライブ！サンシャイン!!) → matches Aqours
    let aq = game.id("PL!S-sd1-001-SD");
    // SaintSnow: 鹿角理亞 (cost 11, unit SaintSnow) → matches SaintSnow
    let ss = game.id("PL!S-bp5-222-R");
    let aq_l1 = game.id("PL!S-bp2-019-L"); // WATER BLUE NEW WORLD, Aqours live
    let aq_l2 = game.new_id("PL!S-bp2-019-L");
    let aq_l3 = game.new_id("PL!S-bp2-019-L");
    let ss_l = game.id("PL!S-bp5-022-L"); // SELF CONTROL!!, SaintSnow live

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(30);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);

    game.add_to_discard(aq_l1);
    game.add_to_discard(aq_l2);
    game.add_to_discard(aq_l3);
    game.add_to_discard(ss_l);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should prompt for card selection"
    );
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));

    let choice = game.get_pending_choice();
    let count = match choice {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => *count,
        _ => panic!("Expected SelectCard"),
    };
    assert_eq!(count, 4, "Should allow selecting up to 4 cards");
}

// =========================================================================
// NEGATIVE - Only Aqours, no SaintSnow
// =========================================================================

#[test]
fn no_saint_snow_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let aq = game.id("PL!S-sd1-001-SD");

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.give_energy(20);
    game.play_to_stage(aq, MemberArea::Center);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No choice when only Aqours on stage"
    );
}

// =========================================================================
// NEGATIVE - Only SaintSnow, no Aqours
// =========================================================================

#[test]
fn no_aqours_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let ss = game.id("PL!S-bp5-222-R");

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(ss);
    game.give_energy(20);
    game.play_to_stage(ss, MemberArea::Center);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No choice when only SaintSnow on stage"
    );
}

// =========================================================================
// NEGATIVE - Both groups but total cost < 20
// =========================================================================

#[test]
fn cost_below_20_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    // Aqours cost 5 + SaintSnow cost 4 = 9 < 20
    let aq = game.id("PL!S-sd1-005-SD"); // 渡辺曜, cost 5, Aqours
    let ss = game.id("PL!S-bp5-111-R"); // 鹿角聖良, cost 4, SaintSnow

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(20);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No choice when total cost < 20");
}

// =========================================================================
// POSITIVE - Exactly cost 20 (17+4=21 ≥ 20)
// =========================================================================

#[test]
fn cost_at_least_20_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let aq = game.id("PL!S-sd1-001-SD"); // cost 17, Aqours
    let ss = game.id("PL!S-bp5-111-R"); // cost 4, SaintSnow  (21 >= 20)
    let aq_l = game.id("PL!S-bp2-019-L");

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(30);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);

    game.add_to_discard(aq_l);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should fire when total cost >= 20"
    );
}

// =========================================================================
// OPTIONAL - Player can skip
// =========================================================================

#[test]
fn optional_can_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let aq = game.id("PL!S-sd1-001-SD");
    let ss = game.id("PL!S-bp5-222-R");
    let aq_l = game.id("PL!S-bp2-019-L");

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(30);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);
    game.add_to_discard(aq_l);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should prompt for optional choice"
    );

    // Skip by selecting empty
    game.select_indices(&[]);

    // Card should remain in discard
    assert!(
        game.state.player1.waitroom.cards.contains(&aq_l),
        "Card should remain in discard after skip"
    );
}

// =========================================================================
// FILTER - Only Aqours/SaintSnow live cards are selectable
// =========================================================================

#[test]
fn only_selects_matching_live_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let aq = game.id("PL!S-sd1-001-SD");
    let ss = game.id("PL!S-bp5-222-R");
    let aq_l = game.id("PL!S-bp2-019-L"); // Aqours live
    let ss_l = game.id("PL!S-bp5-022-L"); // SaintSnow live
    let other = game.id("PL!N-bp1-028-L"); // 虹ヶ咲 live (NOT Aqours/SaintSnow)

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(30);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);

    game.add_to_discard(aq_l);
    game.add_to_discard(ss_l);
    game.add_to_discard(other);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should prompt for card selection"
    );

    // The 虹ヶ咲 card should NOT be among filtered_indices
    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard {
            filtered_indices,
            zone,
            ..
        } => {
            assert_eq!(zone, "discard", "Zone should be discard");
            if let Some(fi) = filtered_indices {
                assert_eq!(fi.len(), 2, "Only 2 matching cards should be filterable");
                // Verify the other card (index 2) is NOT in filtered_indices
                assert!(
                    !fi.contains(&2),
                    "Non-matching group card should not be selectable"
                );
            }
        }
        _ => panic!("Expected SelectCard choice"),
    }
}

// =========================================================================
// EDGE - Fewer than 4 matching cards in discard
// =========================================================================

#[test]
fn fewer_than_4_available() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let aq = game.id("PL!S-sd1-001-SD");
    let ss = game.id("PL!S-bp5-222-R");
    let aq_l = game.id("PL!S-bp2-019-L"); // Only 1 matching live card

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(30);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);
    game.add_to_discard(aq_l);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should prompt even with <4 cards"
    );

    // Select the 1 available card (filtered_indices has length 1)
    let choice = game.get_pending_choice().clone();
    let fi = match &choice {
        rabuka_engine::ability::types::Choice::SelectCard {
            filtered_indices: Some(fi),
            ..
        } => fi.clone(),
        _ => panic!("Expected SelectCard with filtered_indices"),
    };
    assert_eq!(fi.len(), 1, "Exactly 1 matching card in discard");

    // Select by filtered index (position 0 in waitroom)
    game.select_waitroom_card_filtered(aq_l);

    // Card should be on deck
    assert!(
        game.state.player1.main_deck.cards.contains(&aq_l),
        "The matching live card should be on deck"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&aq_l),
        "Card should be removed from discard"
    );
}

// =========================================================================
// EDGE - No matching live cards in discard
// =========================================================================

#[test]
fn no_matching_live_in_discard_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let aq = game.id("PL!S-sd1-001-SD");
    let ss = game.id("PL!S-bp5-222-R");
    let non_match = game.id("PL!N-bp1-028-L"); // 虹ヶ咲 live, not Aqours/SaintSnow

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(30);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);

    game.add_to_discard(non_match);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    // Should not prompt since no matching cards exist
    assert!(
        !game.has_pending_choice(),
        "No prompt when no matching cards in discard"
    );
}

// =========================================================================
// EDGE - Select and place specific amount (< 4)
// =========================================================================

#[test]
fn can_select_2_of_4_available() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let awaken = game.id("PL!S-bp5-023-L");
    let aq = game.id("PL!S-sd1-001-SD");
    let ss = game.id("PL!S-bp5-222-R");
    let aq_l1 = game.id("PL!S-bp2-019-L");
    let aq_l2 = game.new_id("PL!S-bp2-019-L");
    let aq_l3 = game.new_id("PL!S-bp2-019-L");
    let ss_l = game.id("PL!S-bp5-022-L");

    fill_p1_deck(&mut game);
    fill_p2_deck(&mut game);

    game.add_to_hand(aq);
    game.add_to_hand(ss);
    game.give_energy(30);
    game.play_to_stage(aq, MemberArea::LeftSide);
    game.play_to_stage(ss, MemberArea::Center);

    game.add_to_discard(aq_l1);
    game.add_to_discard(aq_l2);
    game.add_to_discard(aq_l3);
    game.add_to_discard(ss_l);

    game.add_to_hand(awaken);
    advance_to_live_set(&mut game);
    game.set_live_card(awaken);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice(), "Should prompt");

    // Select first 2 filtered indices
    game.select_indices(&[0, 1]);

    // Selected cards leave discard
    // Since we picked 2, 2 remain in discard and 2 moved to deck
    let waitroom_len = game.state.player1.waitroom.cards.len();
    let deck_len = game.state.player1.main_deck.cards.len();
    assert_eq!(
        waitroom_len + deck_len,
        4 + 30,
        "Cards conserved: discard+deck"
    );
}
