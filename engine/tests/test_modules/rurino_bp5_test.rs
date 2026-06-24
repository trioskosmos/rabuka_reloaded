use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
}

/// Discard a みらくらぱーく！ card → only みらくらぱーく！ members get heart01.
/// Non-みらくらぱーく！ (μ's Printemps) are excluded.
#[test]
fn rurino_bp5_discard_only_matching_unit_gets_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp5-003-R＋"); // unit=みらくらぱーく！
    let hs = game.id("PL!HS-bp6-011-R"); // unit=みらくらぱーく！
    let muse = game.id("PL!-sd1-010-SD"); // unit=Printemps
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs, rurino, muse];
    let cost_card = game.new_id("PL!HS-bp6-011-R");
    let live = game.id("PL!-sd1-020-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    // Set hand explicitly to avoid draw-phase card index interference
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(cost_card);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Pay optional cost — select card from hand (index 0 = cost_card)
    assert!(game.has_pending_choice());
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0]);

    // Before selecting a target, verify the pending choice's filtered_indices
    // only includes matching group members (hs at stage pos 0, rurino at pos 1).
    // muse (Printemps, stage pos 2) is excluded from the candidate pool entirely.
    if let rabuka_engine::ability::types::Choice::SelectCard {
        filtered_indices: Some(fi),
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            fi.as_slice(),
            &[0usize, 1],
            "only みらくらぱーく！ members (hs@0, rurino@1) should be selectable, not Printemps@2"
        );
    } else {
        panic!("Expected SelectCard with filtered_indices");
    }
    // Pick rurino (stage index 1 = 2nd position in filtered_indices)
    game.select_indices(&[1]);

    // Non-matching member (Printemps) was never selectable → gets nothing
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(muse, rabuka_engine::card::HeartColor::Heart01),
        0,
        "μ's Printemps member should NOT get heart01"
    );
    // Other matching member (hs) also gets nothing (target_count=1, we picked rurino)
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(hs, rabuka_engine::card::HeartColor::Heart01),
        0,
        "hs (みらくらぱーく！) should NOT get heart01 (only rurino was selected)"
    );
    // Selected member (rurino) gets exactly +1 heart01
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(rurino, rabuka_engine::card::HeartColor::Heart01),
        1,
        "rurino (みらくらぱーく！) should get +1 heart01"
    );
}

/// Discard a 蓮ノ空 card → ALL 蓮ノ空 group members (both みらくらぱーく！ and スリーズブーケ)
/// are selectable, proving same_group_name resolves to c.group (card position ②),
/// NOT c.unit (card position ③).
#[test]
fn rurino_bp5_cross_unit_same_group_all_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp5-003-R＋"); // group=蓮ノ空, unit=みらくらぱーく！
    let mirakura = game.id("PL!HS-bp6-011-R"); // group=蓮ノ空, unit=みらくらぱーく！
    let trois = game.id("PL!HS-bp1-012-PR"); // group=蓮ノ空, unit=スリーズブーケ
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: all 3 are 蓮ノ空 group but different units
    game.state.player1.stage.stage = [mirakura, rurino, trois];
    let cost_card = game.new_id("PL!HS-bp6-011-R"); // group=蓮ノ空
    let live = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game, filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(cost_card);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Pay optional cost — select card from hand (index 0 = cost_card)
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    // After fix (c.group = 蓮ノ空): ALL 3 stage positions match 蓮ノ空 group
    // Before fix (c.unit = みらくらぱーく！): only positions 0,1 would match
    if let rabuka_engine::ability::types::Choice::SelectCard {
        filtered_indices: Some(fi),
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            fi.as_slice(),
            &[0usize, 1, 2],
            "ALL 3 members share group=蓮ノ空: mirakura@0, rurino@1, trois@2 should all be selectable"
        );
    } else {
        panic!("Expected SelectCard with filtered_indices");
    }
    // Pick rurino
    game.select_indices(&[1]);

    // rurino gets +1 heart01
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(rurino, rabuka_engine::card::HeartColor::Heart01),
        1,
        "rurino should get +1 heart01"
    );
    // trois (スリーズブーケ) was NOT selected → gets nothing
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(trois, rabuka_engine::card::HeartColor::Heart01),
        0,
        "trois (スリーズブーケ) was not selected, should get nothing"
    );
    assert!(!game.has_pending_choice(), "No pending choices after setup");
}
