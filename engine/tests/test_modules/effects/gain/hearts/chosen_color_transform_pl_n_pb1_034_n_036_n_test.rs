use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn multiplier(game: &TestGame, cid: i16) -> Option<HeartColor> {
    game.state.mods.heart_color_multiplier.get(&cid).copied()
}

fn pl_n_pb1_034_n_036_n_setup(game: &mut TestGame, no: &str) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id(no);
    game.state.player1.stage.stage[1] = me;
    let bystander = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = bystander;
    me
}

#[test]
fn shioriko_pl_n_pb1_034_n_choose_first_color_heart03_leaves_bystander_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_n_pb1_034_n_036_n_setup(&mut game, "PL!N-pb1-034-N");

    fire_live_start(&mut game, me);
    assert!(game.has_pending_choice(), "color choice offered");
    assert!(
        game.pending_choice_type().is_some(),
        "color choice must carry a choice identity"
    );
    game.select_option(0);

    assert_eq!(
        multiplier(&game, me),
        Some(HeartColor::Heart03),
        "chose heart03 -> original hearts become heart03"
    );
    assert_eq!(multiplier(&game, game.id_ref("PL!-sd1-010-SD")), None);
}

#[test]
fn shioriko_pl_n_pb1_034_n_choose_last_color_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_n_pb1_034_n_036_n_setup(&mut game, "PL!N-pb1-034-N");

    fire_live_start(&mut game, me);
    game.select_option(2);

    assert_eq!(
        multiplier(&game, me),
        Some(HeartColor::Heart05),
        "chose heart05 -> original hearts become heart05"
    );
}

#[test]
fn shioriko_pl_n_pb1_034_n_choose_mid_color_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_n_pb1_034_n_036_n_setup(&mut game, "PL!N-pb1-034-N");

    fire_live_start(&mut game, me);
    game.select_option(1);

    assert_eq!(multiplier(&game, me), Some(HeartColor::Heart04));
}

#[test]
fn lanzhu_pl_n_pb1_036_n_choose_last_color_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_n_pb1_034_n_036_n_setup(&mut game, "PL!N-pb1-036-N");

    fire_live_start(&mut game, me);
    assert!(game.has_pending_choice());
    assert!(
        game.pending_choice_type().is_some(),
        "color choice must carry a choice identity"
    );
    game.select_option(2);

    assert_eq!(
        multiplier(&game, me),
        Some(HeartColor::Heart06),
        "twin card's third option is heart06"
    );
}
