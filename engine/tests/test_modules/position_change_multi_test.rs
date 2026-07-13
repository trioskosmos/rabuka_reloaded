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

    // All 3 positions occupied → 3 sequential choices, each with
    // reduced options as zones are claimed (formation change).
    assert!(game.has_pending_choice(), "First choice");
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 3, "First: all 3 zones available");
    game.select_generated(1); // a → Center

    assert!(game.has_pending_choice(), "Second choice");
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 2, "Second: 2 zones remain after first pick");
    game.select_generated(1); // → Right

    assert!(game.has_pending_choice(), "Third choice");
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 1, "Third: last remaining zone");
    game.select_generated(0); // → Left

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
/// Options should show left, center, right (empty slots are valid destinations).
/// This test has NO group_names filter, so empty slots are valid.
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
    // chii is the only member at center. All 3 positions are valid destinations
    // (empty slots included). Source card's position (center) is not excluded
    // when exclude_self=false.
    let actions = game.generated_actions();
    assert_eq!(
        actions.len(),
        3,
        "Should show 3 position options (left, center, right)"
    );
    // Verify the stage areas of all 3 options
    assert_eq!(
        actions[0]
            .parameters
            .as_ref()
            .unwrap()
            .stage_area
            .as_deref(),
        Some("left")
    );
    assert_eq!(
        actions[1]
            .parameters
            .as_ref()
            .unwrap()
            .stage_area
            .as_deref(),
        Some("center")
    );
    assert_eq!(
        actions[2]
            .parameters
            .as_ref()
            .unwrap()
            .stage_area
            .as_deref(),
        Some("right")
    );
    game.select_generated(0); // move to Left

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
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 3, "First: all 3 zones available");
    game.select_generated(2); // Move to Right

    assert!(game.has_pending_choice(), "Second choice");
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 2, "Second: 2 zones remain after first pick");
    game.select_generated(0); // Move to Left

    assert!(game.has_pending_choice(), "Third choice");
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 1, "Third: last remaining zone");
    game.select_generated(0); // Move to Center

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
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::RightSide);

    // Play himeno to RightSide — debut/live trigger will fire later.
    // Advance to live start to trigger the ability
    game.pass(); // Main -> Active
    game.pass(); // Active -> Energy
    game.pass(); // Energy -> Draw
    game.pass(); // Draw -> Main
    game.pass(); // Main -> LiveCardSetFirstAttacker

    // Set a live card so live starts
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP1 -> LiveCardSetSecondAttacker
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
        let _choice_type = game.pending_choice_type();
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
    // Left slot has member_other (non-group) → excluded. Right slot was where
    // himeno played but exclude_self excludes her own position. Center has
    // member_same (みらくらぱーく！) → valid. Empty slots are excluded here
    // because this is a single position change (not multiple_targets).
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
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
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
    game.pass(); // Main -> LiveCardSetFirstAttacker

    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP1 -> LiveCardSetSecondAttacker
    game.state
        .player2
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP2 -> FirstAttackerPerformance
    game.drain_auto_ability_choices();

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
    // Verify specific position labels
    let offered: Vec<&str> = position_actions
        .iter()
        .filter_map(|a| a.parameters.as_ref()?.stage_area.as_deref())
        .collect();
    assert!(offered.contains(&"left"), "Left should be offered");
    assert!(offered.contains(&"right"), "Right should be offered");
    assert!(
        !offered.contains(&"center"),
        "Center (self) should be excluded"
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
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::RightSide);

    // Advance to live start
    game.pass(); // Main -> Active
    game.pass(); // Active -> Energy
    game.pass(); // Energy -> Draw
    game.pass(); // Draw -> Main
    game.pass(); // Main -> LiveCardSetFirstAttacker

    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP1 -> LiveCardSetSecondAttacker
    game.state
        .player2
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.pass(); // LiveCardSetP2 -> FirstAttackerPerformance

    // No valid destinations → position change should be skipped silently.
    // This is the expected behavior for single position changes (no multiple_targets):
    // empty slots are excluded because the effect says "move to ANOTHER member's
    // area", not "move to any area". Left has non-group → excluded, Center is
    // empty → excluded (non-formation change), Right is self → excluded.
    // Total=0 valid destinations → no choice presented.
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
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 3, "First: all 3 zones available");
    game.select_generated(1); // a → Center

    assert!(game.has_pending_choice());
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 2, "Second: 2 zones remain");
    game.select_generated(0); // → Left

    assert!(game.has_pending_choice());
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 1, "Third: last remaining zone");
    game.select_generated(0); // → Right

    let moved = &game.state.cards_moved_this_turn;
    assert!(!moved.is_empty(), "At least one card should have moved");
}

