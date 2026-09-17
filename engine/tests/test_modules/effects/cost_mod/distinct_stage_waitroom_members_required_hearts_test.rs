use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

fn setup(distinct: usize) -> (TestGame, i16) {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!HS-pb1-026-L");
    assert_eq!(game.db.get_card(live).unwrap().card_no, "PL!HS-pb1-026-L");
    game.state.player1.live_card_zone.cards.push(live);
    let members = [
        "PL!HS-sd1-001-SD",
        "PL!HS-sd1-002-SD",
        "PL!HS-sd1-003-SD",
        "PL!HS-sd1-004-SD",
        "PL!HS-sd1-005-SD",
        "PL!HS-sd1-006-SD",
    ];
    for (i, print) in members.iter().take(distinct).enumerate() {
        let member = game.id(print);
        if i < 2 {
            game.state.player1.stage.stage[i] = member;
        } else {
            game.state.player1.waitroom.cards.push(member);
        }
    }
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    (game, live)
}

#[test]
fn six_distinct_hasunosora_stage_and_waitroom_members_reduce_own_required_hearts_two() {
    let (mut game, live) = setup(6);
    let other_live = game.id("PL!HS-sd1-017-SD");
    game.state.player1.live_card_zone.cards.push(other_live);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_need_heart_modifier(live, HeartColor::Heart00), -2);
    assert_eq!(game.state.mods.get_need_heart_modifier(other_live, HeartColor::Heart00), 0);
}

#[test]
fn five_distinct_hasunosora_members_plus_duplicate_do_not_reduce_required_hearts() {
    let (mut game, live) = setup(5);
    let duplicate = game.new_id("PL!HS-sd1-001-SD");
    game.state.player1.waitroom.cards.push(duplicate);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_need_heart_modifier(live, HeartColor::Heart00), 0);
}

#[test]
fn five_hasunosora_members_plus_hasunosora_live_do_not_reduce_required_hearts() {
    let (mut game, live) = setup(5);
    let other_live = game.id("PL!HS-sd1-017-SD");
    game.state.player1.waitroom.cards.push(other_live);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_need_heart_modifier(live, HeartColor::Heart00), 0);
}

#[test]
fn five_hasunosora_members_plus_wrong_group_do_not_reduce_required_hearts() {
    let (mut game, live) = setup(5);
    let wrong_group = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(wrong_group);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_need_heart_modifier(live, HeartColor::Heart00), 0);
}
