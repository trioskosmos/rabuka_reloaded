/// Tests for PL!SP-pb2-000-R 嵐 千砂都＆鬼塚夏美 (Chisato Natsumi).
/// ab#0: double baton touch (常時, like Sumire).
/// ab#1: debut — gated on「バトンタッチして登場した場合」(movement_condition);
///       draws 1 per Liella! member moved to the waitroom BY THIS baton touch,
///       and grants +2 blade until live end per such member WITHOUT blade heart.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn advance_to_turn2(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
    }
}

/// Drain auto-trigger prompts. Dispatches on the Choice variant so a
/// mis-typed prompt surfaces as a stuck choice instead of being blindly
/// consumed (WRITING_TESTS §Hard assertions).
fn drain_auto_prompts(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            _ => break,
        }
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

    drain_auto_prompts(&mut game);

    assert_eq!(game.state.baton_touch_count_p1, 2, "double baton → count=2");
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

    drain_auto_prompts(&mut game);

    assert_eq!(game.state.baton_touch_count_p1, 2, "double baton → count=2");
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

    drain_auto_prompts(&mut game);

    assert_eq!(game.state.baton_touch_count_p1, 2, "double baton → count=2");
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

// ==== ab#1 debut tests (draw + blade per replaced Liella!) ====

/// ab#1: draw should count only the baton-touch-replaced Liella! members,
/// NOT the full waitroom (regression test for the per_unit discard bug).
#[test]
fn chisato_natsumi_debut_draw_counts_only_replaced_not_full_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-001-R");
    let liella2 = game.id("PL!SP-bp1-005-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    // Pre-fill waitroom with several non-Liella! cards to test the regression:
    // old bug would draw = full waitroom count, but correct is only replaced count.
    for _ in 0..5 {
        game.state.player1.waitroom.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
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

    drain_auto_prompts(&mut game);

    // Waitroom had 5 pre-existing + 2 replaced = 7 total.
    // But ab#1 should count only the 2 replaced Liella! (via recently_moved).
    // Hand: 3 - 1 (played) + 2 (draw) = 4
    assert_eq!(
        game.state.player1.hand.cards.len(),
        4,
        "draw 2 cards from 2 replaced Liella!, ignoring pre-existing waitroom cards"
    );
}

/// ab#1: double baton, both Liella! have blade heart → draw 2, blade +0.
#[test]
fn chisato_natsumi_debut_double_baton_both_blade_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R");
    let liella2 = game.id("PL!SP-bp1-005-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
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

    drain_auto_prompts(&mut game);

    // Hand: 3 - 1 + 2 = 4
    assert_eq!(
        game.state.player1.hand.cards.len(),
        4,
        "draw 2 from 2 replaced Liella! members"
    );
    // Blade: both have blade heart → +0
    let blade_mod = game.state.mods.get_blade_modifier(card);
    assert_eq!(blade_mod, 0, "both have blade heart → +0 blade");
}

/// ab#1: double baton, no Liella! have blade heart → draw 2, blade +4.
#[test]
fn chisato_natsumi_debut_double_baton_no_blade_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-001-R");
    let liella2 = game.id("PL!SP-bp1-001-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
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

    drain_auto_prompts(&mut game);

    // Hand: 3 - 1 + 2 = 4
    assert_eq!(
        game.state.player1.hand.cards.len(),
        4,
        "draw 2 from 2 replaced Liella! members"
    );
    // Blade: 2 without blade heart → +4
    let blade_mod = game.state.mods.get_blade_modifier(card);
    assert_eq!(blade_mod, 4, "2 no-blade-heart Liella! → +4 blade");
}

/// ab#1: single baton with 1 Liella! member → draw 1, blade +2 (if no blade heart).
#[test]
fn chisato_natsumi_debut_single_baton_one_liella() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-001-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    // Only 1 Liella! on stage at Left.  Play directly to that area so the
    // cost-phase baton touch replaces the occupied member.
    game.state.player1.stage.stage = [liella1, -1, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with single baton");

    drain_auto_prompts(&mut game);

    // Hand: 3 - 1 + 1 = 3 (drew 1 for the 1 replaced Liella!)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "draw 1 from 1 replaced Liella! member"
    );
    // Blade: 1 without blade heart → +2
    let blade_mod = game.state.mods.get_blade_modifier(card);
    assert_eq!(blade_mod, 2, "1 no-blade-heart Liella! → +2 blade");
}

/// ab#1 GATE: the movement_condition「バトンタッチして登場した場合」must block
/// the entire effect on a NORMAL debut — no draws, no blade bonus. Without
/// this test, dropping `condition` from the parsed effect would go unnoticed.
#[test]
fn chisato_natsumi_normal_debut_without_baton_touch_fires_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.give_energy(20);

    advance_to_turn2(&mut game);

    // Baselines AFTER all phase-advance draws.
    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    // Normal play onto an EMPTY area — no baton touch happens at all.
    game.play_to_stage(card, MemberArea::Center);
    drain_auto_prompts(&mut game);

    assert!(
        !game.has_pending_choice(),
        "a normal debut must leave no unresolved prompts"
    );
    game.assert_hand(hand_before - 1, "played 1, drew 0");
    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        0,
        "deck untouched: no draw on a non-baton debut"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "no blade bonus on a non-baton debut"
    );
}

/// ab#1 GROUP FILTER: replacing [Liella!, non-Liella!] must count ONLY the
/// Liella! member for both the draw and the blade gain (group_names filter).
#[test]
fn chisato_natsumi_baton_replacing_non_liella_counts_only_liella() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella = game.id("PL!SP-bp1-001-R"); // Liella!, no blade_heart
    let non_liella = game.id("PL!-sd1-001-SD"); // μ's-era member, NOT Liella!

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    // Sharpen discrimination: pre-existing waitroom cards must not be counted.
    for _ in 0..3 {
        game.state.player1.waitroom.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella, non_liella, -1];
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

    drain_auto_prompts(&mut game);

    // Only 1 replaced member is『Liella!』→ draw exactly 1.
    // Hand: 3 - 1 (played) + 1 (draw) = 3. A group-blind implementation
    // would draw 2 (or the full waitroom's 5).
    game.assert_hand(3, "drew only for the ONE replaced Liella!");
    // Blade: +2 per blade-heart-less Liella! — the non-Liella! adds nothing
    // even though it also lacks a blade heart.
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        2,
        "only the Liella! member feeds the blade gain"
    );
}

/// ab#1 DURATION:「ライブ終了時まで」— the blade bonus must be gone once the
/// live ends (victory determination).
#[test]
fn chisato_natsumi_blade_bonus_expires_at_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-pb2-000-R");
    let filler = game.id("PL!-sd1-010-SD");
    let liella = game.id("PL!SP-bp1-001-R");
    let live = game.id("PL!-sd1-020-SD"); // score-2 live card

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella, -1, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with single baton");

    drain_auto_prompts(&mut game);
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        2,
        "bonus active right after the baton debut"
    );

    // Run a full live: set a live card, pass through both live-card-set
    // phases and the performances into victory determination.
    game.add_to_hand(live);
    for _ in 0..4 {
        game.pass();
    }
    game.set_live_card(live);
    for _ in 0..3 {
        game.pass();
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "blade bonus expired at live end"
    );
}
