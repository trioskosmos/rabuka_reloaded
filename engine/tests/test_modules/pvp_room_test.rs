use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::web_server::pvp_player_can_act;
use std::sync::Arc;

use crate::helpers::*;

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

fn fill_both_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn both_players_multiple_live_start_abilities_get_correct_choice_routing() {
    use rabuka_engine::ability::types::Choice;

    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");

    // Get 3 copies of Shizuku for each player
    let p1_a = game.id("PL!N-bp3-015-N");
    let p1_b = game.id("PL!N-bp3-015-N");
    let p1_c = game.id("PL!N-bp3-015-N");
    let p2_a = game.id("PL!N-bp3-015-N");
    let p2_b = game.id("PL!N-bp3-015-N");
    let p2_c = game.id("PL!N-bp3-015-N");

    // Set up both players' stages with 3x Shizuku
    game.state.player1.stage.stage = [p1_a, p1_b, p1_c];
    game.state.player2.stage.stage = [p2_a, p2_b, p2_c];

    fill_both_decks(&mut game, filler);

    // Advance through turns to reach FirstAttackerPerformance (triggers LiveStart)
    // 7 passes: Main → Active → Energy → Draw → Main → LiveCardSetFirst → LiveCardSetSecond → FirstAttackerPerformance
    for _ in 0..7 {
        game.pass();
    }

    // After LiveCardSetSecondAttacker → FirstAttackerPerformance transition,
    // LiveStart abilities for both players are triggered and processed.
    // P1 (first attacker) should have SelectAutoAbility for their 3 Shizuku.
    assert!(
        game.state.has_pending_choice(),
        "Should have pending choice after LiveStart triggers"
    );

    // Verify P1's SelectAutoAbility routing
    {
        let choice = game.state.get_pending_choice().unwrap();
        match choice {
            Choice::SelectAutoAbility {
                player_id, options, ..
            } => {
                assert_eq!(player_id, "p1", "First SelectAutoAbility must be for P1");
                assert_eq!(options.len(), 3, "P1 should have 3 ability options");
                assert!(
                    pvp_player_can_act(&game.state, 0),
                    "P1 must be able to act on their own SelectAutoAbility"
                );
                assert!(
                    !pvp_player_can_act(&game.state, 1),
                    "P2 must NOT be able to act on P1's SelectAutoAbility"
                );
            }
            _ => panic!("Expected SelectAutoAbility for P1, got {:?}", choice),
        }
    }

    // Verify choice_player_id is in the JSON for P1
    {
        let json = game.state.get_pending_choice_json().unwrap();
        let cpid = json.get("choice_player_id").and_then(|v| v.as_str());
        assert_eq!(
            cpid,
            Some("p1"),
            "choice_player_id must be 'p1' in JSON for P1's choice"
        );
    }

    // Drain P1's 3 abilities:
    //   - When >1 options: SelectAutoAbility → pick 0 → ability runs → SelectHeartColor → pick 0
    //   - When only 1 option remains, engine auto-starts it (no SelectAutoAbility), goes directly to SelectHeartColor
    for i in 0..2 {
        // First two: 3 options then 2 options → engine shows SelectAutoAbility
        assert_eq!(
            game.pending_choice_type(),
            Some("SelectAutoAbility".to_string()),
            "Iteration {} of P1: expected SelectAutoAbility",
            i
        );
        let choice = game.state.get_pending_choice().unwrap();
        match choice {
            Choice::SelectAutoAbility { player_id, .. } => {
                assert_eq!(player_id, "p1", "Iteration {}: still P1's choice", i);
            }
            _ => {}
        }
        game.select_option(0);

        assert_eq!(
            game.pending_choice_type(),
            Some("SelectHeartColor".to_string()),
            "Iteration {} of P1: expected SelectHeartColor after picking ability",
            i
        );
        game.select_option(0);
    }

    // Third ability: only 1 option left → engine auto-starts, goes directly to SelectHeartColor
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectHeartColor".to_string()),
        "Third P1 ability: expected SelectHeartColor (last option auto-started)"
    );
    game.select_option(0);

    // Now P2 should have their SelectAutoAbility
    assert!(
        game.state.has_pending_choice(),
        "P2 should have pending choice after P1's abilities are done"
    );

    let choice = game.state.get_pending_choice().unwrap();
    match choice {
        Choice::SelectAutoAbility {
            player_id, options, ..
        } => {
            assert_eq!(player_id, "p2", "Second SelectAutoAbility must be for P2");
            assert_eq!(options.len(), 3, "P2 should have 3 ability options");
            assert!(
                !pvp_player_can_act(&game.state, 0),
                "P1 must NOT be able to act on P2's SelectAutoAbility"
            );
            assert!(
                pvp_player_can_act(&game.state, 1),
                "P2 must be able to act on their own SelectAutoAbility"
            );
        }
        _ => panic!("Expected SelectAutoAbility for P2, got {:?}", choice),
    }

    // Verify choice_player_id is in the JSON for P2
    let json = game.state.get_pending_choice_json().unwrap();
    let cpid = json.get("choice_player_id").and_then(|v| v.as_str());
    assert_eq!(
        cpid,
        Some("p2"),
        "choice_player_id must be 'p2' in JSON for P2's choice"
    );
}
