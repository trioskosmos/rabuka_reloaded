use crate::helpers::*;
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

#[test]
fn shizuku_bp5_015_n_all_six_stage_heart_colors_grant_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let shizuku = game.id("PL!N-bp5-015-N");
    game.state.player1.stage.stage[0] = shizuku;
    let honoka = game.id("PL!-sd1-001-SD");
    let kanan = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage[1] = honoka;
    game.state.player1.stage.stage[2] = kanan;

    fire_live_start(&mut game, shizuku);

    assert_eq!(
        game.state.mods.get_blade_modifier(shizuku),
        2,
        "all six colors present across members -> +2 blades"
    );
}

#[test]
fn shizuku_bp5_015_n_missing_stage_heart_colors_grant_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let shizuku = game.id("PL!N-bp5-015-N");
    game.state.player1.stage.stage[0] = shizuku;
    let honoka = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage[1] = honoka;

    fire_live_start(&mut game, shizuku);

    assert_eq!(
        game.state.mods.get_blade_modifier(shizuku),
        0,
        "colors 02/04/05 missing -> no blades"
    );
}
