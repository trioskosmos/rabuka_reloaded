/// Untested-abilities batch 39 — chosen-color heart transformation.
///
/// - PL!N-pb1-034-N 三船栞子 (ライブ開始時): choose 1 of {heart03, heart04,
///   heart05}; until live end THIS member's original hearts become the
///   chosen color (set_heart_type driven by a player selection).
/// - PL!N-pb1-036-N twin: choose among {heart01, heart02, heart06}.
///
/// Observable primitive: mods.heart_color_multiplier entry (same as batch 25).
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

fn shizuku_setup(game: &mut TestGame, no: &str) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id(no);
    game.state.player1.stage.stage[1] = me;
    let bystander = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = bystander;
    me
}

#[test]
fn pb1034_choose_first_color_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = shizuku_setup(&mut game, "PL!N-pb1-034-N");

    fire_live_start(&mut game, me);
    assert!(game.has_pending_choice(), "color choice offered");
    game.select_option(0); // heart03 is the first option

    assert_eq!(
        multiplier(&game, me),
        Some(HeartColor::Heart03),
        "chose heart03 -> original hearts become heart03"
    );
    // The bystander is untouched by a self-targeted transformation.
    assert_eq!(multiplier(&game, game.id_ref("PL!-sd1-010-SD")), None);
}

#[test]
fn pb1034_choose_last_color_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = shizuku_setup(&mut game, "PL!N-pb1-034-N");

    fire_live_start(&mut game, me);
    game.select_option(2); // heart05 is the third option

    assert_eq!(
        multiplier(&game, me),
        Some(HeartColor::Heart05),
        "chose heart05 -> original hearts become heart05"
    );
}

#[test]
fn pb1034_choose_mid_color_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = shizuku_setup(&mut game, "PL!N-pb1-034-N");

    fire_live_start(&mut game, me);
    game.select_option(1); // heart04

    assert_eq!(multiplier(&game, me), Some(HeartColor::Heart04));
}

#[test]
fn pb1036_twin_chooses_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = shizuku_setup(&mut game, "PL!N-pb1-036-N"); // {01,02,06}

    fire_live_start(&mut game, me);
    assert!(game.has_pending_choice());
    game.select_option(2); // heart06

    assert_eq!(
        multiplier(&game, me),
        Some(HeartColor::Heart06),
        "twin card's third option is heart06"
    );
}
