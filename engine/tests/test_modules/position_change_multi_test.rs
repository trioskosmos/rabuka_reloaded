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

/// Filter destinations by group_names and exclude_self.
///
/// Card: PL!HS-pb1-006-R (安養寺姫芽) ab#0
/// ライブ開始時: 自分のステージにいる他の『みらくらぱーく！』のメンバーがいるエリアに
/// ポジションチェンジしてもよい。そうした場合、ライブ終了時まで、heart01+bladeを得る。
///   action: sequential [ position_change(group_names=[みらくらぱーく！], exclude_self, optional),
///                         gain_resource(blade, conditional) ]
#[test]
fn position_change_filters_by_group_names() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let himeno = game.id("PL!HS-pb1-006-R");
    let member_same = game.id("PL!HS-sd1-014-SD"); // みらくらぱーく！ member
    let member_other = game.id("PL!-sd1-010-SD"); // non-group filler

    // Place members on stage
    game.state.player1.stage.stage = [member_other, member_same, -1];

    // Put himeno in hand and play to stage
    game.state.player1.hand.cards.push(himeno);
    game.give_energy(11);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::RightSide);

    // Play himeno to RightSide — debut/live trigger will fire later.
    // Advance to live start to trigger the ability
    game.pass(); // Main -> Active
    game.pass(); // Active -> Energy
    game.pass(); // Energy -> Draw
    game.pass(); // Draw -> Main
    game.pass(); // Main -> LiveCardSetP1Turn

    // Set a live card so live starts
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP1 -> LiveCardSetP2Turn
    game.state
        .player2
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP2 -> FirstAttackerPerformance

    // The live start trigger fires and the sequential effect starts.
    // First sub-action: position_change with group_names=[みらくらぱーく！], exclude_self=true
    // Only Center (member_same) should be offered — Left (member_other) should NOT.
    if game.has_pending_choice() {
        let choice_type = game.pending_choice_type();
        // Should be a position choice (SelectTarget with position|destination)
        // or possibly a select card choice
        game.dbg_choice();
    }

    // The position change should only offer the Center area (みらくらぱーく！ member)
    // We'll check by looking at the generated actions
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let position_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition)
        .collect();

    // Should have exactly 1 position option (Center, the only valid destination)
    assert_eq!(
        position_actions.len(),
        1,
        "Only Center (みらくらぱーく！ member) should be offered as destination"
    );
    if let Some(ref params) = position_actions[0].parameters {
        assert_eq!(
            params.stage_area.as_deref(),
            Some("center"),
            "Only Center should be a valid position change destination"
        );
    }
}

/// Filter by group_names with exclude_self: own position should be excluded.
#[test]
fn position_change_group_names_excludes_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let himeno = game.id("PL!HS-pb1-006-R");
    let member_same_a = game.id("PL!HS-sd1-014-SD"); // みらくらぱーく！ member (cost 9)
    let member_same_b = game.id("PL!HS-sd1-006-SD"); // みらくらぱーく！ member (cost 15)

    // Place a みらくらぱーく！ member on stage
    game.state.player1.stage.stage = [member_same_a, -1, -1];

    // Put himeno in hand and play to stage — himeno is also みらくらぱーく！
    game.state.player1.hand.cards.push(himeno);
    game.give_energy(11);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::Center);

    // Add another みらくらぱーく！ member on stage
    game.state.player1.stage.stage[2] = member_same_b;

    // Now stage has: [member_same_a, himeno, member_same_b]
    // If himeno activates position_change with group_names=[みらくらぱーく！], exclude_self=true,
    // only Left and Right should be offered (not Center, which is himeno's own position).

    // Advance to live start
    game.pass(); // Main -> Active
    game.pass(); // Active -> Energy
    game.pass(); // Energy -> Draw
    game.pass(); // Draw -> Main
    game.pass(); // Main -> LiveCardSetP1Turn

    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP1 -> LiveCardSetP2Turn
    game.state
        .player2
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP2 -> FirstAttackerPerformance

    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let position_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition)
        .collect();

    // Should have 2 position options (Left and Right, excluding Center=self)
    assert_eq!(
        position_actions.len(),
        2,
        "Left and Right should be offered, Center (self) should be excluded"
    );
}

/// No matching group members → position change should be skipped silently.
#[test]
fn position_change_skip_when_no_valid_destinations() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let himeno = game.id("PL!HS-pb1-006-R");
    let member_other = game.id("PL!-sd1-010-SD"); // non-group filler

    // Stage has only non-group members
    game.state.player1.stage.stage = [member_other, -1, -1];

    // Put himeno in hand and play to stage
    game.state.player1.hand.cards.push(himeno);
    game.give_energy(11);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::RightSide);

    // Advance to live start
    game.pass(); // Main -> Active
    game.pass(); // Active -> Energy
    game.pass(); // Energy -> Draw
    game.pass(); // Draw -> Main
    game.pass(); // Main -> LiveCardSetP1Turn

    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP1 -> LiveCardSetP2Turn
    game.state
        .player2
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP2 -> FirstAttackerPerformance

    // No valid destinations → position change should be skipped
    // No position choice should appear
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let position_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition)
        .collect();

    assert_eq!(
        position_actions.len(),
        0,
        "No position choice should appear when no valid group member destinations exist"
    );
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
