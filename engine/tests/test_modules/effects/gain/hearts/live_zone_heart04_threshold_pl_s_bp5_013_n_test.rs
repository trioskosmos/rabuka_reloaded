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

fn dia_pl_s_bp5_013_n_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let dia = game.id("PL!S-bp5-013-N");
    game.state.player1.stage.stage[1] = dia;
    dia
}

#[test]
fn dia_pl_s_bp5_013_n_heart04_total_exactly_four_grants_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = dia_pl_s_bp5_013_n_setup(&mut game);

    for _ in 0..2 {
        let l = game.id("PL!S-bp2-020-L");
        game.state.player1.live_card_zone.cards.push(l);
    }

    fire_live_start(&mut game, dia);

    assert_eq!(
        game.state.mods.get_heart_modifier(dia, HeartColor::Heart04),
        1,
        "aggregate == 4 satisfies >=4 -> heart04 granted"
    );
}

#[test]
fn dia_pl_s_bp5_013_n_heart04_total_three_no_grant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = dia_pl_s_bp5_013_n_setup(&mut game);

    let l1 = game.id("PL!S-bp2-020-L");
    let l2 = game.id("PL!S-bp3-020-L");
    game.state.player1.live_card_zone.cards.push(l1);
    game.state.player1.live_card_zone.cards.push(l2);

    fire_live_start(&mut game, dia);

    assert_eq!(
        game.state.mods.get_heart_modifier(dia, HeartColor::Heart04),
        0,
        "aggregate 3 < 4 -> no grant"
    );
}

#[test]
fn dia_pl_s_bp5_013_n_empty_live_zone_no_grant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = dia_pl_s_bp5_013_n_setup(&mut game);

    fire_live_start(&mut game, dia);

    assert_eq!(
        game.state.mods.get_heart_modifier(dia, HeartColor::Heart04),
        0
    );
}
