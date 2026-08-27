use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!S-bp6-015 / PL!SP-pb2-024 — 登場: opponent stage cost≤2 member → wait
#[test]
fn s_bp6_015_wait_opponent_cost2_succeeds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp6-015-N"); // cost 2? The card itself is cost 2? Actually the ability's cost_limit is 2, so it targets opponent cost ≤2
    let opp_low2 = game.id("PL!N-bp7-017-N"); // cost 2
    game.state.player2.stage.stage = [opp_low2, -1, -1];
    game.state.player1.hand.cards.push(yoshiko);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(yoshiko, MemberArea::Center);
    // The Debut should have waited the opponent's cost2 member
    let waited = game.state.mods.get_orientation_modifier(opp_low2);
    assert_eq!(waited.as_deref(), Some("wait"), "opponent cost2 should be waited");
}

#[test]
fn s_bp6_015_wait_opponent_cost_high_no_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp6-015-N");
    let opp_high = game.id("PL!-sd1-003-SD"); // cost 13 (>2)
    game.state.player2.stage.stage = [opp_high, -1, -1];
    game.state.player1.hand.cards.push(yoshiko);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(yoshiko, MemberArea::Center);
    let waited = game.state.mods.get_orientation_modifier(opp_high);
    assert!(waited.is_none() || waited.as_deref() != Some("wait"), "cost13 should NOT be waited");
}

#[test]
fn s_bp6_015_wait_opponent_empty_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp6-015-N");
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(yoshiko);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(yoshiko, MemberArea::Center);
    assert!(!game.has_pending_choice(), "empty opponent should have no pending");
}

#[test]
fn s_bp6_015_wait_opponent_multiple_cost2_choose_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp6-015-N");
    let opp1 = game.id("PL!N-bp7-017-N"); // cost2
    let opp2 = game.new_id("PL!N-bp7-017-N"); // another cost2
    game.state.player2.stage.stage = [opp1, opp2, -1];
    game.state.player1.hand.cards.push(yoshiko);
    game.give_energy(15);
    for _ in 0..5 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    game.play_to_stage(yoshiko, MemberArea::Center);
    // Debut should present a choice to pick which cost2 to wait (if multiple)
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let waited1 = game.state.mods.get_orientation_modifier(opp1);
    let waited2 = game.state.mods.get_orientation_modifier(opp2);
    let waited_count = [waited1, waited2].iter().filter(|m| m.as_deref()==Some("wait")).count();
    assert_eq!(waited_count, 1, "exactly one of the two cost2 should be waited");
}
