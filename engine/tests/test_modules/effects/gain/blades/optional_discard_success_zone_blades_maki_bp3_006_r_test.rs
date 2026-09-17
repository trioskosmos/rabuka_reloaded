use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, live: i16) {
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(live).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn maki_bp3_006_r_paid_discard_three_success_cards_grant_six_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let maki = game.id("PL!-bp3-006-R");
    game.state.player1.live_card_zone.cards.push(maki);
    for _ in 0..3 {
        let s = game.id("PL!-sd1-019-SD");
        game.state.player1.success_live_card_zone.cards.push(s);
    }

    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, maki);
    assert!(game.has_pending_choice(), "optional cost prompted");
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_blade_modifier(maki),
        6,
        "3 success-zone cards x 2 blades"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hand_fodder),
        "cost fodder was discarded"
    );
}

#[test]
fn maki_bp3_006_r_empty_hand_skips_discard_and_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let maki = game.id("PL!-bp3-006-R");
    game.state.player1.live_card_zone.cards.push(maki);
    for _ in 0..3 {
        let s = game.id("PL!-sd1-019-SD");
        game.state.player1.success_live_card_zone.cards.push(s);
    }

    fire_live_start(&mut game, maki);
    assert!(
        !game.has_pending_choice(),
        "unpayable optional cost (empty hand) must auto-skip without prompting"
    );

    assert_eq!(
        game.state.mods.get_blade_modifier(maki),
        0,
        "declined -> no blades despite full success zone"
    );
}

#[test]
fn maki_bp3_006_r_paid_discard_empty_success_zone_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let maki = game.id("PL!-bp3-006-R");
    game.state.player1.live_card_zone.cards.push(maki);
    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, maki);
    assert!(
        game.has_pending_choice(),
        "optional discard cost must be prompted when a hand card exists"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard skippable discard-cost prompt"
    );
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_blade_modifier(maki),
        0,
        "no success cards -> 0 x 2 blades"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_fodder),
        "cost was still paid even though it yielded nothing"
    );
}
