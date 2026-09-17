use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn hs_bp2_017_n_debut_ten_waitroom_cards_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp2-017-N");
    game.state.player1.stage.stage[0] = me;
    for _ in 0..10 {
        let c = game.new_id(FILLER);
        game.state.player1.waitroom.cards.push(c);
    }
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "waitroom has 10+ cards -> draw 1"
    );
}

#[test]
fn hs_bp2_017_n_debut_nine_waitroom_cards_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp2-017-N");
    game.state.player1.stage.stage[0] = me;
    for _ in 0..9 {
        let c = game.new_id(FILLER);
        game.state.player1.waitroom.cards.push(c);
    }
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "only 9 cards in waitroom -> no draw"
    );
}
