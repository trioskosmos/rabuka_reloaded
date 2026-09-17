use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

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
fn yuna_sp_bp5_222_live_start_optional_payment_places_wait_energy_only_when_paid() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yuna = game.id("PL!SP-bp5-222-R");
    game.state.player1.stage.stage[1] = yuna;
    game.give_energy(3);
    fill_energy_deck(&mut game, 0, 5);
    let zone_before = game.state.player1.energy_zone.cards.len();

    trigger_auto(&mut game, yuna, AbilityTrigger::LiveStart, "ライブ開始時");

    // Optional payment is offered (rules 9.6.2.3: optional ⇒ still legal at 3).
    assert!(
        game.has_pending_choice(),
        "optional E payment should prompt the player"
    );
    game.select_option(0); // skip
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "skipping pays nothing"
    );

    // Trigger again (no turn limit on this auto ability within a test) → pay.
    trigger_auto(&mut game, yuna, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        game.has_pending_choice(),
        "optional E payment prompt expected on re-trigger"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget (pay_optional_cost:skip)"
    );
    game.select_option(1); // pay
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "paying places 1 energy card from the energy deck"
    );
    // Placed WAIT: active count unchanged by the placement itself (3 − 1 paid
    // stays 2; the new card is wait).
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "paid 1 of 3 active; placed energy sits in wait"
    );
}
