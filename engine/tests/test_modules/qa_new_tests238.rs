/// Tests for Q238 — 大沢琉璃乃 PL!HS-bp5-003-AR/P/R+/SEC
///
/// Auto ability (ab#0):
///   このメンバーがステージから控え室に置かれたとき、メンバー1人を
///   ポジションチェンジさせてもよい。
///
/// Q238: Can this ability target an opponent's member card?
/// Answer: Yes — the text says "メンバー1人" without "自分の" qualifier,
/// so any member on either player's stage is a valid target.
use crate::helpers::*;

const OSAWA_RINO: &str = "PL!HS-bp5-003-AR";

/// Manually trigger Rino's auto ability (stage → discard zone change).
fn trigger_rino_auto(game: &mut TestGame, rino: i16) {
    game.state.recently_moved_cards = Some(vec![rino].into());
    game.state.recently_moved_from_zone = Some("stage".to_string());
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
}

/// Helper: count how many position choices are generated for the pending choice.
fn count_position_actions(game: &TestGame) -> usize {
    let all = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    all.iter()
        .filter(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition)
        .count()
}

/// Q238 main: reposition opponent member.
#[test]
fn q238_reposition_opponent_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rino = game.id(OSAWA_RINO);
    let opp_member = game.new_id("PL!-sd1-010-SD");

    // Place Rino on stage and opponent member
    game.state.player1.stage.stage = [-1, rino, -1];
    game.state.player2.stage.stage = [-1, opp_member, -1];

    // Manually trigger stage→discard for Rino
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(rino);
    trigger_rino_auto(&mut game, rino);

    // Should have a pending choice (optional position_change)
    assert!(
        game.has_pending_choice(),
        "Rino's auto should fire on stage→discard"
    );

    // Options should include opponent:center (the only member on stage)
    let actions = count_position_actions(&game);
    assert_eq!(actions, 1, "Only opponent:center should be available");

    // Select the opponent member
    game.select_generated(0);

    // Should now ask for destination
    assert!(game.has_pending_choice(), "Should prompt for destination");
    let dest_actions = count_position_actions(&game);
    assert!(dest_actions > 0, "Should have destination options");

    // Move opponent member to left
    game.select_generated(0);

    assert!(
        !game.has_pending_choice(),
        "No more choices after reposition"
    );
    assert_eq!(
        game.state.player2.stage.stage[0], opp_member,
        "Opponent member moved to left"
    );
    assert_eq!(
        game.state.player2.stage.stage[1], -1,
        "Opponent center now empty"
    );
}

/// Reposition own member (basic case).
#[test]
fn q238_reposition_own_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rino = game.id(OSAWA_RINO);
    let own_member = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [own_member, rino, -1];
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(rino);
    trigger_rino_auto(&mut game, rino);

    assert!(game.has_pending_choice(), "Rino's auto should fire");

    // Should offer self:left only
    let actions = count_position_actions(&game);
    assert_eq!(actions, 1, "Should offer self:left");

    game.select_generated(0);

    assert!(game.has_pending_choice(), "Should prompt for destination");
    game.select_generated(0); // move to whatever destination

    assert!(!game.has_pending_choice(), "No more choices");
}

/// No other members on either stage → auto ability skips gracefully.
#[test]
fn q238_no_other_members_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rino = game.id(OSAWA_RINO);

    // Rino is the only member on an otherwise empty stage
    game.state.player1.stage.stage = [-1, rino, -1];
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(rino);
    trigger_rino_auto(&mut game, rino);

    // After moving to discard, no other members on either stage
    // → valid_sources is empty → ability should skip
    assert!(
        !game.has_pending_choice(),
        "No valid targets → auto ability should skip"
    );
}

/// Both players have members → all are selectable.
#[test]
fn q238_both_players_members_all_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rino = game.id(OSAWA_RINO);
    let own_left = game.new_id("PL!-sd1-010-SD");
    let opp_center = game.new_id("PL!-sd1-010-SD");
    let opp_right = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [own_left, rino, -1];
    game.state.player2.stage.stage = [-1, opp_center, opp_right];
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(rino);
    trigger_rino_auto(&mut game, rino);

    assert!(game.has_pending_choice(), "Rino's auto should fire");

    // Should offer: self:left, opponent:center, opponent:right
    let actions = count_position_actions(&game);
    assert_eq!(actions, 3, "Should offer 3 members total");
}

/// Select opponent center from two opponent members.
#[test]
fn q238_select_opponent_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rino = game.id(OSAWA_RINO);
    let opp_left = game.new_id("PL!-sd1-010-SD");
    let opp_center = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, rino, -1];
    game.state.player2.stage.stage = [opp_left, opp_center, -1];
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(rino);
    trigger_rino_auto(&mut game, rino);

    assert!(game.has_pending_choice(), "Rino's auto should fire");
    let actions = count_position_actions(&game);
    assert_eq!(actions, 2, "Both opponent members offered");

    // Select opponent:center (index 1)
    game.select_generated(1);

    assert!(game.has_pending_choice(), "Destination prompt");
    game.select_generated(0); // move to left

    assert!(!game.has_pending_choice(), "Done");
    assert_eq!(
        game.state.player2.stage.stage[0], opp_center,
        "Center member moved to left"
    );
    assert_eq!(
        game.state.player2.stage.stage[1], opp_left,
        "Left member stayed in place"
    );
}

/// Player may decline the optional reposition.
#[test]
fn q238_optional_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rino = game.id(OSAWA_RINO);
    let opp_member = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, rino, -1];
    game.state.player2.stage.stage = [-1, opp_member, -1];
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(rino);
    trigger_rino_auto(&mut game, rino);

    assert!(game.has_pending_choice(), "Rino's auto should fire");

    // Skip the optional reposition
    rabuka_engine::turn::TurnEngine::resume_with_choice(&mut game.state, Some(-1), None)
        .expect("skip should succeed");

    assert!(!game.has_pending_choice(), "No more choices after skip");
    // Opponent member should remain in place
    assert_eq!(
        game.state.player2.stage.stage[1], opp_member,
        "Opponent member unchanged after skip"
    );
}
