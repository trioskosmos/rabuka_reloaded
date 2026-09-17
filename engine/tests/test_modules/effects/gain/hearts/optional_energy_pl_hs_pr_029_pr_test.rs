use crate::helpers::*;
use rabuka_engine::card::HeartColor;
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

fn pl_hs_pr_029_pr_heart_cost_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!HS-PR-029-PR");
    game.state.player1.stage.stage[1] = me;
    me
}

#[test]
fn pl_hs_pr_029_pr_pay_energy_grants_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_hs_pr_029_pr_heart_cost_setup(&mut game);
    let active_before = {
        game.give_energy(3);
        game.state.player1.energy_zone.active_count()
    };
    fire_live_start(&mut game, me);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_option(1);
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        1,
        "paid -> heart01 until live end"
    );
    assert!(
        game.state.player1.energy_zone.active_count() < active_before,
        "energy was consumed"
    );
}

#[test]
fn pl_hs_pr_029_pr_decline_energy_no_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_hs_pr_029_pr_heart_cost_setup(&mut game);
    game.give_energy(3);
    fire_live_start(&mut game, me);
    game.select_indices(&[]);
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        0
    );
}
