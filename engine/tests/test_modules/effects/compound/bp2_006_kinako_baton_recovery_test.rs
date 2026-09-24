use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KINAKO: &str = "PL!SP-bp2-006-P";
const LIELLA_MEMBER: &str = "PL!SP-pb1-001-R";
const NON_LIELLA_MEMBER: &str = "PL!-sd1-010-SD";
const FILLER: &str = "PL!-sd1-010-SD";

fn setup(game: &mut TestGame, displaced: Option<i16>) -> (i16, i16) {
    let kinako = game.id(KINAKO);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(25);
    if let Some(card) = displaced {
        game.state.player1.stage.stage[1] = card;
    }
    (kinako, filler)
}

fn drain_choices(game: &mut TestGame) {
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

#[test]
fn kinako_baton_touch_recovers_the_displaced_liella_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, Some(target));

    assert_eq!(game.db.get_card(kinako).unwrap().card_no.as_ref(), KINAKO);
    assert_eq!(
        game.db.get_card(target).unwrap().card_no.as_ref(),
        LIELLA_MEMBER
    );

    game.state.player1.hand.cards.push(kinako);
    game.play_to_stage(kinako, MemberArea::Center);
    drain_choices(&mut game);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "the Liella! member displaced by Kinako's baton touch must return to hand"
    );
    assert!(!game.state.player1.waitroom.cards.contains(&target));
}

#[test]
fn kinako_baton_touch_does_not_recover_a_non_liella_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(NON_LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, Some(target));

    game.state.player1.hand.cards.push(kinako);
    game.play_to_stage(kinako, MemberArea::Center);
    drain_choices(&mut game);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        !game.state.player1.hand.cards.contains(&target),
        "Kinako must only recover a displaced Liella! member"
    );
    assert!(game.state.player1.waitroom.cards.contains(&target));
}

#[test]
fn kinako_normal_debut_does_not_recover_a_hand_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id(LIELLA_MEMBER);
    let (kinako, _) = setup(&mut game, None);

    game.state.player1.hand.cards.push(kinako);
    game.state.player1.hand.cards.push(target);
    game.play_to_stage(kinako, MemberArea::Center);
    drain_choices(&mut game);

    assert_eq!(game.state.player1.stage.stage[1], kinako);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "without a baton touch, a hand card is not a recovery target"
    );
}
