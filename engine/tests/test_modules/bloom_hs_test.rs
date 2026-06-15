/// Tests for "Bloom the smile, Bloom the dream!" (PL!HS-bp2-019-L):
///
/// ab#0 (ライブ開始時): If you have a 蓮ノ空 member on stage,
/// may choose one of three heart patterns as the card's required hearts:
///   - 2×heart01 + heart0
///   - 2×heart04 + heart0
///   - 2×heart05 + heart0
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn bloom_live_start_heart_choice_changes_requirements() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];

    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Bloom ability should create a heart pattern choice at live start"
    );

    // Select the first option: heart01 pattern (2×heart01 + heart0)
    game.select_option(0);

    // After choice, the effect should have set heart01 modifier to 2 and heart0 modifier to 1
    let heart01_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart01);
    assert_eq!(
        heart01_mod, 2,
        "Bloom: heart01 should be set to 2 (2×heart01 + heart0)"
    );

    let heart0_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart00);
    assert_eq!(heart0_mod, 1, "Bloom: heart0 should be set to 1");
}

/// Should NOT offer a choice when no 蓮ノ空 member is on stage.
#[test]
fn bloom_no_hasunosuka_member_no_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let bloom = game.id("PL!HS-bp2-019-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(bloom);
    // No 蓮ノ空 member on stage — condition fails
    game.state.player1.stage.stage = [-1, -1, -1];

    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(
        !game.has_pending_choice(),
        "Bloom: no choice should appear without 蓮ノ空 member"
    );

    // Original need_heart should NOT be modified
    let heart01_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart01);
    assert_eq!(heart01_mod, 0, "Bloom: heart01 should not be modified");
    let heart0_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart00);
    assert_eq!(heart0_mod, 0, "Bloom: heart0 should not be modified");
}

#[test]
fn bloom_live_start_choice_second_option_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];

    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());

    // Select the second option: heart04 pattern (2×heart04 + heart0)
    game.select_option(1);

    let heart04_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart04);
    assert_eq!(heart04_mod, 2, "Bloom: heart04 should be set to 2");

    let heart0_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart00);
    assert_eq!(heart0_mod, 1, "Bloom: heart0 should be set to 1");
}

#[test]
fn bloom_live_start_choice_third_option_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let bloom = game.id("PL!HS-bp2-019-L");
    let hasunosuka_member = game.id("PL!HS-bp1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(bloom);
    game.state.player1.stage.stage = [-1, hasunosuka_member, -1];

    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bloom);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice());

    // Select the third option: heart05 pattern (2×heart05 + heart0)
    game.select_option(2);

    let heart05_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart05);
    assert_eq!(heart05_mod, 2, "Bloom: heart05 should be set to 2");

    let heart0_mod = game
        .state
        .mods
        .get_need_heart_modifier(bloom, rabuka_engine::card::HeartColor::Heart00);
    assert_eq!(heart0_mod, 1, "Bloom: heart0 should be set to 1");
}
