/// Tests for 桜坂しずく (PL!N-pb1-003-R) — Q196
///
/// 起動/2E: このカードを手札から控え室に置く：カードを1枚引き、ライブ終了時まで、
/// 自分のステージにいる『虹ヶ咲』のメンバー1人はブレードを得る。
/// この能力は、このカードが手札にある場合のみ起動できる。
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

fn heart_mods(game: &TestGame, card: i16) -> [i32; 6] {
    use rabuka_engine::card::HeartColor;
    [
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart01),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart02),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart03),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart04),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart05),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart06),
    ]
}

/// Test ability activation: pay 2E + draw 1 card
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
/// Base hearts: heart04=1, heart05=3. After ability: heart04=1+1, heart05=3+0.
#[test]
fn shizuku_bp1_live_start_gains_chosen_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-bp1-003-R＋");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage = [-1, shizuku, -1];
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(20);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    game.set_live_card(live_card);

    let before = heart_mods(&game, shizuku);
    eprintln!(
        "[BEFORE] total: 01={} 02={} 03={} 04={} 05={} 06={}",
        before[0],
        before[1],
        before[2],
        1 + before[3],
        3 + before[4],
        before[5]
    );

    game.pass();
    game.pass();

    if game.has_pending_choice() {
        game.select_option(1);
    }
    assert!(
        game.has_pending_choice(),
        "heart color selection should be pending"
    );
    // SelectHeartColor options are [heart01..heart06], 0-indexed.
    // heart04 is at index 3.
    game.select_option(3);

    let after = heart_mods(&game, shizuku);
    eprintln!(
        "[AFTER]  total: 01={} 02={} 03={} 04={} 05={} 06={}",
        after[0],
        after[1],
        after[2],
        1 + after[3],
        3 + after[4],
        after[5]
    );

    assert_eq!(after[3], 1, "heart04 should have +1 modifier");
    assert_eq!(after[4], 0, "heart05 should have 0 modifier");
}
