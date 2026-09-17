use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn formation_setup(matching: usize) -> (TestGame, i16, [i16; 3]) {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!SP-pb2-050-L");
    assert_eq!(game.db.get_card(live).unwrap().card_no, "PL!SP-pb2-050-L");
    let stage = std::array::from_fn(|i| {
        game.id(if i < matching {
            "PL!SP-bp1-014-N"
        } else {
            "PL!-sd1-010-SD"
        })
    });
    game.state.player1.stage.stage = stage;
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    (game, live, stage)
}

#[test]
fn live_start_two_subunit_members_can_reform_all_members() {
    let (mut game, live, [a, b, c]) = formation_setup(2);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice());
    game.select_option(2);
    assert!(game.has_pending_choice());
    game.select_option(1);
    assert!(game.has_pending_choice());
    game.select_option(0);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [c, b, a]);
    assert_eq!(game.state.player2.stage.stage, [-1; 3]);
}

#[test]
fn live_start_two_subunit_members_can_keep_formation() {
    let (mut game, live, stage) = formation_setup(2);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    for index in 0..3 {
        assert!(game.has_pending_choice());
        game.select_option(index);
    }
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, stage);
    assert!(game.state.cards_moved_this_turn.is_empty());
}

#[test]
fn live_start_one_subunit_member_does_not_offer_formation() {
    let (mut game, live, stage) = formation_setup(1);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, stage);
    assert!(game.state.cards_moved_this_turn.is_empty());
}
