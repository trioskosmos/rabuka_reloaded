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

/// Cost with group_reference: "same_group_name" — hand cards whose group
/// forms a matching pair (2+ cards sharing a group) are selectable.
/// Cards from groups with only 1 member are excluded.
#[test]
fn himeno_bp5_same_group_cost_filters_hand_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Activating card: 安養寺 姫芽 (PL!HS-bp5-006-R, group=蓮ノ空)
    let himeno = game.id("PL!HS-bp5-006-R");
    // Same-group hand cards (蓮ノ空) — pair qualifies
    let same1 = game.new_id("PL!HS-bp6-011-R");
    let same2 = game.new_id("PL!HS-bp6-011-R");
    // Different-group hand card (Printemps, only 1 → excluded)
    let wrong = game.id("PL!-sd1-010-SD");
    // Live card
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, himeno, -1];

    fill_decks(&mut game, filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(same1);
    game.state.player1.hand.cards.push(same2);
    game.state.player1.hand.cards.push(wrong);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    // Hand: [same1@0(蓮ノ空), same2@1(蓮ノ空), wrong@2(Printemps)]
    assert!(game.has_pending_choice(), "Cost prompt should appear");
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));

    // filtered_indices must only include the 蓮ノ空 pair (0,1)
    // wrong (Printemps) has only 1 member → excluded
    if let rabuka_engine::ability::types::Choice::SelectCard {
        filtered_indices: Some(fi),
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            fi.as_slice(),
            &[0usize, 1],
            "Only 蓮ノ空 pair (indices 0,1) should be selectable"
        );
    } else {
        panic!("Expected SelectCard with filtered_indices");
    }

    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "Second selection prompt should appear"
    );
    game.select_indices(&[0]);

    assert!(game.state.player1.waitroom.cards.contains(&same1));
    assert!(game.state.player1.waitroom.cards.contains(&same2));
    assert!(!game.state.player1.waitroom.cards.contains(&wrong));
    assert!(game.state.player1.hand.cards.contains(&wrong));

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::HeartColor::Heart01),
        2,
        "Activating card should gain 2 heart01"
    );
}

/// Cost with group_reference: "same_group_name" — no cards share a group name
/// with each other → optional cost auto-skips.
#[test]
fn himeno_bp5_same_group_cost_auto_skips_when_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-bp5-006-R");
    // 2 cards from different groups — no pair
    let a = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空
    let b = game.new_id("PL!-sd1-010-SD"); // Printemps
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, himeno, -1];

    fill_decks(&mut game, filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(a);
    game.state.player1.hand.cards.push(b);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No cost prompt — 1 蓮ノ空 + 1 Printemps = no matching pair"
    );

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::HeartColor::Heart01),
        0,
        "No heart01 should be granted when cost is skipped"
    );
}

/// Cost with group_reference: "same_group_name" — 2 same-group cards plus
/// a no-group card. The pair qualifies, no-group card is excluded.
#[test]
fn himeno_bp5_same_group_cost_excludes_no_group_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-bp5-006-R");
    let nogroup = game.new_id("PL!-bp5-111-R");
    let same1 = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空
    let same2 = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, himeno, -1];

    fill_decks(&mut game, filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nogroup);
    game.state.player1.hand.cards.push(same1);
    game.state.player1.hand.cards.push(same2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    // Hand: [nogroup@0, same1@1(蓮ノ空), same2@2(蓮ノ空)]
    assert!(game.has_pending_choice(), "Cost prompt should appear");

    if let rabuka_engine::ability::types::Choice::SelectCard {
        filtered_indices: Some(fi),
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            fi.as_slice(),
            &[1usize, 2],
            "Only 蓮ノ空 pair (indices 1,2) should be selectable, not no-group@0"
        );
    } else {
        panic!("Expected SelectCard with filtered_indices");
    }

    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Second selection should appear");
    game.select_indices(&[0]);
}

/// Cost with group_reference: "same_group_name" — ALL hand cards are no-group
/// (empty series), none match the activating card's group → optional cost auto-skips.
#[test]
fn himeno_bp5_same_group_cost_auto_skips_when_only_no_group_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-bp5-006-R"); // group=蓮ノ空
    let nogroup1 = game.new_id("PL!-bp5-111-R");
    let nogroup2 = game.new_id("PL!-bp5-111-R");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, himeno, -1];

    fill_decks(&mut game, filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(nogroup1);
    game.state.player1.hand.cards.push(nogroup2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // No cost prompt — auto-skipped because no same-group cards in hand
    assert!(
        !game.has_pending_choice(),
        "No cost prompt when only no-group cards in hand"
    );

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(himeno, rabuka_engine::card::HeartColor::Heart01),
        0,
        "No heart01 should be granted when cost is skipped"
    );
}
