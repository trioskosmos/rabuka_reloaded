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

fn wwd_pl_sp_bp7_027_l_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!SP-bp7-027-L");
    game.state.player1.live_card_zone.cards.push(live);
    live
}

#[test]
fn wwd_pl_sp_bp7_027_l_accept_energy_return_while_ahead_grants_score_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = wwd_pl_sp_bp7_027_l_setup(&mut game);
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);
    for _ in 0..2 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
        game.state.player2.energy_zone.add_active(1);
    }

    let deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_live_start(&mut game, live);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_option(1);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before + 1,
        "accepted: one energy moved from zone back to the energy deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before - 1,
        "zone lost exactly the moved card"
    );
    assert!(
        game.state.mods.get_score_modifier(live) >= 1,
        "energy still ahead after paying -> score +1"
    );
}

#[test]
fn wwd_pl_sp_bp7_027_l_decline_no_score_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = wwd_pl_sp_bp7_027_l_setup(&mut game);
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);
    for _ in 0..2 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
        game.state.player2.energy_zone.add_active(1);
    }

    fire_live_start(&mut game, live);
    assert!(game.has_pending_choice());
    game.select_indices(&[]);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "declined -> no score bonus"
    );
}

#[test]
fn wwd_pl_sp_bp7_027_l_accept_cost_while_behind_has_no_immediate_score_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = wwd_pl_sp_bp7_027_l_setup(&mut game);
    game.give_energy(3);
    fill_energy_deck(&mut game, 0, 2);
    for _ in 0..5 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
        game.state.player2.energy_zone.add_active(1);
    }

    fire_live_start(&mut game, live);
    assert!(
        game.has_pending_choice(),
        "optional energy-pay cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget (pay_optional_cost:skip)"
    );
    game.select_option(1);

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "energy behind even before/after paying -> no bonus"
    );
}
