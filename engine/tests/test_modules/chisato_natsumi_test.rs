/// Tests for PL!SP-pb2-000-R 嵐 千砂都＆鬼塚夏美 (Chisato Natsumi).
/// ab#0: double baton touch (常時, like Sumire).
/// ab#1: debut — baton touch → draw 1 per Liella! replaced, +2 blade per Liella! without blade heart.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn advance_to_turn2(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
    }
}

/// Double baton touch with 2 Liella! members (both with blade heart):
/// → draw 2, gain 0 blade (both have blade heart, so no blade bonus).
#[test]
fn chisato_natsumi_double_baton_both_have_blade_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R"); // Liella!, has blade_heart
    let liella2 = game.id("PL!SP-bp1-005-R"); // Liella!, has blade_heart

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, liella2, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("play with double baton");

    eprintln!(
        "[DEBUG] hand:{} waitroom:{} deck:{} before drain",
        game.state.player1.hand.cards.len(),
        game.state.player1.waitroom.cards.len(),
        game.state.player1.main_deck.cards.len()
    );

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    eprintln!(
        "[DEBUG] hand:{} waitroom:{} deck:{} after drain",
        game.state.player1.hand.cards.len(),
        game.state.player1.waitroom.cards.len(),
        game.state.player1.main_deck.cards.len()
    );

    assert_eq!(game.state.baton_touch_count, 2, "double baton → count=2");
    // Draw: 2 Liella! replaced → 2 cards. Hand: 2 (initial) - 1 (played) + 2 (draw) = 3
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "draw 2 cards from 2 Liella! replaced"
    );
    // Blade: both have blade heart → 0 bonus
    let blade_mod = game.state.mods.get_blade_modifier(card);
    assert_eq!(blade_mod, 0, "both have blade heart → +0 blade");
}

/// Double baton touch with 2 Liella! members (both WITHOUT blade heart):
/// → draw 2, gain 4 blade (2 members × 2 blade each).
#[test]
fn chisato_natsumi_double_baton_no_blade_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-001-R"); // Liella!, no blade_heart
    let liella2 = game.id("PL!SP-bp1-005-R"); // Liella!, has blade_heart

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, liella2, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("play with double baton");

    eprintln!(
        "[DEBUG2] hand:{} waitroom:{} deck:{} before drain",
        game.state.player1.hand.cards.len(),
        game.state.player1.waitroom.cards.len(),
        game.state.player1.main_deck.cards.len()
    );

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    eprintln!(
        "[DEBUG2] hand:{} waitroom:{} deck:{} after drain",
        game.state.player1.hand.cards.len(),
        game.state.player1.waitroom.cards.len(),
        game.state.player1.main_deck.cards.len()
    );

    assert_eq!(game.state.baton_touch_count, 2, "double baton → count=2");
    // Draw: 2 Liella! replaced → 2 cards. Hand: 2 - 1 + 2 = 3
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "draw 2 cards from 2 Liella! replaced"
    );
    // Blade: 1 Liella! without blade heart → 2 blade
    let blade_mod = game.state.mods.get_blade_modifier(card);
    assert_eq!(blade_mod, 2, "1 no-blade-heart Liella! → +2 blade");
}

/// Q265: Double baton touch with 2 Liella! members (both WITHOUT blade heart):
/// → draw 2, gain 4 blade (2 members × 2 blade each).
#[test]
fn chisato_natsumi_q265_double_baton_both_no_blade_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-001-R"); // Liella!, no blade_heart
    let liella2 = game.id("PL!SP-bp1-001-R"); // Liella!, no blade_heart (same card_no, different instance)

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, liella2, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("play with double baton");

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(game.state.baton_touch_count, 2, "double baton → count=2");
    // Draw: 2 Liella! replaced → 2 cards. Hand: 2 - 1 + 2 = 3
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "draw 2 cards from 2 Liella! replaced"
    );
    // Blade: 2 Liella! without blade heart → 4 blade
    let blade_mod = game.state.mods.get_blade_modifier(card);
    assert_eq!(blade_mod, 4, "2 no-blade-heart Liella! → +4 blade (Q265)");
}
