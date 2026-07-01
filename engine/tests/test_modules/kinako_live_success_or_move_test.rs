use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

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

#[test]
fn kinako_auto_triggers_on_live_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-pb2-006-R");
    let liella = game.id("PL!SP-sd1-020-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = kinako;
    game.add_to_discard(liella);
    game.add_to_hand(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();

    assert_eq!(
        game.state.player1.stage.under_cards[1].len(),
        1,
        "Kinako ab#1 should place 1 Liella! card under member after live success"
    );
    assert_eq!(
        game.state.player1.stage.under_cards[1][0], liella,
        "The Liella! card from discard should be under Kinako"
    );
}

#[test]
fn kinako_auto_triggers_on_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-pb2-006-R");
    let liella = game.id("PL!SP-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let pos_changer = game.id("PL!SP-bp5-006-R");

    game.add_to_hand(kinako);
    game.add_to_hand(pos_changer);
    game.add_to_discard(liella);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(20);

    game.try_play_to_stage(kinako, MemberArea::LeftSide)
        .unwrap();
    game.try_play_to_stage(pos_changer, MemberArea::Center)
        .unwrap();
    game.drain_auto_ability_choices();

    game.activate_ability(pos_changer);
    game.drain_auto_ability_choices();

    let actions = game.generated_actions();
    let left_idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == "left")
        })
        .expect("left position option not found");
    game.select_generated(left_idx);
    game.drain_auto_ability_choices();

    assert!(
        !game.state.player1.waitroom.cards.contains(&liella),
        "Liella card should have been removed from waitroom by Kinako's auto ability"
    );
    let kinako_idx = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&id| id == kinako)
        .unwrap();
    assert!(
        game.state.player1.stage.under_cards[kinako_idx].contains(&liella),
        "Liella card should be under Kinako after position change"
    );
}
