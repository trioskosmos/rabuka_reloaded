use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const KOTORI_CLEAN: &str = "PL!-pb1-021-PR";

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
fn pl_pb1_003_r_declined_self_wait_preserves_active_energy_and_ends_choices() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-pb1-003-R");
    let p1m = game.id(FILLER);
    let p2m = game.id(KOTORI_CLEAN);
    game.state.player1.stage.stage = [p1m, kotori, p2m];
    game.give_energy(5);
    game.state.player1.energy_zone.set_active_count(2);
    trigger_auto(&mut game, kotori, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "optional self-wait must prompt");
    game.select_option(0);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "declined cost → no energy activated"
    );
    assert!(
        !game.has_pending_choice(),
        "nothing further is asked after declining"
    );
}

#[test]
fn pl_pb1_003_r_paid_self_wait_activates_energy_per_printemps_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-pb1-003-R");
    let p1m = game.id(FILLER);
    let p2m = game.id(KOTORI_CLEAN);
    game.state.player1.stage.stage = [p1m, kotori, p2m];
    game.give_energy(5);
    game.state.player1.energy_zone.set_active_count(2);
    trigger_auto(&mut game, kotori, AbilityTrigger::Debut, "登場");
    game.select_option(1);
    assert_eq!(
        game.state.mods.get_orientation_modifier(kotori),
        Some("wait"),
        "accepted cost waits kotori"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "per-Printemps count includes every stage member (3) → 2+3=5 active"
    );
}
