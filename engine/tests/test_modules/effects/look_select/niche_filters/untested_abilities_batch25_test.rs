/// Untested-abilities batch 25 — set_heart_type (PL!S-bp7-024-L):
/// 「ライブ終了時まで、自分のステージにいる『Aqours』のメンバー1人は、
///   元々持つハートがすべて{{heart_04}}になる。」
/// Observable primitive: heart_color_multiplier entry -> Heart04.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

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

fn multiplier_is_heart04(game: &TestGame, cid: i16) -> bool {
    game.state.mods.heart_color_multiplier.get(&cid).copied()
        == Some(HeartColor::Heart04)
}

#[test]
fn bp7024_lone_aqours_member_hearts_become_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp7-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    let aqours = game.id("PL!S-bp5-007-R"); // Aqours, mixed base hearts
    game.state.player1.stage.stage[0] = aqours;
    let other = game.new_id(FILLER); // μ's — unaffected
    game.state.player1.stage.stage[1] = other;

    fire_live_start(&mut game, live);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        multiplier_is_heart04(&game, aqours),
        "the lone Aqours member's hearts become heart04"
    );
    assert!(
        !multiplier_is_heart04(&game, other),
        "non-Aqours member untouched"
    );
}

#[test]
fn bp7024_no_aqours_member_no_transform() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp7-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    let other = game.new_id(FILLER);
    game.state.player1.stage.stage[0] = other;

    fire_live_start(&mut game, live);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !multiplier_is_heart04(&game, other),
        "no Aqours member -> nothing transformed"
    );
}
