use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn natsumeki_pain_pl_hs_sd1_017_sd_draws_and_discards_with_hasunosora_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-017-SD");
    let hino = game.id("PL!HS-bp5-001-P");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage[0] = hino;
    let hand_filler = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_filler);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "hand discard prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    game.select_indices(&[0]);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "gate met -> draw 1"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hand_filler),
        "second step discards 1 hand card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_filler),
        "discarded card lands in the waitroom"
    );
}

#[test]
fn natsumeki_pain_pl_hs_sd1_017_sd_no_hasunosora_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-017-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage[0] = game.id("PL!-sd1-010-SD");

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        0,
        "no 蓮ノ空 member on stage -> nothing happens"
    );
}
