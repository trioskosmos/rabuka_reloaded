use crate::helpers::*;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;

const HIMENO: &str = "PL!HS-pb1-014-R";
const FILLER: &str = "PL!-sd1-010-SD";

/// Debut: position change an opponent member to the front area of this member.
/// Himeno at center, opponent has 3 members.
/// Front of center = center → selected left member swaps to opponent center.
#[test]
fn himeno_front_3_opponents_select_left_moves_to_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp_left = game.id(FILLER);
    let opp_center = game.id(FILLER);
    let opp_right = game.id(FILLER);

    game.state.player2.stage.stage = [opp_left, opp_center, opp_right];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_option(0);
    }

    assert_eq!(game.state.player2.stage.stage[1], opp_left);
}

/// Himeno at center, opponent has 1 member at left.
#[test]
fn himeno_front_1_opponent_at_left_moves_to_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp = game.id(FILLER);

    game.state.player2.stage.stage[0] = opp;
    game.state.player2.stage.stage[1] = -1;
    game.state.player2.stage.stage[2] = -1;
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_option(0);
    }

    assert_eq!(game.state.player2.stage.stage[1], opp);
    assert_eq!(game.state.player2.stage.stage[0], -1);
}

/// Himeno on left. Front of left = right (mirrored). Select opponent center → moves to right.
#[test]
fn himeno_front_on_left_opponent_moves_to_right() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp_center = game.id(FILLER);
    let opp_right = game.id(FILLER);

    game.state.player2.stage.stage[0] = -1;
    game.state.player2.stage.stage[1] = opp_center;
    game.state.player2.stage.stage[2] = opp_right;
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::LeftSide);

    while game.has_pending_choice() {
        game.select_option(0);
    }

    assert_eq!(game.state.player2.stage.stage[2], opp_center);
    assert_eq!(game.state.player2.stage.stage[1], opp_right);
}

/// Himeno on right. Front of right = left (mirrored). Select opponent center → moves to left.
#[test]
fn himeno_front_on_right_opponent_moves_to_left() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp_center = game.id(FILLER);
    let opp_left = game.id(FILLER);

    game.state.player2.stage.stage = [opp_left, opp_center, -1];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::RightSide);

    while game.has_pending_choice() {
        // First option is "left" (index 0), which IS the front of right.
        // Select "center" (index 1) as the source instead.
        game.select_option(1);
    }

    assert_eq!(game.state.player2.stage.stage[0], opp_center);
}

/// No opponent members → no valid source → ability does nothing.
#[test]
fn himeno_front_no_opponent_skill_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);

    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.is_first_attacker = true;
    game.state.current_phase = Phase::Main;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 1;

    game.state.player1.hand.cards.push(himeno);
    game.give_energy(9);
    game.play_to_stage(himeno, MemberArea::Center);

    assert!(
        !game.has_pending_choice(),
        "No choice when no opponent target"
    );
}

/// Constant ability (ab#1): gain heart01 when front opponent has higher cost.
#[test]
fn himeno_front_constant_evaluates() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp = game.id(FILLER);

    game.state.player1.stage.stage[1] = himeno;
    game.state.player2.stage.stage[1] = opp;
    game.state.recalculate_constants();
}

/// No opponent in front → no heart01.
#[test]
fn himeno_front_constant_no_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);

    game.state.player1.stage.stage[1] = himeno;
    game.state.player2.stage.stage[1] = -1;
    game.state.recalculate_constants();

    let m = game
        .state
        .mods
        .get_heart_modifier(himeno, rabuka_engine::zones::parse_heart_color("heart01"));
    assert_eq!(m, 0);
}

/// Opponent at side not front → no heart01.
#[test]
fn himeno_front_constant_opponent_at_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let himeno = game.id(HIMENO);
    let opp = game.id(FILLER);

    game.state.player1.stage.stage[1] = himeno;
    game.state.player2.stage.stage[0] = opp;
    game.state.player2.stage.stage[1] = -1;
    game.state.recalculate_constants();

    let m = game
        .state
        .mods
        .get_heart_modifier(himeno, rabuka_engine::zones::parse_heart_color("heart01"));
    assert_eq!(m, 0);
}
