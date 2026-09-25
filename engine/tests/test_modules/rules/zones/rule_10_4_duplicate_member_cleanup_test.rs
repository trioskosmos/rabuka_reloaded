use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn rule_10_4_keeps_newest_member_and_routes_older_stacks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let older = game.id("PL!-sd1-010-SD");
    let middle = game.id("PL!-sd1-010-SD");
    let newest = game.id("PL!-sd1-010-SD");
    let under_member = game.id("PL!-sd1-010-SD");
    let under_energy = game.id("LL-E-001-SD");

    game.state.player1.main_deck.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player1.stage.set_area(MemberArea::Center, older);
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, under_member);
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, under_energy);
    game.state
        .player1
        .stage
        .stage_member_for_check_timing(MemberArea::Center, middle);
    game.state
        .player1
        .stage
        .stage_member_for_check_timing(MemberArea::Center, newest);

    TurnEngine::check_timing(&mut game.state);

    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::Center),
        Some(newest)
    );
    assert!(game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::Center)
        .is_empty());
    assert!(game.state.player1.waitroom.cards.contains(&older));
    assert!(game.state.player1.waitroom.cards.contains(&middle));
    assert!(game.state.player1.waitroom.cards.contains(&under_member));
    assert!(game.state.player1.energy_deck.cards.contains(&under_energy));
}
