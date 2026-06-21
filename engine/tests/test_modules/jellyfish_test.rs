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

#[test]
fn jellyfish_one_member_both_flags_counts_once() {
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
fn jellyfish_member_only_moved_still_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N");

    fill_decks(&mut game);
    game.add_to_hand(jellyfish);
    game.add_to_hand(chisato);
    game.give_energy(10);

    game.play_to_stage(chisato, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -1);
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
