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
fn honoka_bp5_010_live_start_with_discard_and_arise_mills_three_then_recovers_arise() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp5-010-N");
    game.state.player1.live_card_zone.cards.push(me);
    // Deck top: three member cards to mill.
    let m1 = game.id("PL!N-bp3-006-R");
    let m2 = game.id("PL!SP-bp4-022-N");
    let m3 = game.id("PL!S-sd1-001-SD");
    for m in [m3, m2, m1] {
        game.state.player1.main_deck.cards.insert(0, m);
    }
    // A-RISE member already in the waitroom.
    let arise = game.id("PL!-bp5-111-R");
    game.state.player1.waitroom.cards.push(arise);
    // Hand card for the optional discard cost.
    let fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, me);
    // Accept the optional hand-discard cost by choosing which card to
    // discard (allow_skip=true SelectCard), then drain.
    game.select_indices(&[0]);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "milled exactly 3"
    );
    assert!(
        game.state.player1.hand.cards.contains(&arise),
        "A-RISE member retrieved to hand"
    );
}

#[test]
fn honoka_bp5_010_live_start_with_discard_without_arise_still_mills_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp5-010-N");
    game.state.player1.live_card_zone.cards.push(me);
    // Waitroom has no A-RISE member, but the cost needs a hand card.
    let fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, me);
    game.select_indices(&[0]); // accept the optional discard
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "mill still happened"
    );
}
