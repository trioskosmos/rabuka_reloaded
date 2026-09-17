use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn pl_hs_bp5_021_l_advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn pl_hs_bp5_021_l_single_hasunosora_member_heart01_conversion() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let member = game.id("PL!HS-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(member);
    game.play_to_stage(member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(
        game.state.mods.heart_color_multiplier.contains_key(&member),
        "Member should have heart_color_multiplier"
    );
    assert_eq!(
        game.state.mods.heart_color_multiplier.get(&member),
        Some(&HeartColor::Heart01),
        "Member's hearts should be converted to heart01"
    );
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&live_card),
        "Live card should NOT have heart_color_multiplier"
    );
    let stage_hearts = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    let heart01 = stage_hearts
        .hearts
        .get(&HeartColor::Heart01)
        .copied()
        .unwrap_or(0);
    assert!(
        heart01 >= 2,
        "Should have ≥2 heart01 after conversion (got {})",
        heart01
    );
}

#[test]
fn pl_hs_bp5_021_l_multiple_hasunosora_members_only_selected_converted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let member_a = game.id("PL!HS-sd1-001-SD");
    let member_b = game.id("PL!HS-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(20);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(member_a);
    game.state.player1.hand.cards.push(member_b);
    game.play_to_stage(member_a, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(member_b, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let count: usize = game.state.mods.heart_color_multiplier.len();
    assert_eq!(
        count, 1,
        "Exactly 1 member should have heart_color_multiplier, got {}",
        count
    );
    assert!(
        game.state
            .mods
            .heart_color_multiplier
            .contains_key(&member_a),
        "Selected member (member_a) should have heart_color_multiplier"
    );
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&member_b),
        "Unselected member (member_b) should NOT have heart_color_multiplier"
    );
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&live_card),
        "Live card should NOT have heart_color_multiplier"
    );
}

#[test]
fn pl_hs_bp5_021_l_no_hasunosora_member_no_heart_conversion() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let filler_member = game.id("PL!-sd1-007-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(filler_member);
    game.play_to_stage(filler_member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(
        game.state.mods.heart_color_multiplier.is_empty(),
        "heart_color_multiplier should be empty when no 蓮ノ空 members on stage"
    );
}

#[test]
fn pl_hs_bp5_021_l_three_eligible_members_only_picked_center_converted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let member = game.id("PL!HS-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(30);
    fill_decks(&mut game, filler);
    let member_b = game.new_id("PL!HS-sd1-003-SD");
    let member_c = game.new_id("PL!HS-sd1-003-SD");
    game.state.player1.hand.cards.push(member);
    game.state.player1.hand.cards.push(member_b);
    game.state.player1.hand.cards.push(member_c);
    game.play_to_stage(member, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(member_b, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(member_c, MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[0]);
            }
            Some("SelectCard") => {
                use rabuka_engine::ability::types::Choice;
                let is_stage = match game.get_pending_choice() {
                    Choice::SelectCard { zone, .. } => zone == "stage",
                    _ => false,
                };
                if is_stage {
                    game.select_indices(&[1]);
                    break;
                }
                game.select_indices(&[]);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }
    assert_eq!(
        game.state.mods.heart_color_multiplier.len(),
        1,
        "Exactly 1 member has the multiplier"
    );
    assert!(
        game.state
            .mods
            .heart_color_multiplier
            .contains_key(&member_b),
        "Center member (picked) should have the multiplier"
    );
    assert!(
        !game.state.mods.heart_color_multiplier.contains_key(&member),
        "LeftSide should NOT have multiplier"
    );
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&member_c),
        "RightSide should NOT have multiplier"
    );
}

#[test]
fn pl_hs_bp5_021_l_one_eligible_one_ineligible_auto_selects_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let hasuno = game.id("PL!HS-sd1-003-SD");
    let non_hasuno = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(20);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(hasuno);
    game.state.player1.hand.cards.push(non_hasuno);
    game.play_to_stage(hasuno, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(non_hasuno, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    let mut saw_stage_choice = false;
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[0]);
            }
            Some("SelectCard") => {
                use rabuka_engine::ability::types::Choice;
                let is_stage = match game.get_pending_choice() {
                    Choice::SelectCard { zone, .. } => zone == "stage",
                    _ => false,
                };
                if is_stage {
                    saw_stage_choice = true;
                    game.select_indices(&[0]);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }
    assert!(
        !saw_stage_choice,
        "No stage choice expected (only 1 eligible \u{2192} auto-select)"
    );
    assert!(
        game.state.mods.heart_color_multiplier.contains_key(&hasuno),
        "Hasunosora member should have the multiplier"
    );
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&non_hasuno),
        "Non-Hasunosora member should NOT have the multiplier"
    );
}

#[test]
fn pl_hs_bp5_021_l_heart01_conversion_preserves_exact_total() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let member = game.id("PL!HS-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(member);
    game.play_to_stage(member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let stage_hearts = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    assert_eq!(
        stage_hearts.hearts.get(&HeartColor::Heart01),
        Some(&3),
        "All 3 hearts should be heart01 after transform"
    );
    assert_eq!(
        stage_hearts.hearts.get(&HeartColor::Heart04),
        None,
        "Original heart04 should have 0 after transform"
    );
    assert_eq!(
        stage_hearts.hearts.values().sum::<u8>(),
        3,
        "Total heart count unchanged at 3"
    );
}
