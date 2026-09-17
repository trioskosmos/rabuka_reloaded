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
fn honoka_pb1_010_r_paid_discard_grants_other_members_one_blade_each() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let honoka = game.id("PL!-pb1-010-R");
    game.state.player1.live_card_zone.cards.push(honoka);

    let mate_a = game.id("PL!S-sd1-001-SD");
    let mate_b = game.id("PL!-sd1-007-SD");
    game.state.player1.stage.stage[0] = mate_a;
    game.state.player1.stage.stage[1] = mate_b;

    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, honoka);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    assert_eq!(game.state.mods.get_blade_modifier(mate_a), 1);
    assert_eq!(game.state.mods.get_blade_modifier(mate_b), 1);
    assert_eq!(
        game.state.mods.get_blade_modifier(honoka),
        0,
        "ほかのメンバー excludes Honoka herself"
    );
}

#[test]
fn honoka_pb1_010_r_empty_hand_skips_discard_and_other_member_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let honoka = game.id("PL!-pb1-010-R");
    game.state.player1.live_card_zone.cards.push(honoka);
    let mate_a = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = mate_a;

    fire_live_start(&mut game, honoka);
    assert!(
        !game.has_pending_choice(),
        "unpayable optional cost (empty hand) must auto-skip without prompting"
    );

    assert_eq!(game.state.mods.get_blade_modifier(mate_a), 0);
}

#[test]
fn honoka_pb1_010_r_paid_discard_without_other_members_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let honoka = game.id("PL!-pb1-010-R");
    game.state.player1.live_card_zone.cards.push(honoka);

    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, honoka);
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_blade_modifier(honoka),
        0,
        "no other members -> no blades, and Honoka never boosts herself"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_fodder),
        "cost fodder was discarded"
    );
}
