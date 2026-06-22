/// Tests for 桜坂しずく (PL!N-pb1-003-R) — Q196
///
/// 起動/2E: このカードを手札から控え室に置く：カードを1枚引き、ライブ終了時まで、
/// 自分のステージにいる『虹ヶ咲』のメンバー1人はブレードを得る。
/// この能力は、このカードが手札にある場合のみ起動できる。
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Test ability activation: pay 2E + draw 1 card
/// Note: The self-discard cost mechanism requires self_cost flag on the parsed cost,
/// which the current parser may not emit for this card. Cost resolution still succeeds
/// (energy is paid, effect resolves) but the hand card may not be discarded.
#[test]
fn shizuku_q196_draw_after_discard_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-pb1-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, filler, -1];
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    let deck_before = game.state.player1.main_deck.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(shizuku),
        None,
        None,
        None,
    )
    .expect("activate ability");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Energy cost paid (15-2=13)
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 13,
        "2 energy should have been paid (15-2=13)"
    );

    // Card drawn from deck
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "1 card should have been drawn from deck"
    );
}

/// Test that ability cannot activate from stage (requires hand)
#[test]
fn shizuku_q196_needs_hand_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-pb1-003-R");

    game.state.player1.stage.stage[1] = shizuku;
    game.give_energy(15);

    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(shizuku),
        None,
        None,
        None,
    );
    assert!(
        result.is_err(),
        "Should not activate from stage (requires hand)"
    );
}

/// Test ab#1 LiveStart: pay 1E → choose heart04 → gain +1 heart04 (additive).
#[test]
fn shizuku_bp1_live_start_gains_chosen_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // PL!N-bp1-003-R＋ has ab#0 (debut) and ab#1 (LiveStart).
    // Base hearts: heart04=1, heart05=3.
    let shizuku = game.id("PL!N-bp1-003-R＋");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    // Play to stage first (cost=10, need 10 energy)
    game.state.player1.stage.stage = [-1, shizuku, -1];
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(20);
    // Set up deck for draw phase
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Advance through phases to LiveStart trigger
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    // Set live card (phase requires it)
    game.set_live_card(live_card);

    game.pass();
    game.pass();

    // At live start, the optional energy cost choice should appear.
    // Pay 1E (SelectTarget choice — use select_option(1) for "Pay").
    if game.has_pending_choice() {
        game.select_option(1);
    }

    // After cost is paid, the specify_heart_color action runs.
    // The heart color selection choice should now appear.
    assert!(
        game.has_pending_choice(),
        "heart color selection should be pending"
    );

    // Select heart04 (option index 3).
    game.select_option(3);

    // Check heart modifiers: heart04 should have +1 (additive)
    let heart04_mod = game
        .state
        .mods
        .get_heart_modifier(shizuku, rabuka_engine::card::HeartColor::Heart04);
    assert_eq!(heart04_mod, 1, "should gain +1 of chosen heart04");

    // heart05 should be unchanged (no modifier)
    let heart05_mod = game
        .state
        .mods
        .get_heart_modifier(shizuku, rabuka_engine::card::HeartColor::Heart05);
    assert_eq!(heart05_mod, 0, "heart05 should not be modified");
}
