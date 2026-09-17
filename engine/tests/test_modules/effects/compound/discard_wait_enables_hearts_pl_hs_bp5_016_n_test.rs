use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn pl_hs_bp5_016_n_discard_waits_opponent_and_enables_constant_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let discard_target = game.id("PL!-sd1-019-SD");
    let opp_member = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player2.stage.stage = [opp_member, opp_member, -1];
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(discard_target);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp_member),
        Some("wait"),
        "Appear: opponent member should be waited"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&discard_target),
        "Appear: discard target should be removed from hand"
    );
    game.state.recalculate_constants();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert!(
        h06 >= 1,
        "Constant: should gain heart06 with 2+ opponent wait (got {})",
        h06
    );
}

#[test]
fn pl_hs_bp5_016_n_decline_without_waited_opponents_no_constant_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.recalculate_constants();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert_eq!(
        h06, 0,
        "Should get 0 heart06 without opponent wait members (got {})",
        h06
    );
}
