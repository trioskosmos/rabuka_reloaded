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

fn fire_enemy_wait_test(game: &mut TestGame, enemy_card_no: &str) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!SP-pb2-047-L");
    game.state.player1.live_card_zone.cards.push(live);
    // Own stage: two genuine Liella! (series Superstar) members.
    let l1 = game.id("PL!SP-pb2-036-N");
    let l2 = game.id("PL!SP-pb2-037-N");
    game.state.player1.stage.stage[0] = l1;
    game.state.player1.stage.stage[1] = l2;
    let enemy = game.id(enemy_card_no);
    game.state.player2.stage.stage[0] = enemy;
    fire_live_start(game, live);
    enemy
}

#[test]
fn welcome_to_bokura_no_sekai_sp_pb2_047_live_start_with_all_liella_and_discard_waits_cost_two_enemy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // Optional discard-1-hand cost first.
    let fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);
    let enemy = fire_enemy_wait_test(&mut game, "PL!SP-PR-010-PR"); // cost 2

    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard cost"
    );
    game.select_indices(&[0]); // accept the discard

    assert_eq!(
        game.state.mods.get_orientation_modifier(enemy),
        Some("wait"),
        "all-Liella stage -> enemy cost<=2 member waited"
    );
}

#[test]
fn welcome_to_bokura_no_sekai_sp_pb2_047_live_start_with_non_liella_and_discard_does_not_wait_enemy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(_fodder);
    // Replace one own member with a mu's member -> not ALL Liella!.
    let live = game.id("PL!SP-pb2-047-L");
    game.state.player1.live_card_zone.cards.push(live);
    let l1 = game.id("PL!SP-pb2-036-N");
    let outsider = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = l1;
    game.state.player1.stage.stage[1] = outsider;
    let enemy = game.id("PL!SP-PR-010-PR");
    game.state.player2.stage.stage[0] = enemy;

    fire_live_start(&mut game, live);
    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected even when the wait condition fails"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard cost"
    );
    game.select_indices(&[0]);

    assert_ne!(
        game.state.mods.get_orientation_modifier(enemy),
        Some("wait"),
        "non-Liella present -> enemy not waited"
    );
}
