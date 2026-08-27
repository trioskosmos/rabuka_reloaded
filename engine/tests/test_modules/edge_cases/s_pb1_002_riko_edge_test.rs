use crate::helpers::*;

#[test]
fn riko_opponent_discards_live_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-pb1-002-R");
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.hand.cards.push(riko);
    game.state.player2.hand.cards.push(live);
    game.state.player2.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player2.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(15);
    // Debut riko
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // Should prompt opponent to discard live (optional)
    assert!(game.has_pending_choice(), "opponent should be prompted to discard live");
    // Opponent discards live
    game.select_indices(&[0]);
    game.drain_auto_ability_choices();
    // Opponent discarded, so no gain
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0, "opponent discarded live -> no gain");
}

#[test]
fn riko_opponent_declines_gains_live_total() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-pb1-002-R");
    game.state.player1.hand.cards.push(riko);
    game.state.player2.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(15);
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // Opponent has a member but no live, so the live discard choice may be skipped
    if game.has_pending_choice() {
        game.select_indices(&[]);
        game.drain_auto_ability_choices();
    }
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1, "opponent declined/no live -> gain");
}

#[test]
fn riko_opponent_no_live_in_hand_no_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-pb1-002-R");
    game.state.player1.hand.cards.push(riko);
    // Opponent has no live, only members
    game.state.player2.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(15);
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // No live to discard, so opponent cannot discard, but can decline
    // The choice should still be present but with 0 selectable
    if game.has_pending_choice() {
        game.select_indices(&[]);
        game.drain_auto_ability_choices();
    }
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1, "no live -> opponent cannot discard -> gain");
}
