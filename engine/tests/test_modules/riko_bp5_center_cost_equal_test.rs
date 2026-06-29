/// Tests for 桜内梨子 (PL!S-bp5-002-R+) — LiveStart center ability.
///
/// ライブ開始時/センター: 自分のステージの右サイドエリアと左サイドエリアにいる
/// メンバーのコストが同じ場合、相手のステージにいる元々持つブレードの数が
/// 3つ以下のすべてのメンバーをウェイトにする。
///
/// Condition: location_condition, position=left_side, position_compare=right_side,
///            operator="=", require_position_cards=true
/// Effect: change_state(wait, opponent_stage_members_with_blade<=3)
///
/// require_position_cards=true means empty slots DON'T count as cost 0 —
/// both left and right must have members for the equality check to activate.
use crate::helpers::*;

/// Advance from Main (turn 1) to LiveCardSetFirstAttacker.
fn advance_to_live_card_set(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

/// Advance from LiveCardSet to LiveStart (after set_live_card).
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn riko(game: &TestGame) -> i16 {
    game.id("PL!S-bp5-002-R+") // cost=11, blade=2
}

fn aqours_cost4(game: &TestGame) -> i16 {
    game.id("PL!S-bp2-002-R") // cost=4, blade=0
}

fn aqours_cost2(game: &TestGame) -> i16 {
    game.id("PL!S-PR-025-PR") // cost=2, blade=1
}

fn filler_live(game: &TestGame) -> i16 {
    game.id("PL!-sd1-020-SD")
}

fn filler_hand(game: &TestGame) -> i16 {
    game.id("PL!-sd1-010-SD") // blade=1
}

/// Setup: fill decks, give energy, place Riko at center.
fn setup_riko_at_center(game: &mut TestGame) {
    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler_hand(game));
        game.state.player2.main_deck.cards.push(filler_hand(game));
    }
    game.give_energy(15);
    game.state.player1.stage.stage = [-1, riko(game), -1];
}

fn run_live_start(game: &mut TestGame) {
    let live = filler_live(game);
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set(game);
    game.set_live_card(live);
    advance_to_live_start(game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

// ===================================================================
// Positive: both positions occupied with same cost → ability fires
// ===================================================================

#[test]
fn riko_equal_cost_both_occupied_triggers() {
    let mut game = TestGame::new(load_real_database());
    setup_riko_at_center(&mut game);

    game.state.player1.stage.stage[0] = aqours_cost4(&game); // cost=4
    game.state.player1.stage.stage[2] = aqours_cost4(&game); // cost=4

    let opp = filler_hand(&game);
    game.state.player2.stage.stage[1] = opp;

    run_live_start(&mut game);

    assert!(
        game.state.mods.get_orientation_modifier(opp) == Some(&"wait".to_string()),
        "Low-blade opponent member should be wait when costs are equal"
    );
}

// ===================================================================
// Negative: different costs → no trigger
// ===================================================================

#[test]
fn riko_different_costs_no_trigger() {
    let mut game = TestGame::new(load_real_database());
    setup_riko_at_center(&mut game);

    game.state.player1.stage.stage[0] = aqours_cost4(&game); // cost=4
    game.state.player1.stage.stage[2] = aqours_cost2(&game); // cost=2

    let opp = filler_hand(&game);
    game.state.player2.stage.stage[1] = opp;

    run_live_start(&mut game);

    assert!(
        game.state.mods.get_orientation_modifier(opp) != Some(&"wait".to_string()),
        "Should NOT trigger when costs differ"
    );
}

// ===================================================================
// require_position_cards: left empty → no trigger
// ===================================================================

#[test]
fn riko_left_empty_no_trigger() {
    let mut game = TestGame::new(load_real_database());
    setup_riko_at_center(&mut game);

    // Left empty, right has a card
    game.state.player1.stage.stage[2] = aqours_cost4(&game);

    let opp = filler_hand(&game);
    game.state.player2.stage.stage[1] = opp;

    run_live_start(&mut game);

    assert!(
        game.state.mods.get_orientation_modifier(opp) != Some(&"wait".to_string()),
        "Should NOT trigger when left side is empty"
    );
}

// ===================================================================
// require_position_cards: right empty → no trigger
// ===================================================================

#[test]
fn riko_right_empty_no_trigger() {
    let mut game = TestGame::new(load_real_database());
    setup_riko_at_center(&mut game);

    game.state.player1.stage.stage[0] = aqours_cost4(&game);
    // Right empty

    let opp = filler_hand(&game);
    game.state.player2.stage.stage[1] = opp;

    run_live_start(&mut game);

    assert!(
        game.state.mods.get_orientation_modifier(opp) != Some(&"wait".to_string()),
        "Should NOT trigger when right side is empty"
    );
}

// ===================================================================
// require_position_cards: both empty → no trigger
// ===================================================================

#[test]
fn riko_both_empty_no_trigger() {
    let mut game = TestGame::new(load_real_database());
    setup_riko_at_center(&mut game);

    let opp = filler_hand(&game);
    game.state.player2.stage.stage[1] = opp;

    run_live_start(&mut game);

    assert!(
        game.state.mods.get_orientation_modifier(opp) != Some(&"wait".to_string()),
        "Should NOT trigger when both sides are empty"
    );
}

// ===================================================================
// Same cost (cost=2) on both sides → triggers
// ===================================================================

#[test]
fn riko_same_cost_different_cards_triggers() {
    let mut game = TestGame::new(load_real_database());
    setup_riko_at_center(&mut game);

    game.state.player1.stage.stage[0] = aqours_cost2(&game); // cost=2
    game.state.player1.stage.stage[2] = aqours_cost2(&game); // cost=2

    let opp = filler_hand(&game);
    game.state.player2.stage.stage[1] = opp;

    run_live_start(&mut game);

    assert!(
        game.state.mods.get_orientation_modifier(opp) == Some(&"wait".to_string()),
        "Should trigger when both cost=2"
    );
}
