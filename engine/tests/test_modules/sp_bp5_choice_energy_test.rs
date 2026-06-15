/// Tests for PL!SP-bp5-001-R+ — Choice with pay_energy cost.
///
/// 登場/ライブ開始時: {{icon_energy.png|E}}支払ってもよい：
///   以下から1つを選ぶ。
///   • 相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
///   • カードを1枚引く。
///
/// Covers: choice + pay_energy (0% coverage)
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Pay energy, effect auto-resolves to draw (no opponent member to wait):
/// verify card drawn.
#[test]
fn sp_bp5_choice_energy_pay_and_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-001-R+");

    game.add_to_hand(card);
    game.give_energy(15);

    let deck_card = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(deck_card);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");

    // Optional cost choice: select_option(1) = PAY (0 = skip)
    while game.has_pending_choice() {
        game.select_option(1);
    }

    assert!(
        game.player().hand.cards.len() >= 1,
        "Should have drawn at least 1 card (no opponent target → auto-select draw)"
    );
}

/// Pay energy, effect auto-selects wait opponent (valid target exists):
/// verify opponent member is waited.
#[test]
fn sp_bp5_choice_energy_pay_and_wait_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-001-R+");
    let opponent = game.id("PL!-sd1-010-SD");

    game.add_to_hand(card);
    game.give_energy(15);

    // Deck card for draw option (not used, but in case)
    let d1 = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(d1);

    // Place opponent member on stage (cost 4, type member_card)
    game.state.player2.stage.stage = [-1, opponent, -1];
    assert_eq!(
        game.state.player2.stage.stage[1], opponent,
        "Opponent should have a valid member on stage at index 1"
    );
    // Sanity: verify the opponent's stage is correctly initialized
    eprintln!(
        "[W TEST] Before play: p2 stage={:?}, opp_id={}",
        game.state.player2.stage.stage, opponent
    );

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");

    eprintln!(
        "[W TEST] After play: p1 stage={:?}, p2 stage={:?}",
        game.state.player1.stage.stage, game.state.player2.stage.stage
    );

    // Optional cost choice: select_option(1) = PAY
    // Effect auto-resolves: finds opponent member → selects wait opponent
    let mut choice_count = 0;
    while game.has_pending_choice() {
        let hand_len = game.player().hand.cards.len();
        let ct = game.pending_choice_type();
        eprintln!(
            "[TEST_CHOICE] #{} type={:?}, hand={}",
            choice_count, ct, hand_len,
        );
        // Choice #0: optional cost → select_option(1) = PAY
        // Choice #1: effect choice → select_option(0) = wait opponent
        if choice_count == 0 {
            game.select_option(1);
        } else {
            game.select_option(0);
        }
        choice_count += 1;
        if choice_count > 10 {
            panic!("Too many choices");
        }
    }

    // After choice resolution, check opponent state
    let opp_wait = game.state.mods.get_orientation_modifier(opponent);
    eprintln!(
        "[TEST] after choices: opp_wait={:?}, choices_seen={}",
        opp_wait, choice_count
    );

    assert!(
        opp_wait == Some(&"wait".to_string()),
        "Opponent member should be in wait state, got {:?}",
        opp_wait
    );
}

/// Insufficient energy: no cost paid, no effect.
#[test]
fn sp_bp5_choice_energy_decline_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-001-R+");

    game.add_to_hand(card);
    game.give_energy(10); // enough to play card, NOT enough for ability cost

    let hand_before = game.player().hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");

    // No energy for cost → no optional cost choice → ability fires without effect
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.player().hand.cards.len(),
        hand_before - 1,
        "Hand should only lose the played card (no draw)"
    );
}
