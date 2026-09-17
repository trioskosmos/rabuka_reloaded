use crate::helpers::*;
use rabuka_engine::core::types::{AbilityTrigger, Phase, TurnPhase};

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

#[test]
fn live_success_placed_energy_skips_next_active_phase_wwd_pl_sp_bp7_027_l() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!SP-bp7-027-L");
    let placed_energy = game.id("LL-E-001-SD");
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.energy_deck.cards.push(placed_energy);
    game.state.current_phase = Phase::LiveVictoryDetermination;
    game.state.current_turn_phase = TurnPhase::Live;
    let turn_before = game.state.turn_number;

    let card = game.db.get_card(live).unwrap();
    assert_eq!(card.card_no.as_ref(), "PL!SP-bp7-027-L");
    assert_eq!(card.name.as_ref(), "What a Wonderful Dream!!");
    let ability = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("WWD must have its printed LiveSuccess ability");
    let ability_id = format!("{}_{}", card.card_no, ability.full_text);
    let card_no = card.card_no.to_string();
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

    assert!(!game.has_pending_choice());
    assert!(game.state.player1.energy_deck.cards.is_empty());
    assert_eq!(
        game.state.player1.energy_zone.cards.as_slice(),
        &[placed_energy]
    );
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
    assert!(
        game.state.mods.is_delayed_cannot_active(placed_energy),
        "WWD must flag the energy placed by its ability"
    );
    assert!(!game.state.mods.is_delayed_cannot_active(live));

    game.pass();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.turn_number, turn_before + 1);
    assert_eq!(game.state.current_phase, Phase::Active);
    assert_eq!(
        game.state.current_turn_phase,
        TurnPhase::FirstAttackerNormal
    );
    assert_eq!(game.state.active_player().id, pid);
    assert!(game.state.mods.is_delayed_cannot_active(placed_energy));

    game.pass();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.current_phase, Phase::Energy);
    assert_eq!(
        game.state.player1.energy_zone.cards.as_slice(),
        &[placed_energy]
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "WWD's placed energy must remain waited after the next active phase"
    );
    assert!(
        !game.state.mods.is_delayed_cannot_active(placed_energy),
        "the one-active-phase lock is consumed only after blocking activation"
    );
}
