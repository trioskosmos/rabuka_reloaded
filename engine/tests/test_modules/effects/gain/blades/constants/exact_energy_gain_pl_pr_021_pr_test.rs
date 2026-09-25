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
fn pl_pr_021_pr_exact_seven_energy_blades_refresh_after_pl_sp_bp5_222_r_energy_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-PR-021-PR");
    let yuna = game.id("PL!SP-bp5-222-R");
    game.state.player1.stage.stage = [niko, yuna, -1];
    game.give_energy(6);
    fill_energy_deck(&mut game, 0, 3);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(niko), 0);
    trigger_auto(&mut game, yuna, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(1);
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        7,
        "placement brings the zone to exactly 7"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(niko),
        2,
        "exactly-7 condition must hold right after the gain (no manual recalc)"
    );
}
