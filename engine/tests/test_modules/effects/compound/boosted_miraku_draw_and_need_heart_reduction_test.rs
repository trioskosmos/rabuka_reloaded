use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const BOOST: i16 = 2;
const MIRAKU_A: &str = "PL!HS-bp1-005-PR";
const MIRAKU_B: &str = "PL!HS-PR-005-PR";

fn kyun_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    // Use the TEMPLATE id: trigger processing re-resolves activating_card
    // from the card_no, so the zone copy must be the same instance.
    let live = crate::helpers::card_id(&game.db, "PL!HS-pb1-029-L");
    game.state.player1.live_card_zone.cards.push(live);
    live
}

fn fire_live_start_kyun(game: &mut TestGame, live: i16) {
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(live).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn hs_pb1_029_l_live_start_one_boosted_miraku_draws_one_without_heart_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    let a = game.id(MIRAKU_A);
    game.state.player1.stage.stage[0] = a;
    game.state.mods.add_heart_modifier(a, HeartColor::Heart01, BOOST);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "one boosted みらくらぱーく！ member -> draw 1"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart00),
        0,
        "one member is not '2 or more' -> no need-heart reduction"
    );
}

#[test]
fn hs_pb1_029_l_live_start_two_boosted_miraku_draws_one_and_reduces_two_heart00() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    let a = game.id(MIRAKU_A);
    let b = game.id(MIRAKU_B);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[1] = b;
    game.state.mods.add_heart_modifier(a, HeartColor::Heart01, BOOST);
    game.state.mods.add_heart_modifier(b, HeartColor::Heart02, BOOST);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "two boosted members -> draw 1 (not 2)"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart00),
        -2,
        "2+ members -> this live card's heart00 requirement -2"
    );
}

#[test]
fn hs_pb1_029_l_live_start_unboosted_miraku_neither_draws_nor_reduces_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    let a = game.id(MIRAKU_A);
    game.state.player1.stage.stage[0] = a;

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(
        deck_before, game.state.player1.main_deck.cards.len(),
        "みらくらぱーく！ without extra hearts doesn't qualify"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart00),
        0
    );
}

#[test]
fn hs_pb1_029_l_live_start_boosted_non_miraku_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    // μ's member with extra hearts — wrong group.
    let other = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = other;
    game.state.mods.add_heart_modifier(other, HeartColor::Heart01, BOOST);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(deck_before, game.state.player1.main_deck.cards.len());
}
