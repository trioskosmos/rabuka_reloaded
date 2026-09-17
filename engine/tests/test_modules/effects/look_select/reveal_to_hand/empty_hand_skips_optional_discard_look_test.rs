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

#[test]
fn nozomi_bp3_007_live_start_with_empty_hand_skips_discard_cost_and_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp3-007-R");

    game.state.player1.stage.stage[1] = nozomi;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(
        &mut game,
        nozomi,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    assert!(
        !game.has_pending_choice(),
        "empty hand → optional 2-discard auto-skips, must not prompt"
    );
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined cost → no look, deck untouched"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declined cost → no card joins the hand"
    );
}