/// Formation change (multiple_targets=true) with group_names and empty slots.
///
/// Card: PL!SP-bp4-027-L (Chance Day, Chance Way!) ab#0
/// ライブ成功時: 自分のステージにいるメンバーが『Liella!』のみの場合、
/// 自分のステージにいるメンバーをフォーメーションチェンジしてもよい。
///   action: position_change, multiple_targets: true, optional: true,
///            group_names: ["Liella!"]
///
/// Before the fix, empty slots were excluded when group_names was set,
/// so with 1 Liella! member in center and empty left/right, only center
/// would be offered (no-op). After the fix, all 3 positions should be
/// offered because this is a formation change (multiple_targets=true).
#[test]
fn formation_change_with_group_names_and_empty_slots() {
    use rabuka_engine::card::{HeartColor, HeartMap};
    use rabuka_engine::turn::TurnEngine;

    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chance_day = game.id("PL!SP-bp4-027-L");
    let liella_member = game.id("PL!SP-bp1-014-N"); // 唐 可可, Liella!
    let filler = game.id("PL!-sd1-010-SD");

    // Fill decks
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // 1 Liella! member in center, left and right empty
    game.state.player1.stage.stage = [-1, liella_member, -1];

    // Put Chance Day in live card zone
    game.state.player1.live_card_zone.cards.push(chance_day);

    // Set stage hearts to satisfy Chance Day's need_heart: heart02=1, heart0=2
    let mut heart_map = HeartMap::new();
    heart_map.insert(HeartColor::Heart02, 1);
    heart_map.insert(HeartColor::Heart00, 2);
    let hearts = rabuka_engine::card::BaseHeart { hearts: heart_map };
    game.state.player1.stage_hearts = Some(hearts);

    let player_id = game.state.player1.id.clone();

    // Set phase for LiveVictoryDetermination (required by should_trigger_live_success)
    game.state.current_phase = rabuka_engine::game_state::Phase::LiveVictoryDetermination;

    // Trigger live success abilities — this fires Chance Day's ability.
    // The group_condition checks ALL members on stage are Liella! (true:
    // the only member is 唐 可可 from Liella!). Then the position_change
    // effect fires with multiple_targets=true and group_names=["Liella!"].
    game.state.live_success_triggered_this_turn = false;
    TurnEngine::trigger_live_success_abilities(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);

    // Should have a position change choice now
    assert!(
        game.has_pending_choice(),
        "Position change choice should be presented"
    );

    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let position_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition)
        .collect();

    // After the fix: all 3 positions should be valid destinations even though
    // group_names=["Liella!"] is set, because this is a formation change
    // (multiple_targets=true). Empty slots are valid destinations for
    // formation changes — you can move the Liella! member to any area.
    assert_eq!(
        position_actions.len(),
        3,
        "All 3 positions should be offered as destinations for formation change with group_names"
    );

    // Verify all position labels
    let offered: Vec<&str> = position_actions
        .iter()
        .filter_map(|a| a.parameters.as_ref()?.stage_area.as_deref())
        .collect();
    assert!(
        offered.contains(&"left"),
        "Left should be offered (empty slot)"
    );
    assert!(
        offered.contains(&"center"),
        "Center should be offered (Liella! member)"
    );
    assert!(
        offered.contains(&"right"),
        "Right should be offered (empty slot)"
    );
}
