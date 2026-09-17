/// Tests for PL!SP-bp5-010-R (ウィーン・マルガレーテ) ab#0 — Q223
///
/// Ability (登場):
///   自分と相手は、自分のステージのセンターにいるメンバーを
///   ポジション変更する。
///
/// Q223: 相手のメンバーをポジションチェンジする場合、
///        移動するメンバーを決めるのはどちらのプレイヤーか？
/// Answer: 相手プレイヤー。
///
/// Position "center" = SOURCE. Both sides get a choice. Opponent first,
/// then self (via pending_sequential_actions). Swap if dest occupied.
use crate::helpers::*;

/// P1 center empty → no choice for P1.
/// P2 center has p2_center → choice for P2's member.
#[test]
fn wien_bp5_q223_p1_center_empty_opponent_chooses() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp5-010-R");
    let p1_right = game.id("PL!-sd1-010-SD");
    let p2_center = game.id("PL!-sd1-013-SD");

    game.state.player1.hand.cards.push(wien);
    game.state.player1.hand.cards.push(p1_right);
    game.state.player1.stage.stage = [-1, -1, p1_right];
    game.state.player2.stage.stage = [-1, p2_center, -1];

    game.give_energy(13);
    game.play_to_stage(wien, rabuka_engine::zones::MemberArea::LeftSide);

    // P2 center has member → opponent's choice resolves first.
    // Q223: opponent decides where their member goes.
    assert!(
        game.has_pending_choice(),
        "Opponent should get choice for their center member (Q223)"
    );
    game.select_option(0); // choose Left

    // P2 center member moved to LeftSide
    assert_eq!(
        game.state.player2.stage.stage[0], p2_center,
        "P2's center member moved to LeftSide (opponent chose)"
    );
    assert_eq!(game.state.player2.stage.stage[1], -1, "P2 Center now empty");

    // P1 center was empty → no self-side choice
    assert!(
        !game.has_pending_choice(),
        "No self-side choice since P1 center was empty"
    );
}

/// Both centers occupied → opponent chooses first, then self chooses.
#[test]
fn wien_bp5_q223_both_centers_occupied_both_choose() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp5-010-R");
    let p1_center = game.id("PL!-sd1-013-SD");
    let p1_right = game.id("PL!-sd1-014-SD");
    let p2_center = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(wien);
    game.state.player1.hand.cards.push(p1_right);
    game.state.player1.stage.stage = [-1, p1_center, p1_right];
    // P2: Center occupied, LeftSide+RightSide empty
    game.state.player2.stage.stage = [-1, p2_center, -1];

    game.give_energy(13);
    game.play_to_stage(wien, rabuka_engine::zones::MemberArea::LeftSide);

    // Opponent's choice first
    assert!(
        game.has_pending_choice(),
        "Opponent's choice should be first"
    );
    game.select_option(0); // opponent chooses Left

    assert_eq!(
        game.state.player2.stage.stage[0], p2_center,
        "P2 center moved to LeftSide"
    );
    assert_eq!(game.state.player2.stage.stage[1], -1);

    // Self's choice second (from pending_sequential_actions)
    assert!(
        game.has_pending_choice(),
        "Self's choice should come second"
    );
    game.select_option(2); // self chooses Right

    // P1 center moved to RightSide, p1_right swapped to Center
    assert_eq!(
        game.state.player1.stage.stage[2], p1_center,
        "P1 center moved to RightSide (swap)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], p1_right,
        "p1_right swapped to Center"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], wien,
        "Wien stays at LeftSide"
    );
}

/// Both centers empty → no choices at all.
#[test]
fn wien_bp5_q223_both_centers_empty_no_moves() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp5-010-R");
    let p1_right = game.id("PL!-sd1-010-SD");
    let p2_right = game.id("PL!-sd1-013-SD");

    game.state.player1.hand.cards.push(wien);
    game.state.player1.hand.cards.push(p1_right);
    game.state.player1.stage.stage = [-1, -1, p1_right];
    game.state.player2.stage.stage = [-1, -1, p2_right];

    game.give_energy(13);
    game.play_to_stage(wien, rabuka_engine::zones::MemberArea::LeftSide);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [wien, -1, p1_right]);
    assert_eq!(game.state.player2.stage.stage, [-1, -1, p2_right]);
}

/// Opponent chooses destination that has a member → swap.
#[test]
fn wien_bp5_q223_opponent_swap_when_destination_occupied() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp5-010-R");
    let p1_right = game.id("PL!-sd1-010-SD");
    let p2_center = game.id("PL!-sd1-013-SD");
    let p2_left = game.id("PL!-sd1-014-SD");

    game.state.player1.hand.cards.push(wien);
    game.state.player1.hand.cards.push(p1_right);
    game.state.player1.stage.stage = [-1, -1, p1_right];
    // P2: LeftSide occupied, Center occupied, RightSide empty
    game.state.player2.stage.stage = [p2_left, p2_center, -1];

    game.give_energy(13);
    game.play_to_stage(wien, rabuka_engine::zones::MemberArea::LeftSide);

    assert!(game.has_pending_choice());
    game.select_option(0); // opponent chooses Left (occupied)

    // Swap: p2_center ↔ p2_left
    assert_eq!(
        game.state.player2.stage.stage[0], p2_center,
        "p2_center moved to LeftSide (swap)"
    );
    assert_eq!(
        game.state.player2.stage.stage[1], p2_left,
        "p2_left swapped to Center"
    );
}
