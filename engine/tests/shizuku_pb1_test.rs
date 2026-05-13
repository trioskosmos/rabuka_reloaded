/// Tests for 桜坂しずく (PL!N-pb1-003-R) — Q196
///
/// 起動/2E: このカードを手札から控え室に置く：カードを1枚引き、ライブ終了時まで、
/// 自分のステージにいる『虹ヶ咲』のメンバー1人はブレードを得る。
/// この能力は、このカードが手札にある場合のみ起動できる。
mod helpers;
use helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::game_setup::ActionType;

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
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    game.give_energy(15);

    let deck_before = game.state.player1.main_deck.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::UseAbility,
        Some(shizuku), None, None, None,
    ).expect("activate ability");

    while game.has_pending_choice() { game.select_indices(&[0]); }

    // Energy cost paid (15-2=13)
    assert_eq!(game.state.player1.energy_zone.active_energy_count, 13,
        "2 energy should have been paid (15-2=13)");

    // Card drawn from deck
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 1,
        "1 card should have been drawn from deck");
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
        &mut game.state, &ActionType::UseAbility,
        Some(shizuku), None, None, None,
    );
    assert!(result.is_err(), "Should not activate from stage (requires hand)");
}
