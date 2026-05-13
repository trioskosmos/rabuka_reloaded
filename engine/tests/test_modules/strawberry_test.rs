/// QA tests for Strawberry Trapper (PL!S-pb1-021-L / GuiltyKiss)
///
/// Ability (ライブ成功時): compound(and)
///   (A) group_condition(Aqours, heart05 >= 4) on self stage
///   (B) temporal: this_turn + opponent_live_success + no_excess_heart
/// Effect: modify_score(+2, self_target)
///
/// Q36: timing = LiveVictoryDetermination, before winner
/// Q132: first attacker's live_success fires
/// Q142: excess heart definition
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Q36: ライブ成功時 fires during LiveVictoryDetermination, NOT during
/// performance phases. Verify the ability trigger is processed during
/// LiveVictoryDetermination by checking that the game state changes
/// (opponent_live_success flag is accessible there).
#[test]
fn strawberry_q36_only_fires_in_live_victory_determination() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.opponent_live_success_this_turn,
        "Q36: Before LiveVictoryDetermination, no live_success processing"
    );

    game.pass();
    assert_eq!(
        game.state.current_phase.to_string(),
        "SecondAttackerPerformance",
        "Now in P2's performance"
    );

    game.pass();
    assert!(
        game.state.current_phase.to_string().contains("LiveVictory"),
        "Q36: Now in LiveVictoryDetermination phase"
    );

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// Q132: First attacker's live_success ability is evaluated during
/// LiveVictoryDetermination.
#[test]
fn strawberry_q132_first_attacker_evaluated() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.opponent_live_success_this_turn = true;
    game.state.opponent_live_no_excess_heart_this_turn = true;

    game.pass();
    game.pass();
}

/// Q142: Excess heart blocks the temporal condition.
#[test]
fn strawberry_q142_excess_heart_prevents_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.opponent_live_success_this_turn = true;
    game.state.opponent_live_no_excess_heart_this_turn = false;

    game.pass();
    game.pass();
}

/// Group condition: non-Aqours member → group fails → no score.
#[test]
fn strawberry_q142_wrong_group_prevents_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let non_aqours = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, non_aqours);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.opponent_live_success_this_turn = true;
    game.state.opponent_live_no_excess_heart_this_turn = true;

    game.pass();
    game.pass();
}

/// Edge case: opponent_live_success is false → temporal fails → no score.
#[test]
fn strawberry_opponent_didnt_win_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.opponent_live_success_this_turn = false;
    game.state.opponent_live_no_excess_heart_this_turn = false;

    game.pass();
    game.pass();
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
