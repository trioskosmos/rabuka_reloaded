use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn nandodatte_yakusoku_pl_s_bp7_019_l_places_two_aqours_cards_under_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp7-019-L");
    let aq1 = game.id("PL!S-sd1-003-SD");
    let aq2 = game.id("PL!S-sd1-017-SD");
    let non_aq = game.new_id("PL!-sd1-010-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.waitroom.cards.push(non_aq);
    game.state.player1.waitroom.cards.push(aq1);
    game.state.player1.waitroom.cards.push(aq2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        game.has_pending_choice(),
        "valid candidates -> selection prompt"
    );
    game.select_indices(&[0, 1]);

    assert!(
        !game.state.player1.waitroom.cards.contains(&aq1)
            && !game.state.player1.waitroom.cards.contains(&aq2),
        "both Aqours cards left the waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_aq),
        "non-Aqours card stays in the waitroom"
    );
    let deck = &game.state.player1.main_deck.cards;
    assert_eq!(deck.len(), 32, "30 fillers + 2 returned cards");
    assert!(
        deck.ends_with(&vec![aq1, aq2]) || deck.ends_with(&vec![aq2, aq1]),
        "the two Aqours cards sit at the deck BOTTOM (either order)"
    );
}
