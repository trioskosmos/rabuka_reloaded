use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const MIRAGE: &str = "PL!HS-bp2-023-L";
const KOKON: &str = "PL!HS-bp2-025-L";
const HNS_A: &str = "PL!HS-bp5-004-R";
const HNS_B: &str = "PL!HS-bp5-006-R";
const HNS_C: &str = "PL!HS-bp5-001-P";

fn drain(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
}

fn setup_live(game: &mut TestGame, no: &str) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id(no);
    game.state.player1.live_card_zone.cards.push(live);
    live
}

fn fire_live_start(game: &mut TestGame, live: i16) {
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(live).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pl_hs_bp2_023_l_two_hasunosora_baton_arrivals_reduce_only_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);
    let seated_a = game.id(HNS_A);
    game.state.player1.stage.stage[0] = seated_a;
    let seated_f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = seated_f;
    let b = game.new_id(HNS_B);
    let c = game.new_id(HNS_C);
    game.add_to_hand(b);
    game.add_to_hand(c);
    game.give_energy(40);
    game.play_to_stage(b, MemberArea::LeftSide);
    drain(&mut game);
    game.play_to_stage(c, MemberArea::Center);
    drain(&mut game);
    fire_live_start(&mut game, live);
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart05),
        -1,
        "two baton-touch 蓮ノ空 arrivals -> heart05 requirement -1"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart01),
        0,
        "Mirage Voyage touches heart05 only"
    );
}

#[test]
fn pl_hs_bp2_023_l_one_baton_arrival_does_not_reduce_required_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);
    let seated_a = game.id(HNS_A);
    game.state.player1.stage.stage[0] = seated_a;
    let seated_f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = seated_f;
    let b = game.new_id(HNS_B);
    game.add_to_hand(b);
    game.give_energy(40);
    game.play_to_stage(b, MemberArea::LeftSide);
    drain(&mut game);
    fire_live_start(&mut game, live);
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart05),
        0,
        "only one baton arrival -> gate needs 2"
    );
}

#[test]
fn pl_hs_bp2_023_l_non_hasunosora_arrivals_do_not_reduce_required_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);
    let f1 = game.id("PL!-sd1-010-SD");
    let f2 = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = f1;
    game.state.player1.stage.stage[1] = f2;
    let b = game.new_id("PL!-sd1-007-SD");
    let c = game.new_id("PL!-sd1-001-SD");
    game.add_to_hand(b);
    game.add_to_hand(c);
    game.give_energy(40);
    game.play_to_stage(b, MemberArea::LeftSide);
    drain(&mut game);
    game.play_to_stage(c, MemberArea::RightSide);
    drain(&mut game);
    fire_live_start(&mut game, live);
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart05),
        0,
        "baton arrivals without 蓮ノ空 membership don't qualify"
    );
}

#[test]
fn pl_hs_bp2_023_l_hasunosora_without_baton_arrivals_do_not_reduce_required_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);
    let a = game.id(HNS_A);
    let b = game.id(HNS_B);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[1] = b;
    fire_live_start(&mut game, live);
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart05),
        0,
        "蓮ノ空 members without baton-touch arrival don't qualify"
    );
}

#[test]
fn pl_hs_bp2_025_l_two_hasunosora_baton_arrivals_reduce_only_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, KOKON);
    let seated_a = game.id(HNS_A);
    game.state.player1.stage.stage[0] = seated_a;
    let seated_f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = seated_f;
    let b = game.new_id(HNS_B);
    let c = game.new_id(HNS_C);
    game.add_to_hand(b);
    game.add_to_hand(c);
    game.give_energy(40);
    game.play_to_stage(b, MemberArea::LeftSide);
    drain(&mut game);
    game.play_to_stage(c, MemberArea::Center);
    drain(&mut game);
    fire_live_start(&mut game, live);
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart01),
        -1,
        "ココン東西 reduces heart01 requirement"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart05),
        0,
        "ココン東西 touches heart01 only"
    );
}
