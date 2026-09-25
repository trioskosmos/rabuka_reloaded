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

fn mari_pl_s_bp5_017_n_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let mari = game.id("PL!S-bp5-017-N");
    game.state.player1.stage.stage[1] = mari;
    mari
}

#[test]
fn pl_s_bp5_017_n_live_start_heart05_total_four_grants_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = mari_pl_s_bp5_017_n_setup(&mut game);
    let l = game.id("PL!HS-PR-011-PR");
    game.state.player1.live_card_zone.cards.push(l);

    fire_live_start(&mut game, mari);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(mari, HeartColor::Heart05),
        1,
        "aggregate == 4 satisfies >=4"
    );
}

#[test]
fn pl_s_bp5_017_n_live_start_heart05_total_two_grants_no_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = mari_pl_s_bp5_017_n_setup(&mut game);
    let l = game.id("PL!S-PR-023-PR");
    game.state.player1.live_card_zone.cards.push(l);

    fire_live_start(&mut game, mari);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(mari, HeartColor::Heart05),
        0,
        "aggregate 2 < 4 -> no grant"
    );
}

#[test]
fn pl_s_bp5_017_n_live_start_empty_live_zone_grants_no_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = mari_pl_s_bp5_017_n_setup(&mut game);

    fire_live_start(&mut game, mari);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(mari, HeartColor::Heart05),
        0
    );
}
