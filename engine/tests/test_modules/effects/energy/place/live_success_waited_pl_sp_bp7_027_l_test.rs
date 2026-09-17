use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn wwd_pl_sp_bp7_027_l_live_success_places_one_waited_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!SP-bp7-027-L");
    game.state.player1.live_card_zone.cards.push(live);
    fill_energy_deck(&mut game, 0, 1);

    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
            .unwrap_or_else(|| panic!("missing ライブ成功時 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(live).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card_no),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        1,
        "one energy placed"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "placed WAITED"
    );
}
