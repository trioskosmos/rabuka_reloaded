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
fn pl_bp4_024_l_mus_member_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!-bp4-024-L");
    game.state.player1.live_card_zone.cards.push(live);
    let mus_mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[1] = mus_mate;
    let mus_member = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = mus_member;
    fire_live_start(&mut game, live);
    assert_eq!(game.state.mods.get_blade_modifier(mus_member), 1);
}

#[test]
fn pl_bp4_024_l_no_mus_member_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!-bp4-024-L");
    game.state.player1.live_card_zone.cards.push(live);
    let aq = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = aq;
    fire_live_start(&mut game, live);
    assert_eq!(game.state.mods.get_blade_modifier(aq), 0);
}
