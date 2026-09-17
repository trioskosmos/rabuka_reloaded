use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains(trigger_str))
        })
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

fn is_waited(game: &TestGame, cid: i16) -> bool {
    game.state.mods.get_orientation_modifier(cid) == Some("wait")
}

#[test]
fn hanayo_pb1_017_debut_without_baton_touch_waits_self_draws_then_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-pb1-017-R");

    game.state.player1.stage.stage[1] = hanayo;
    let spare = game.id(FILLER);
    game.add_to_hand(spare);
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    let deck_card = game.id(FILLER);
    put_on_deck_top(&mut game, 0, deck_card);

    trigger_auto(&mut game, hanayo, AbilityTrigger::Debut, "登場");
    game.select_option(1); // accept self-wait cost

    assert!(is_waited(&game, hanayo), "accepted cost waits her");
    // Draw +1 then mandatory discard −1 → net zero. The discard asks which
    // hand card to drop — resolve it.
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "draw 1 then discard 1 without baton touch → hand back to 1"
    );
}

#[test]
fn hanayo_pb1_017_debut_after_baton_touch_draws_without_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-pb1-017-R");

    game.state.player1.stage.stage[1] = hanayo;
    let spare = game.id(FILLER);
    game.add_to_hand(spare);
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    let deck_card = game.id(FILLER);
    put_on_deck_top(&mut game, 0, deck_card);

    // This turn already had a baton touch.
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.state.baton_touch_count_p1 += 1;

    trigger_auto(&mut game, hanayo, AbilityTrigger::Debut, "登場");
    game.select_option(1);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "バトンタッチ済み → conditional discard skipped → net +1 card"
    );
}
