/// Untested-abilities batch 36 — baton-this-turn required-heart reductions.
///
/// - PL!HS-bp2-023-L Mirage Voyage (ライブ開始時): ≥2 『蓮ノ空』 members who
///   arrived via baton touch THIS turn -> this live card's heart05
///   requirement −1.
/// - PL!HS-bp2-025-L ココン東西 (ライブ開始時): same gate -> heart01 −1.
///
/// Baton arrivals use the real play pipeline (play_to_stage onto an
/// occupied area), which records baton_touch_arriving_card_ids and the
/// per-player baton count.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const MIRAGE: &str = "PL!HS-bp2-023-L";
const KOKON: &str = "PL!HS-bp2-025-L";
// 蓮ノ空-series members (HS set).
const HNS_A: &str = "PL!HS-bp5-004-R"; // 百生吟子 cost 15
const HNS_B: &str = "PL!HS-bp5-006-R"; // 桜小路きな子 cost 11
const HNS_C: &str = "PL!HS-bp5-001-P"; // 日野下花帆 cost 11

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
fn mirage_two_baton_arrivals_reduce_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);

    // Occupied areas: one 蓮ノ空 member + one μ's member.
    let seated_a = game.id(HNS_A);
    game.state.player1.stage.stage[0] = seated_a;
    let seated_f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = seated_f;

    // Two 蓮ノ空 arrivals via baton touch.
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
fn mirage_one_baton_arrival_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);

    let seated_a = game.id(HNS_A);
    game.state.player1.stage.stage[0] = seated_a;
    let seated_f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = seated_f;

    // Only ONE baton arrival; the other seat stays μ's.
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
fn mirage_batons_of_wrong_group_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);

    // Two seats held by μ's members; both replaced by MORE μ's members.
    let f1 = game.id("PL!-sd1-010-SD");
    let f2 = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = f1;
    game.state.player1.stage.stage[1] = f2;

    let b = game.new_id("PL!-sd1-007-SD"); // 東條希, μ's
    let c = game.new_id("PL!-sd1-001-SD"); // 高坂穂乃果, μ's
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
fn mirage_seated_without_baton_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_live(&mut game, MIRAGE);

    // Two 蓮ノ空 members present but placed DIRECTLY (no baton touch).
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
fn kokon_two_baton_arrivals_reduce_heart01() {
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
