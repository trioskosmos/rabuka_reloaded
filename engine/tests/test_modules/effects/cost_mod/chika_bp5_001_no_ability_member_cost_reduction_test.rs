use crate::helpers::*;

/// Cost reduction applies to no-ability members (4 → 3).
#[test]
fn chika_bp5_001_cost_reduction_applies_to_no_ability_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD"); // no abilities, cost 4

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    let filler_cost = game.db.get_card(filler).unwrap().cost.unwrap_or(0) as usize;
    assert_eq!(filler_cost, 4, "Filler cost 4");

    game.state.player1.hand.cards.push(chika);
    game.give_energy(chika_cost + filler_cost + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(energy_before - energy_after, 3, "Cost 4 reduced to 3");
}

/// Cost reduction does NOT apply to members WITH abilities.
#[test]
fn chika_bp5_001_no_reduction_for_member_with_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let has_ability = game.id("PL!SP-PR-003-PR"); // has 登場 ability

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    let target_cost = game.db.get_card(has_ability).unwrap().cost.unwrap_or(0) as usize;

    game.state.player1.hand.cards.push(chika);
    game.give_energy(chika_cost + target_cost + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(has_ability);
    game.play_to_stage(has_ability, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(
        energy_before - energy_after,
        target_cost as u8,
        "Full cost paid — no reduction"
    );
}

/// Cost reduction applies to no-ability members regardless of cost.
#[test]
fn chika_bp5_001_cost_reduction_applies_floor_check() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let no_ability = game.id("PL!-sd1-010-SD"); // cost 4, no abilities

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;

    game.state.player1.hand.cards.push(chika);
    game.give_energy(chika_cost + 4 + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(no_ability);
    game.play_to_stage(no_ability, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(energy_before - energy_after, 3, "Cost 4 reduced to 3");
}

/// Two Chikas stack reduction (4 → 2, not 3).
#[test]
fn chika_bp5_001_cost_reduction_stacks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let chika2 = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD"); // cost 4, no abilities

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;

    game.state.player1.hand.cards.push(chika);
    game.state.player1.hand.cards.push(chika2);
    game.give_energy(chika_cost * 2 + 4 + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.play_to_stage(chika2, rabuka_engine::zones::MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, rabuka_engine::zones::MemberArea::RightSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(energy_before - energy_after, 2, "Cost 4 reduced by 2 → 2");
}
