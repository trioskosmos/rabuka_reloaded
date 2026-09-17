use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn setup() -> (TestGame, i16, i16, i16) {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!-bp5-024-L");
    let arise = game.id("PL!-bp5-333-R");
    let ally = game.id("PL!-sd1-010-SD");
    assert_eq!(game.db.get_card(live).unwrap().card_no, "PL!-bp5-024-L");
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.stage.stage = [arise, ally, -1];
    game.state.mods.add_orientation_modifier(ally, "wait");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(ally);
        game.state.player2.main_deck.cards.push(ally);
    }
    (game, live, arise, ally)
}

fn fire(game: &mut TestGame, live: i16) {
    let card = game.db.get_card(live).unwrap();
    let ab = card.resolved_abilities().next().unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn private_wars_activates_waited_member_and_grants_only_that_member_one_blade() {
    let (mut game, live, arise, ally) = setup();
    fire(&mut game, live);
    game.select_choice_option(0);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_orientation_modifier(ally).as_deref(), Some("active"));
    assert_eq!(game.state.mods.get_blade_modifier(ally), 1);
    assert_eq!(game.state.mods.get_blade_modifier(arise), 0);
    assert_eq!(game.state.mods.get_blade_modifier(live), 0);
}

#[test]
fn private_wars_waits_original_three_blades_but_not_four() {
    let (mut game, live, _, ally) = setup();
    let low = game.id("PL!-sd1-014-SD");
    let high = game.id("PL!SP-sd1-010-SD");
    game.state.player2.stage.stage = [high, low, -1];
    assert_eq!(game.db.get_card(low).unwrap().blade, 3);
    assert_eq!(game.db.get_card(high).unwrap().blade, 4);
    fire(&mut game, live);
    game.select_choice_option(1);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_orientation_modifier(low).as_deref(), Some("wait"));
    assert_ne!(game.state.mods.get_orientation_modifier(high).as_deref(), Some("wait"));
    assert_eq!(game.state.mods.get_orientation_modifier(ally).as_deref(), Some("wait"));
    assert_eq!(game.state.mods.get_blade_modifier(ally), 0);
}

#[test]
fn private_wars_without_arise_does_not_activate_or_grant_blades() {
    let (mut game, live, _, ally) = setup();
    game.state.player1.stage.stage[0] = -1;
    fire(&mut game, live);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_orientation_modifier(ally).as_deref(), Some("wait"));
    assert_eq!(game.state.mods.get_blade_modifier(ally), 0);
}
