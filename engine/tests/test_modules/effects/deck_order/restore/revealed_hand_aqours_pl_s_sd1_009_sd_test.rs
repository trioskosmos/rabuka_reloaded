use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pl_s_sd1_009_sd_live_start_revealed_aqours_card_moves_from_hand_to_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ruby = game.id("PL!S-sd1-009-SD");
    game.state.player1.stage.stage[1] = ruby;

    let aqours_card = game.new_id("PL!S-sd1-003-SD");
    let non_aq = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(aqours_card);
    game.add_to_hand(non_aq);

    fire_live_start(&mut game, ruby);

    assert!(game.has_pending_choice(), "reveal cost gate offered");
    game.select_option(1);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&aqours_card),
        "revealed Aqours card moved onto the deck"
    );
    let in_deck = game.state.player1.main_deck.cards.contains(&aqours_card);
    assert!(in_deck, "revealed card ends up on the deck");
}

#[test]
fn pl_s_sd1_009_sd_live_start_without_aqours_preserves_non_aqours_hand_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ruby = game.id("PL!S-sd1-009-SD");
    game.state.player1.stage.stage[1] = ruby;
    let only_mu = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(only_mu);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, ruby);

    if !game.has_pending_choice() {
        assert!(game.state.player1.hand.cards.contains(&only_mu));
        return;
    }
    game.select_indices(&[]);
    assert!(game.state.player1.hand.cards.contains(&only_mu));
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined -> deck untouched"
    );
}
