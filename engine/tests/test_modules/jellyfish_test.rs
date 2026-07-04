use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn check_heart_reduction(game: &TestGame, card_id: i16, expected: i32) {
    use rabuka_engine::card::HeartColor;
    let reduction = game
        .state
        .mods
        .get_need_heart_modifier(card_id, HeartColor::Heart00);
    assert_eq!(reduction, expected);
}

fn fill_decks(game: &mut TestGame) {
    for _ in 0..10 {
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player2.main_deck.cards.push(f);
    }
}

#[test]
fn jellyfish_two_members_appeared_reduce_by_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N");
    let natsumi = game.id("PL!SP-pb1-009-R");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(chisato);
    game.add_to_hand(natsumi);
    game.give_energy(10);

    advance_to_live_card_set_p1(&mut game);

    game.play_to_stage(chisato, MemberArea::LeftSide);
    game.play_to_stage(natsumi, MemberArea::Center);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -2);
}

/// Q99: A member who both appeared AND moved counts as 1, not 2.
#[test]
fn jellyfish_q99_one_member_both_flags_counts_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(chisato);
    game.give_energy(10);

    advance_to_live_card_set_p1(&mut game);

    game.play_to_stage(chisato, MemberArea::Center);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -1);
}

#[test]
fn jellyfish_position_change_swap_creates_countable_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let wakana = game.id("PL!SP-pb1-008-R");
    let chisato = game.id("PL!SP-pb1-014-N");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(wakana);
    game.give_energy(20);

    game.state.player1.stage.stage = [chisato, -1, -1];

    advance_to_live_card_set_p1(&mut game);

    game.play_to_stage(wakana, MemberArea::Center);

    let choice = game
        .state
        .ability_queue
        .is_waiting_for_choice()
        .cloned()
        .expect("Area select choice should be pending after Wakana debut");
    match &choice {
        rabuka_engine::ability::types::Choice::SelectTarget {
            target, options, ..
        } => {
            assert_eq!(target, "area_select");
            assert!(options
                .as_ref()
                .map_or(false, |o| o.contains(&"left".to_string())));
        }
        _ => panic!("Expected SelectTarget for area_select"),
    }

    game.select_option(0);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -2);
}

#[test]
fn jellyfish_mixed_qualifying_and_non_qualifying() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N");
    let natsumi = game.id("PL!SP-pb1-009-R");
    let honoka = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(chisato);
    game.add_to_hand(natsumi);
    game.add_to_hand(honoka);
    game.give_energy(10);

    advance_to_live_card_set_p1(&mut game);

    game.play_to_stage(chisato, MemberArea::LeftSide);
    game.play_to_stage(natsumi, MemberArea::Center);
    game.play_to_stage(honoka, MemberArea::RightSide);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -2);
}

#[test]
fn jellyfish_no_qualifying_members_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let honoka = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(honoka);
    game.give_energy(10);

    advance_to_live_card_set_p1(&mut game);

    game.play_to_stage(honoka, MemberArea::Center);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, 0);
}

#[test]
fn jellyfish_three_members_all_qualify_reduce_by_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N");
    let natsumi = game.id("PL!SP-pb1-009-R");
    let wakana_pr = game.id("PL!SP-PR-010-PR");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(chisato);
    game.add_to_hand(natsumi);
    game.add_to_hand(wakana_pr);
    game.give_energy(10);

    advance_to_live_card_set_p1(&mut game);

    game.play_to_stage(chisato, MemberArea::LeftSide);
    game.play_to_stage(natsumi, MemberArea::Center);
    game.play_to_stage(wakana_pr, MemberArea::RightSide);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -3);
}

#[test]
fn jellyfish_member_moved_by_position_change_without_appearing_this_turn_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let wakana = game.id("PL!SP-pb1-008-R");
    let chisato = game.id("PL!SP-pb1-014-N");
    let honoka = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(wakana);
    game.give_energy(20);

    game.state.player1.stage.stage = [chisato, -1, honoka];

    advance_to_live_card_set_p1(&mut game);

    game.play_to_stage(wakana, MemberArea::Center);

    game.select_option(0);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -2);
}

#[test]
fn jellyfish_member_neither_appeared_nor_moved_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.state.player1.stage.stage = [chisato, -1, -1];

    advance_to_live_card_set_p1(&mut game);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, 0);
}

/// Q98: Member who appeared/moved but left stage → NOT counted.
/// Place Chisato (5yncri5e!) on stage directly, mark as appeared this turn,
/// then remove her. At Jellyfish resolution: Chisato not on stage → not counted.
#[test]
fn jellyfish_q98_member_appeared_then_left_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 5yncri5e!

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.give_energy(20);

    // Place Chisato on stage directly (bypasses area lock)
    game.state.player1.stage.stage = [-1, chisato, -1];

    // Now remove Chisato from stage (simulates leaving via baton-touch, discard, etc.)
    game.state.player1.stage.stage[1] = -1;

    advance_to_live_card_set_p1(&mut game);

    // Mark Chisato as appeared this turn AFTER phase advance (tracking may be cleared during passes)
    game.state.cards_appeared_this_turn.insert(chisato);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    // Q98: Chisato appeared + moved but is NOT on stage → not counted. Reduction = 0.
    check_heart_reduction(&game, jellyfish, 0);
}

/// Q98 edge: 1 member left + 1 stays → only the one who stays counts.
#[test]
fn jellyfish_q98_one_left_one_stays() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 5yncri5e!
    let natsumi = game.id("PL!SP-pb1-009-R"); // 5yncri5e!

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.give_energy(20);

    // Place both on stage
    game.state.player1.stage.stage = [chisato, natsumi, -1];

    // Remove Chisato (she left), Natsumi stays
    game.state.player1.stage.stage[0] = -1;

    advance_to_live_card_set_p1(&mut game);

    // Re-insert appeared tracking AFTER phase advance (it may be cleared during passes)
    game.state.cards_appeared_this_turn.insert(natsumi);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    // Q98: Only Natsumi (5yncri5e!) is on stage → reduction = 1
    check_heart_reduction(&game, jellyfish, -1);
}
