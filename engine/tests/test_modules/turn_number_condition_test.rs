use rabuka_engine::ability::condition::ConditionContext;
use rabuka_engine::card::{CardDatabase, Condition};
use rabuka_engine::core::types::ArcStr;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::player::Player;
use std::sync::Arc;

fn make_game_state(turn: u8, phase: Phase) -> GameState {
    let db = Arc::new(CardDatabase::new());
    let p1 = Player::new("player1".into(), "Player 1".into(), true);
    let p2 = Player::new("player2".into(), "Player 2".into(), false);
    let mut gs = GameState::new(p1, p2, db);
    gs.turn_number = turn;
    gs.current_phase = phase;
    gs
}

fn temporal_condition(turn_number: Option<u8>, text: &str) -> Condition {
    Condition::Temporal {
        text: Some(text.to_string()),
        negation: None,
        phase: Some(ArcStr::from("live_phase")),
        phase_target: None,
        cache: None,
        trigger_event: None,
        temporal: None,
        turn_number,
        count: None,
        location: None,
        card_type: None,
        target: None,
        group_names: None,
        temporal_scope: None,
        position: None,
        locations: None,
        heart_colors: None,
        aggregate: None,
        self_target: None,
        condition: None,
    }
}

/// temporal_condition with turn_number=1 matches turn 1.
#[test]
fn turn_number_1_on_turn_1_true() {
    let gs = make_game_state(1, Phase::LiveCardSetFirstAttacker);
    let ctx = ConditionContext::new(&gs);
    let cond = temporal_condition(Some(1), "このゲームの1ターン目のライブフェイズの場合");
    assert!(ctx.evaluate_condition(&cond));
}

/// temporal_condition with turn_number=1 on turn 2 → false.
#[test]
fn turn_number_1_on_turn_2_false() {
    let gs = make_game_state(2, Phase::LiveCardSetFirstAttacker);
    let ctx = ConditionContext::new(&gs);
    let cond = temporal_condition(Some(1), "このゲームの1ターン目のライブフェイズの場合");
    assert!(!ctx.evaluate_condition(&cond));
}

/// temporal_condition with turn_number=1 on turn 1 but wrong phase → false.
#[test]
fn turn_number_1_on_turn_1_not_live_phase() {
    let gs = make_game_state(1, Phase::Main);
    let ctx = ConditionContext::new(&gs);
    let cond = temporal_condition(Some(1), "このゲームの1ターン目のライブフェイズの場合");
    assert!(!ctx.evaluate_condition(&cond));
}

/// temporal_condition without turn_number → constraint is ignored (backward compat).
#[test]
fn turn_number_none_ignored() {
    let gs = make_game_state(5, Phase::LiveCardSetFirstAttacker);
    let ctx = ConditionContext::new(&gs);
    let cond = temporal_condition(None, "ライブフェイズの場合");
    assert!(ctx.evaluate_condition(&cond));
}
