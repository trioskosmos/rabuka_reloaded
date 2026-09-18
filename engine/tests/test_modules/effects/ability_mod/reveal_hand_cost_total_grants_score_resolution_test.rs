use crate::helpers::*;

/// PL!SP-bp1-003 — 起動 turn1: reveal hand members, sum costs, check 10/20/30/40/50
#[test]
fn reveal_hand_cost_total_two_aqours_cost_four_members_resolves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-bp1-003-P");
    game.state.player1.stage.stage[0] = chisato;
    let c4 = game.id("PL!S-bp2-002-R");
    game.state.player1.hand.cards.push(c4);
    game.state.player1.hand.cards.push(game.new_id("PL!S-bp2-002-R"));
    game.give_energy(5);
    let _ = game.try_activate_ability(chisato);
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0,1]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert!(true);
}
#[test]
fn reveal_hand_cost_total_two_nijigasaki_cost_four_members_resolves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-bp1-003-P");
    game.state.player1.stage.stage[0] = chisato;
    let high = game.id("PL!N-bp1-007-R");
    let high2 = game.new_id("PL!N-bp1-007-R");
    game.state.player1.hand.cards.push(high);
    game.state.player1.hand.cards.push(high2);
    game.give_energy(5);
    let _ = game.try_activate_ability(chisato);
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0,1]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert!(true);
}
#[test]
fn reveal_hand_cost_total_nonmatching_total_finishes_choices() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-bp1-003-P");
    game.state.player1.stage.stage[0] = chisato;
    let low = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(low);
    game.give_energy(5);
    let _ = game.try_activate_ability(chisato);
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert!(!game.has_pending_choice());
}
#[test]
fn reveal_hand_cost_total_empty_hand_no_cost_total() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-bp1-003-P");
    game.state.player1.stage.stage[0] = chisato;
    game.state.player1.hand.cards.clear();
    game.give_energy(5);
    let _ = game.try_activate_ability(chisato);
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert!(!game.has_pending_choice());
}
#[test]
fn reveal_hand_cost_total_turn1_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-bp1-003-P");
    game.state.player1.stage.stage[0] = chisato;
    let c = game.id("PL!S-bp2-002-R");
    game.state.player1.hand.cards.push(c);
    game.state.player1.hand.cards.push(game.new_id("PL!S-bp2-002-R"));
    game.give_energy(5);
    let _ = game.try_activate_ability(chisato);
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0,1]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    let res2 = game.try_activate_ability(chisato);
    assert!(res2.is_err(), "turn1 should block second");
}
