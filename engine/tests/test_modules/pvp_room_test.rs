use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::web_server::pvp_player_can_act;
use std::sync::Arc;

fn make_db() -> Arc<CardDatabase> {
    let cards_path = std::path::Path::new("../cards/cards.json");
    match CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => Arc::new(CardDatabase::load_or_create(cards)),
        Err(_) => Arc::new(CardDatabase::new()),
    }
}

fn make_gs(phase: Phase) -> GameState {
    let db = make_db();
    let p1 = Player::new("0".to_string(), "P1".to_string(), true);
    let p2 = Player::new("1".to_string(), "P2".to_string(), false);
    let mut gs = GameState::new(p1, p2, db);
    gs.current_phase = phase;
    gs
}

#[test]
fn rps_both_can_act() {
    let gs = make_gs(Phase::RockPaperScissors);
    assert!(pvp_player_can_act(&gs, 0));
    assert!(pvp_player_can_act(&gs, 1));
}

#[test]
fn rps_p1_chooses_then_cannot() {
    let mut gs = make_gs(Phase::RockPaperScissors);
    gs.player1_rps_choice = Some(0);
    assert!(!pvp_player_can_act(&gs, 0));
    assert!(pvp_player_can_act(&gs, 1));
}

#[test]
fn rps_both_choose_then_neither_can() {
    let mut gs = make_gs(Phase::RockPaperScissors);
    gs.player1_rps_choice = Some(0);
    gs.player2_rps_choice = Some(1);
    assert!(!pvp_player_can_act(&gs, 0));
    assert!(!pvp_player_can_act(&gs, 1));
}

#[test]
fn choose_first_attacker_winner_only() {
    let mut gs = make_gs(Phase::ChooseFirstAttacker);
    gs.rps_winner = Some(1); // 1 = P1 wins
    assert!(pvp_player_can_act(&gs, 0));
    assert!(!pvp_player_can_act(&gs, 1));
}

#[test]
fn mulligan_first_attacker() {
    let mut gs = make_gs(Phase::MulliganFirstAttacker);
    gs.player1.is_first_attacker = true;
    gs.player2.is_first_attacker = false;
    assert!(pvp_player_can_act(&gs, 0));
    assert!(!pvp_player_can_act(&gs, 1));
}

#[test]
fn mulligan_second_attacker() {
    let mut gs = make_gs(Phase::MulliganSecondAttacker);
    gs.player1.is_first_attacker = true;
    gs.player2.is_first_attacker = false;
    assert!(!pvp_player_can_act(&gs, 0));
    assert!(pvp_player_can_act(&gs, 1));
}

#[test]
fn main_phase_first_attacker_only() {
    let mut gs = make_gs(Phase::Main);
    gs.player1.is_first_attacker = true;
    gs.current_turn_phase = TurnPhase::FirstAttackerNormal;
    assert!(pvp_player_can_act(&gs, 0));
    assert!(!pvp_player_can_act(&gs, 1));
}

#[test]
fn main_phase_second_attacker_only() {
    let mut gs = make_gs(Phase::Main);
    gs.player1.is_first_attacker = false; // P1 = second attacker
    gs.player2.is_first_attacker = true;
    gs.current_turn_phase = TurnPhase::SecondAttackerNormal;
    assert!(
        pvp_player_can_act(&gs, 0),
        "P1 (second attacker) should act"
    );
    assert!(
        !pvp_player_can_act(&gs, 1),
        "P2 (first attacker) should wait"
    );
}

#[test]
fn live_card_set_first_attacker() {
    let mut gs = make_gs(Phase::LiveCardSetFirstAttacker);
    gs.player1.is_first_attacker = true;
    assert!(pvp_player_can_act(&gs, 0));
    assert!(!pvp_player_can_act(&gs, 1));
}

#[test]
fn live_card_set_second_attacker() {
    let mut gs = make_gs(Phase::LiveCardSetSecondAttacker);
    gs.player1.is_first_attacker = false; // P1 = second attacker
    gs.player2.is_first_attacker = true;
    assert!(
        pvp_player_can_act(&gs, 0),
        "P1 (second attacker) should act"
    );
    assert!(
        !pvp_player_can_act(&gs, 1),
        "P2 (first attacker) should wait"
    );
}

#[test]
fn opponent_choice_routed_via_choice_player_id_works_in_pvp() {
    use rabuka_engine::ability::types::Choice;

    let mut gs = make_gs(Phase::Main);
    gs.player1.is_first_attacker = true;
    gs.player2.is_first_attacker = false;

    // Inject a SelectCard choice with choice_player_id set to opponent (P2)
    let choice = Choice::select_cards("hand", 1, "Select card", false)
        .target_player_id(Some("opponent".to_string()))
        .build();
    gs.ability_queue.pause_for_choice(choice);
    if let Some(entry) = gs.ability_queue.current_entry_mut() {
        entry.choice_player_id = Some("p2".to_string());
    }

    // P2 should be allowed to act (choice is routed to them)
    assert!(pvp_player_can_act(&gs, 1), "P2 can act on own choice");
    // P1 should be blocked (not their choice, even though they're active player)
    assert!(!pvp_player_can_act(&gs, 0), "P1 blocked from P2's choice");
}

#[test]
fn opponent_choice_via_select_auto_ability_works_in_pvp() {
    use rabuka_engine::ability::types::Choice;

    let mut gs = make_gs(Phase::Main);
    gs.player1.is_first_attacker = true;
    gs.player2.is_first_attacker = false;

    // Inject a SelectAutoAbility choice routed to P2
    let choice = Choice::SelectAutoAbility {
        player_id: "p2".to_string(),
        options: vec![],
        description: "Choose auto ability".to_string(),
    };
    // Use pause_for_auto_ability_choice (stores in state, no queue entry)
    gs.ability_queue.pause_for_auto_ability_choice(choice);

    // P2 should be allowed to act (auto-ability choice is for them)
    assert!(pvp_player_can_act(&gs, 1), "P2 can act on own auto ability");
    // P1 should be blocked (not their auto ability, even though active player)
    assert!(
        !pvp_player_can_act(&gs, 0),
        "P1 blocked from P2's auto ability"
    );
}

#[test]
fn normal_choice_stays_with_active_player_in_pvp() {
    use rabuka_engine::ability::types::Choice;

    let mut gs = make_gs(Phase::Main);
    gs.player1.is_first_attacker = true;
    gs.player2.is_first_attacker = false;

    // Inject a normal choice (no opponent routing) — should default to P1
    let choice = Choice::select_cards("hand", 1, "Select card", false)
        .target_player_id(Some("self".to_string()))
        .build();
    gs.ability_queue.pause_for_choice(choice);

    // P1 (active player) should be allowed
    assert!(pvp_player_can_act(&gs, 0), "P1 can act on own choice");
    // P2 should be blocked (not active player, choice not routed to them)
    assert!(!pvp_player_can_act(&gs, 1), "P2 blocked from P1's choice");
}
