/// Untested-abilities batch 38 — live-zone aggregate hearts & energy-comparison.
///
/// - PL!S-bp5-013-N 黒澤ダイヤ (ライブ開始時): own live-card zone's need_heart
///   heart04 total ≥4 -> gain heart04 until live end.
/// - PL!SP-bp7-027-L What a Wonderful Dream!! ab#0 (ライブ開始時): optional
///   cost (move 1 energy from zone back to energy deck); if own energy still
///   exceeds opponent's -> this live card's score +1.
/// - same card ab#1 (ライブ成功時): place 1 WAITED energy from energy deck;
///   it must not activate next turn (delayed cannot_active restriction).
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

// ====================================================================
// PL!S-bp5-013-N 黒澤ダイヤ — live-zone heart04 aggregate
// ====================================================================

fn dia_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let dia = game.id("PL!S-bp5-013-N");
    game.state.player1.stage.stage[1] = dia;
    dia
}

#[test]
fn dia_heart04_total_exactly_four_grants_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = dia_setup(&mut game);

    // Two live cards with heart04:2 each -> total exactly 4.
    for _ in 0..2 {
        let l = game.id("PL!S-bp2-020-L");
        game.state.player1.live_card_zone.cards.push(l);
    }

    fire_live_start(&mut game, dia);

    assert_eq!(
        game.state.mods.get_heart_modifier(dia, HeartColor::Heart04),
        1,
        "aggregate == 4 satisfies >=4 -> heart04 granted"
    );
}

#[test]
fn dia_heart04_total_three_no_grant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = dia_setup(&mut game);

    // 2 + 1 = 3 < 4.
    let l1 = game.id("PL!S-bp2-020-L"); // heart04 x2
    let l2 = game.id("PL!S-bp3-020-L"); // heart04 x1
    game.state.player1.live_card_zone.cards.push(l1);
    game.state.player1.live_card_zone.cards.push(l2);

    fire_live_start(&mut game, dia);

    assert_eq!(
        game.state.mods.get_heart_modifier(dia, HeartColor::Heart04),
        0,
        "aggregate 3 < 4 -> no grant"
    );
}

#[test]
fn dia_empty_live_zone_no_grant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = dia_setup(&mut game);

    fire_live_start(&mut game, dia);

    assert_eq!(
        game.state.mods.get_heart_modifier(dia, HeartColor::Heart04),
        0
    );
}

// ====================================================================
// PL!SP-bp7-027-L What a Wonderful Dream!! — energy comparison & waited energy
// ====================================================================

fn wwd_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!SP-bp7-027-L");
    game.state.player1.live_card_zone.cards.push(live);
    live
}

#[test]
fn wwd_accept_cost_energy_ahead_scores_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = wwd_setup(&mut game);
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
    game.select_option(1); // accept
    // Remaining prompts: which energy card leaves the zone, then the
    // ability's own {E} activation-cost payment.
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
fn wwd_decline_no_score_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = wwd_setup(&mut game);
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);
    for _ in 0..2 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
        game.state.player2.energy_zone.add_active(1);
    }

    fire_live_start(&mut game, live);
    assert!(game.has_pending_choice());
    game.select_indices(&[]); // decline

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "declined -> no score bonus"
    );
}

#[test]
fn wwd_energy_not_ahead_after_cost_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = wwd_setup(&mut game);
    game.give_energy(3); // paying drops to 2
    fill_energy_deck(&mut game, 0, 2);
    for _ in 0..5 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
        game.state.player2.energy_zone.add_active(1);
    } // opponent ahead either way

    fire_live_start(&mut game, live);
    if game.has_pending_choice() {
        game.select_option(1); // accept
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "energy behind even before/after paying -> no bonus"
    );
}

#[test]
fn wwd_live_success_places_waited_energy_with_delayed_lock() {
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
