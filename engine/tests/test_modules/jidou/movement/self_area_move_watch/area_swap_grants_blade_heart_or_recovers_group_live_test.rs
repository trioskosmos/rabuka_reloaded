use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::zones::MemberArea;

#[test]
fn area_swap_grants_self_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id("PL!SP-bp7-014-N");
    stage_member_and_swap_area(&mut game, target, MemberArea::LeftSide);
    assert_eq!(blade_modifier(&game, target), 2, "嵐千砂都 should gain 2 blades on area move");
}

#[test]
fn area_swap_grants_self_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id("PL!SP-sd2-012-SD2");
    stage_member_and_swap_area(&mut game, target, MemberArea::Center);
    assert_eq!(heart_modifier(&game, target, HeartColor::Heart02), 1, "澁谷かのん should gain heart02 on area move");
}

#[test]
fn area_swap_grants_self_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id("PL!SP-sd2-022-SD2");
    stage_member_and_swap_area(&mut game, target, MemberArea::Center);
    assert_eq!(heart_modifier(&game, target, HeartColor::Heart03), 1, "鬼塚冬毬 should gain heart03 on area move");
}

#[test]
fn area_swap_recovers_low_score_group_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let may = game.id("PL!SP-bp4-007-R");
    let liella = game.id(LIELIA_LIVE); // score 1, Liella, ≤3
    game.state.player1.waitroom.cards.push(liella);

    stage_member_and_swap_area(&mut game, may, MemberArea::Center);

    assert!(
        game.state.player1.hand.cards.contains(&liella),
        "米女メイ ab#0 should add a score≤3 Liella live card from the discard to hand"
    );
}

#[test]
fn self_wait_activation_then_area_swap_removes_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiki = game.id("PL!SP-bp7-008-R");
    append_twenty_filler_cards(&mut game);
    game.add_to_stage(MemberArea::Center, shiki);
    game.give_energy(10);

    // ab#0 起動: wait self → draw 1.
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&shiki),
        Some(&CardOrientation::Wait),
        "若菜四季 ab#0 should wait herself"
    );

    // ab#1 自動: wait member area-moves → activate her.
    activate_mill_three_position_swap(&mut game, MemberArea::Center);
    assert_ne!(
        game.state.mods.orientation_modifiers.get(&shiki),
        Some(&CardOrientation::Wait),
        "若菜四季 ab#1 should reactivate a wait member on area move"
    );
}
