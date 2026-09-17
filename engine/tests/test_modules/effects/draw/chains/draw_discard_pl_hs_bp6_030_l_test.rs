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
fn pl_hs_bp6_030_l_draws_then_discards_the_only_hand_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!HS-bp6-030-L");
    game.state.player1.live_card_zone.cards.push(live);
    let drawn = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, drawn);
    fire_live_start(&mut game, live);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        !game.state.player1.hand.cards.contains(&drawn),
        "the drawn card was immediately paid back to the waitroom"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 0);
    assert!(
        game.state.player1.waitroom.cards.contains(&drawn),
        "discarded card lands in the waitroom"
    );
}
