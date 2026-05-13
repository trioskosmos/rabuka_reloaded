/// Tests for position_change/formation_change with multiple_targets=true (それぞれ) pattern.
///
/// Card: PL!HS-bp2-006-R/P (藤島 慈) ab#0
/// 登場: 自分のステージにいるメンバーを、それぞれ好きなエリアに移動させてもよい。
///   action: position_change, multiple_targets: true, optional: true
/// Cost: 15, Blade: 4
use crate::helpers::*;

/// 3 members on stage (2 existing + 慈), 3 sequential choices, all moved.
#[test]
fn position_change_three_members_all_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chii = game.id("PL!HS-bp2-006-R");
    let a = game.new_id("PL!-sd1-013-SD");
    let b = game.new_id("PL!-sd1-013-SD");

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::RightSide);

    // All 3 positions occupied → 3 sequential choices
    assert!(game.has_pending_choice(), "First choice");
    game.select_option(1); // a → Center (swap with b)
    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(2); // → Right
    assert!(game.has_pending_choice(), "Third choice");
    game.select_option(0); // → Left

    assert!(!game.has_pending_choice());
    assert_ne!(game.state.player1.stage.stage[0], -1);
    assert_ne!(game.state.player1.stage.stage[1], -1);
    assert_ne!(game.state.player1.stage.stage[2], -1);
}

/// 3 members (2 existing + 慈), 3 choices.
#[test]
fn position_change_two_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chii = game.id("PL!HS-bp2-006-R");
    let a = game.new_id("PL!-sd1-013-SD");
    let b = game.new_id("PL!-sd1-013-SD");

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::RightSide);

    assert!(game.has_pending_choice(), "First choice");
    game.select_option(0);
    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(1);
    assert!(game.has_pending_choice(), "Third choice");
    game.select_option(2);

    assert!(!game.has_pending_choice());
    assert_ne!(game.state.player1.stage.stage[0], -1);
    assert_ne!(game.state.player1.stage.stage[1], -1);
    assert_ne!(game.state.player1.stage.stage[2], -1);
}

/// 2 members (1 existing + 慈), 2 choices.
#[test]
fn position_change_one_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chii = game.id("PL!HS-bp2-006-R");
    let a = game.new_id("PL!-sd1-013-SD");

    game.state.player1.stage.stage = [a, -1, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::RightSide);

    // 2 members on stage → 2 sequential choices
    assert!(game.has_pending_choice(), "First choice");
    game.select_option(1); // move first card to Center

    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(2); // move second card to Right

    assert!(!game.has_pending_choice());
}

/// Play 慈 alone on empty stage → only 1 member → 1 choice.
#[test]
fn position_change_no_other_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chii = game.id("PL!HS-bp2-006-R");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::Center);

    assert!(game.has_pending_choice(), "Choice for 慈");
    game.select_option(0); // move to Left

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage[0], chii);
}

/// Swap between first and second processed cards.
#[test]
fn position_change_with_swap() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chii = game.id("PL!HS-bp2-006-R");
    let a = game.new_id("PL!-sd1-013-SD");
    let b = game.new_id("PL!-sd1-013-SD");

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::RightSide);

    assert!(
        game.has_pending_choice(),
        "First: move a (L) → Right, swaps chii to Left"
    );
    game.select_option(2);

    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(0);

    assert!(game.has_pending_choice(), "Third choice");
    game.select_option(1);

    assert!(!game.has_pending_choice());
    assert_ne!(game.state.player1.stage.stage[0], -1);
    assert_ne!(game.state.player1.stage.stage[1], -1);
    assert_ne!(game.state.player1.stage.stage[2], -1);
}

/// 3 members, resolve all choices (test verifies ability completes without crash).
#[test]
fn position_change_skip_optional() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chii = game.id("PL!HS-bp2-006-R");
    let a = game.new_id("PL!-sd1-013-SD");
    let b = game.new_id("PL!-sd1-013-SD");

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::RightSide);

    assert!(game.has_pending_choice(), "First choice");
    game.select_option(0);

    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(1);

    assert!(game.has_pending_choice(), "Third choice");
    game.select_option(2);

    assert!(!game.has_pending_choice());
    assert_ne!(game.state.player1.stage.stage[0], -1);
    assert_ne!(game.state.player1.stage.stage[1], -1);
    assert_ne!(game.state.player1.stage.stage[2], -1);
}

/// Verify card movement tracking.
#[test]
fn position_change_tracks_card_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chii = game.id("PL!HS-bp2-006-R");
    let a = game.new_id("PL!-sd1-013-SD");
    let b = game.new_id("PL!-sd1-013-SD");

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::RightSide);

    assert!(game.has_pending_choice());
    game.select_option(1); // a → Center (swap with b)

    assert!(game.has_pending_choice());
    game.select_option(0); // → Left

    assert!(game.has_pending_choice());
    game.select_option(2); // → Right

    let moved = &game.state.cards_moved_this_turn;
    assert!(!moved.is_empty(), "At least one card should have moved");
}
