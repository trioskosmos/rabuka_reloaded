/// Tests for 桜坂しずく (PL!N-pb1-003-R) — Q196
///
/// 起動/2E: このカードを手札から控え室に置く：カードを1枚引き、ライブ終了時まで、
/// 自分のステージにいる『虹ヶ咲』のメンバー1人はブレードを得る。
/// この能力は、このカードが手札にある場合のみ起動できる。
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Test ability activation: pay 2E + draw 1 card
#[test]
fn hand_only_self_discard_draw_q196_draw_after_discard_cost() {
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

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        13,
        "2 energy should have been paid (15-2=13)"
    );

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "1 card should have been drawn from deck"
    );
}

/// Test that ability cannot activate from stage (requires hand)
#[test]
fn hand_only_self_discard_draw_q196_needs_hand_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-pb1-003-R");
    game.assert_card_identity(shizuku, "PL!N-pb1-003-R");

    game.state.player1.stage.stage[1] = shizuku;
    game.give_energy(15);
    let deck_before = game.state.player1.main_deck.cards.len();

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
    // The refusal must be "this ability is hand-only", not some unrelated guard:
    // the 15 energy is unspent, no prompt opened, and the card never moved.
    game.assert_energy_untouched_after_refusal(15, "時枝 白雪 hand-only 起動 from stage");
    assert_eq!(
        game.state.player1.stage.stage,
        [-1, shizuku, -1],
        "a refused 起動 leaves her on stage"
    );
    assert!(
        !game.has_pending_choice(),
        "a refused activation must not open the discard cost prompt"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "カードを1枚引く must not run"
    );
}
