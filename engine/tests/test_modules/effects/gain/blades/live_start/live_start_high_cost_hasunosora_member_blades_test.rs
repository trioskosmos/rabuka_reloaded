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
fn pl_hs_cl1_010_cl_cost_fifteen_hasunosora_member_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!HS-cl1-010-CL");
    game.state.player1.live_card_zone.cards.push(live);
    let big = game.id("PL!HS-bp5-004-R");
    game.state.player1.stage.stage[1] = big;
    fire_live_start(&mut game, live);
    assert_eq!(
        game.state.mods.get_blade_modifier(big),
        2,
        "cost-15 Hasunosora member gains +2 blades"
    );
}

#[test]
fn pl_hs_cl1_010_cl_cost_five_hasunosora_member_gains_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!HS-cl1-010-CL");
    game.state.player1.live_card_zone.cards.push(live);
    let low = game.id("PL!HS-cl1-002-CL");
    game.state.player1.stage.stage[1] = low;
    fire_live_start(&mut game, live);
    assert_eq!(
        game.state.mods.get_blade_modifier(low),
        0,
        "cost-5 member does not meet the >=10 threshold"
    );
}
