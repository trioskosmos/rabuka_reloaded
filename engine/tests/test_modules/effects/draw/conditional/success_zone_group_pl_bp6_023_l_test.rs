use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    crate::helpers::fire_trigger(game, cid, trigger, trig);
    game.drain_auto_ability_choices();
}

#[test]
fn sweet_sweet_holiday_pl_bp6_023_l_draws_extra_with_mus_live_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-023-L");
    let mus_live = game.id("PL!-sd1-020-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(mus_live);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "μ's card in success zone -> draw 1 + 1 more"
    );
}

#[test]
fn sweet_sweet_holiday_pl_bp6_023_l_single_draw_without_success_zone_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-023-L");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "no card in success zone -> only the base draw"
    );
}

#[test]
fn sweet_sweet_holiday_pl_bp6_023_l_non_mus_live_in_success_zone_single_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-023-L");
    let niji_live = game.id("PL!N-sd1-025-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(niji_live);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "non-μ's live card in success zone -> no extra draw"
    );
}
