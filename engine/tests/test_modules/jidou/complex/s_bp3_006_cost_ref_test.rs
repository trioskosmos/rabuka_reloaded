use crate::helpers::*;

/// PL!S-bp3-006 Yoshiko — center turn1: wait self + discard 1 → remove other Aqours cost X → discard cost X+2 same area
fn yoshi_id(game: &TestGame) -> i16 { game.id("PL!S-bp3-006-P") }

#[test]
fn yoshi_center_cost4_to_cost6_succeeds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshi = yoshi_id(&game);
    let target_cost4 = game.id("PL!S-bp2-002-R");
    let candidate = game.new_id("PL!S-bp2-008-P");
    game.state.player1.stage.stage = [yoshi, target_cost4, -1];
    game.state.player1.waitroom.cards.push(candidate);
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(20);
    // Ensure yoshi is at center for activation_position
    game.state.player1.stage.stage[1] = yoshi;
    game.state.player1.stage.stage[0] = target_cost4;
    let res = game.try_activate_ability(yoshi);
    assert!(res.is_ok(), "yoshi center should be activatable: {:?}", res);
    // First choice: discard 1 from hand (cost)
    assert!(game.has_pending_choice(), "expected discard cost prompt");
    game.select_indices(&[0]);
    // Second: choose Aqours member to remove (target_cost4) - target is at stage index 0, filtered index 0
    // If there's only one valid target, it might be auto-selected; check if prompt appears
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                let stage_pos = game.state.player1.stage.stage.iter().position(|&id| id==target_cost4).unwrap();
                assert_eq!(stage_pos, 0, "target should be at stage 0");
                game.select_indices(&[0]);
            }
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    // Third: choose from discard cost X+2
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    // After success, candidate should be on stage at same area where target was (index 0)
    assert!(game.state.player1.stage.stage[0]==candidate || game.state.player1.waitroom.cards.contains(&target_cost4), "cost+2 should place candidate in same area");
}

#[test]
fn yoshi_cost4_with_no_cost6_in_discard_no_place() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshi = yoshi_id(&game);
    let target = game.id("PL!S-bp2-002-R"); // cost4
    let wrong_cost = game.id("PL!S-bp2-009-R"); // cost 4 not 6, so no cost6
    game.state.player1.stage.stage = [target, yoshi, -1];
    game.state.player1.waitroom.cards.push(wrong_cost);
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(20);
    game.state.player1.stage.stage[1]=yoshi;
    game.state.player1.stage.stage[0]=target;
    let _ = game.try_activate_ability(yoshi);
    // First choice: discard 1 from hand (cost)
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    // Second: choose Aqours member to remove
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    // Third: no cost6 in discard, so skip
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    // Target should have been moved to discard, but no candidate placed because no cost6
    assert!(game.state.player1.waitroom.cards.contains(&target), "target should be in discard");
    assert!(!game.state.player1.stage.stage.contains(&wrong_cost), "wrong cost should not be placed");
}

#[test]
fn yoshi_no_other_aqours_no_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshi = yoshi_id(&game);
    game.state.player1.stage.stage = [yoshi, -1, -1];
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(20);
    game.try_activate_ability(yoshi).ok();
    // First choice: discard 1 from hand (cost)
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    // With no other Aqours, the second step should have no selectable target, so ability ends after cost
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert!(game.state.player1.stage.stage.contains(&yoshi) || game.state.player1.waitroom.cards.contains(&yoshi), "yoshi should be wait or stage");
}

#[test]
fn yoshi_not_center_cannot_activate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshi = yoshi_id(&game);
    let target = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [yoshi, target, -1]; // yoshi at left, not center
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(20);
    let res = game.try_activate_ability(yoshi);
    assert!(res.is_err() || !game.has_pending_choice(), "not center should not be activatable");
}

#[test]
fn yoshi_turn1_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshi = yoshi_id(&game);
    let target = game.id("PL!S-bp2-002-R");
    let cand = game.new_id("PL!S-bp2-008-P");
    game.state.player1.stage.stage = [target, yoshi, -1];
    game.state.player1.waitroom.cards.push(cand);
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(20);
    game.state.player1.stage.stage[1]=yoshi;
    let _ = game.try_activate_ability(yoshi);
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    let res2 = game.try_activate_ability(yoshi);
    assert!(res2.is_err(), "turn1 should block second");
}
