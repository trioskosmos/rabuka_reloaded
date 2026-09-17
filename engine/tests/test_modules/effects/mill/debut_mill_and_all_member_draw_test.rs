use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const LIVE_FILLER: &str = "PL!-sd1-019-SD";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn kosuzu_hs_bp1_008_debut_mill_three_all_members_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let suzuki = game.id("PL!HS-bp1-008-R");
    game.state.player1.stage.stage[1] = suzuki;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    // top three (indices 0..2) become members
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, filler);
    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(&mut game, suzuki, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "all 3 milled cards are members → draw 1"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "three cards moved to the waitroom (控え室)"
    );
}

#[test]
fn kosuzu_hs_bp1_008_debut_mill_three_with_live_card_skips_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let suzuki = game.id("PL!HS-bp1-008-R");
    game.state.player1.stage.stage[1] = suzuki;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    // top three = member, LIVE CARD, member → not all members → no draw
    put_on_deck_top(&mut game, 0, filler);
    let live_filler = game.id(LIVE_FILLER);
    put_on_deck_top(&mut game, 0, live_filler);
    put_on_deck_top(&mut game, 0, filler);
    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(&mut game, suzuki, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "a non-member among the milled three → no draw"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "the mill itself still happens"
    );
}

#[test]
fn sayaka_hs_bp2_011_debut_mills_top_five_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp2-011-PR");

    game.state.player1.stage.stage[1] = sayaka;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 5,
        "five cards milled to the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "deck shrank by five from the top"
    );
}
