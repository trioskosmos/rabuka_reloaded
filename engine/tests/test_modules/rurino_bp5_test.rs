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

/// Cost with group_reference: "same_group_name" — only hand cards whose group
/// matches the activating card's group are selectable. Different-group and
/// no-group cards must be excluded from the choice.
#[test]
fn himeno_bp5_same_group_cost_filters_hand_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Activating card: 安養寺 姫芽 (PL!HS-bp5-006-R, group=蓮ノ空)
    let himeno = game.id("PL!HS-bp5-006-R");
    // Same-group hand cards (蓮ノ空)
    let same1 = game.new_id("PL!HS-bp6-011-R");
    let same2 = game.new_id("PL!HS-bp6-011-R");
    // Different-group hand card (μ's)
    let wrong = game.id("PL!-sd1-010-SD");
    // Live card (μ's, no group match needed — just triggers live start)
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Put activating card on stage (center area)
    game.state.player1.stage.stage = [-1, himeno, -1];

    fill_decks(&mut game, filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    // Set hand explicitly — no draw-phase interference
    // Hand layout before set_live_card:
    //   [0]=same1(蓮ノ空), [1]=same2(蓮ノ空), [2]=live, [3]=wrong(μ's)
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(same1);
    game.state.player1.hand.cards.push(same2);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(wrong);
    game.set_live_card(live); // removes live from hand to live zone
    finish_live_setup(&mut game);

    // Hand after set_live_card: [same1@0(蓮ノ空), same2@1(蓮ノ空), wrong@2(μ's)]
    // Live start triggers himeno's ability → cost prompt
    assert!(game.has_pending_choice(), "Cost prompt should appear");
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));

    // filtered_indices must only include same-group hand cards (0,1)
    if let rabuka_engine::ability::types::Choice::SelectCard {
        filtered_indices: Some(fi),
        group,
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            fi.as_slice(),
            &[0usize, 1],
            "Only 蓮ノ空 cards (indices 0,1) should be selectable"
        );
        assert_eq!(
            group.as_deref(),
            Some("蓮ノ空"),
            "Choice group should be 蓮ノ空"
        );
    } else {
        panic!("Expected SelectCard with filtered_indices");
    }

    // Select first same-group card (index 0 in filtered = hand index 0)
    game.select_indices(&[0]);

    // Second prompt: select the remaining same-group card
    assert!(
        game.has_pending_choice(),
        "Second selection prompt should appear"
    );
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));

    // Hand after first discard: [same2@0(蓮ノ空), wrong@1(μ's)]
    // Only same2 at index 0 is valid
    game.select_indices(&[0]);

    // Verify both same-group cards were discarded, wrong group still in hand
    assert!(
        game.state.player1.waitroom.cards.contains(&same1),
        "same1 should be in waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&same2),
        "same2 should be in waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&wrong),
        "wrong group card should NOT be in waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&wrong),
        "wrong group card should remain in hand"
    );

    // Verify 2 heart01 granted to activating card
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::HeartColor::Heart01),
        2,
        "Activating card should gain 2 heart01"
    );
}

/// Cost with group_reference: "same_group_name" + no matching cards in hand
/// → optional cost auto-skips with no choice prompt.
#[test]
fn himeno_bp5_same_group_cost_auto_skips_when_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-bp5-006-R");
    // Only wrong-group cards in hand (μ's)
    let wrong1 = game.new_id("PL!-sd1-010-SD");
    let wrong2 = game.new_id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, himeno, -1];

    fill_decks(&mut game, filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    // Hand: [wrong1(μ's), wrong2(μ's), live]
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(wrong1);
    game.state.player1.hand.cards.push(wrong2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // No cost prompt — auto-skipped because no same-group cards in hand
    // The ability fires but optional cost was skipped, so no effect.
    assert!(
        !game.has_pending_choice(),
        "No cost prompt when no same-group cards in hand"
    );

    // No heart01 modifier should be granted
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::HeartColor::Heart01),
        0,
        "No heart01 should be granted when cost is skipped"
    );
}
