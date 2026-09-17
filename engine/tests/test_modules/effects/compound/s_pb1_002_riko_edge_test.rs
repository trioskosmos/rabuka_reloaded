use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn opponent_discards_live_to_waitroom_preventing_total_score_gain() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-pb1-002-R");
    let live = game.id("PL!-sd1-019-SD");
    let member = game.id("PL!-sd1-010-SD");
    let other_member = game.id("PL!-sd1-011-SD");
    game.state.player1.hand.cards.push(source);
    game.state.player2.hand.cards.extend([member, live, other_member]);
    game.give_energy(15);
    game.play_to_stage(source, MemberArea::Center);
    game.drain_auto_ability_choices();
    game.assert_select_card("hand", 1, true);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
    game.select_indices(&[0]);
    game.drain_auto_ability_choices();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player2.waitroom.cards.as_slice(), &[live]);
    assert_eq!(game.state.player2.hand.cards.as_slice(), &[member, other_member]);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
}

#[test]
fn opponent_declines_eligible_live_discard_preserving_hand_and_granting_total_score() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-pb1-002-R");
    let live = game.id("PL!-sd1-019-SD");
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(source);
    game.state.player2.hand.cards.extend([member, live]);
    game.give_energy(15);
    game.play_to_stage(source, MemberArea::Center);
    game.drain_auto_ability_choices();
    game.assert_select_card("hand", 1, true);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
    game.select_indices(&[]);
    game.drain_auto_ability_choices();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player2.hand.cards.as_slice(), &[member, live]);
    assert!(game.state.player2.waitroom.cards.is_empty());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
}

#[test]
fn opponent_without_live_keeps_members_and_grants_total_score_without_choice() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-pb1-002-R");
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(source);
    game.state.player2.hand.cards.push(member);
    game.give_energy(15);
    game.play_to_stage(source, MemberArea::Center);
    game.drain_auto_ability_choices();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player2.hand.cards.as_slice(), &[member]);
    assert!(game.state.player2.waitroom.cards.is_empty());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
}

#[test]
fn opponent_empty_hand_grants_total_score_without_choice() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-pb1-002-R");
    game.state.player1.hand.cards.push(source);
    game.give_energy(15);
    game.play_to_stage(source, MemberArea::Center);
    game.drain_auto_ability_choices();
    assert!(!game.has_pending_choice());
    assert!(game.state.player2.hand.cards.is_empty());
    assert!(game.state.player2.waitroom.cards.is_empty());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
}

#[test]
fn declined_discard_total_score_is_lost_when_yoshiko_activation_discards_source() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-pb1-002-R");
    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let cost_card = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.hand.cards.extend([source, yoshiko, cost_card]);
    game.state.player2.hand.cards.push(live);
    game.give_energy(30);
    game.play_to_stage(yoshiko, MemberArea::Center);
    game.drain_auto_ability_choices();
    game.play_to_stage(source, MemberArea::LeftSide);
    game.drain_auto_ability_choices();
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[]);
    game.drain_auto_ability_choices();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.player1.stage.stage, [source, yoshiko, -1]);
    game.activate_ability(yoshiko);
    game.drain_auto_ability_choices();
    game.assert_select_card("hand", 1, false);
    game.select_indices(&[0]);
    game.drain_auto_ability_choices();
    game.assert_select_card("stage", 1, false);
    game.select_indices(&[0]);
    game.drain_auto_ability_choices();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [-1, yoshiko, -1]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[cost_card, source]);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.mods.get_orientation_modifier(yoshiko), Some("wait"));
    assert_eq!(game.state.player2.hand.cards.as_slice(), &[live]);
    assert!(game.state.player2.waitroom.cards.is_empty());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
}
