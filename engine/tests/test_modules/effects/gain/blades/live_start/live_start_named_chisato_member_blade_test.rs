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
fn pl_sp_bp7_025_l_grants_one_blade_to_staged_chisato_not_other_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let holder = game.id("PL!SP-bp7-025-L");
    game.state.player1.live_card_zone.cards.push(holder);
    let chisato_card = game.id("PL!SP-pb1-014-PR");
    game.state.player1.stage.stage[1] = chisato_card;
    let other = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = other;
    fire_live_start(&mut game, holder);
    assert_eq!(
        game.state.mods.get_blade_modifier(chisato_card),
        1,
        "staged Chisato gains 1 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(other),
        0,
        "other members gain nothing"
    );
}
