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
fn ruby_s_bp5_009_debut_declined_energy_payment_skips_recovery_and_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp5-009-R");
    let saint = game.id("PL!S-pb1-054-SRE"); // 鹿角聖良 unit=SaintSnow
    game.state.player1.stage.stage[1] = ruby;
    game.state.player1.waitroom.cards.push(saint);
    game.give_energy(2);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, ruby, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "optional E payment prompts");
    game.select_option(0); // decline

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declined payment → no SaintSnow fetch"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "declined payment → no blades"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&saint),
        "SaintSnow stays in the waitroom"
    );
}

#[test]
fn ruby_s_bp5_009_debut_paid_energy_recovers_saint_snow_and_grants_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp5-009-R");
    let saint = game.id("PL!S-pb1-054-SRE");
    game.state.player1.stage.stage[1] = ruby;
    game.state.player1.waitroom.cards.push(saint);
    game.give_energy(2);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, ruby, AbilityTrigger::Debut, "登場");
    game.select_option(1); // pay

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "SaintSnow moves from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&saint),
        "fetched card left the waitroom"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        2,
        "そうした場合 → 2 blades until live end"
    );
    assert_eq!(game.state.player1.energy_zone.active_count(), 1, "paid 1 E");
}
