use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    fire_trigger_nth(game, cid, trigger, trig, 0);
}

fn fire_trigger_nth(
    game: &mut TestGame,
    cid: i16,
    trigger: AbilityTrigger,
    trig: &str,
    nth: usize,
) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .filter(|a| a.triggers.as_deref() == Some(trig))
            .nth(nth)
            .unwrap_or_else(|| panic!("card {} lacks '{trig}' ability #{nth}", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn pl_s_bp7_020_l_in_live_zone(game: &mut TestGame) -> i16 {
    let hpt = game.id("PL!S-bp7-020-L");
    game.state.player1.live_card_zone.cards.push(hpt);
    hpt
}

#[test]
fn pl_s_bp7_020_l_all_active_stage_reduces_required_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = pl_s_bp7_020_l_in_live_zone(&mut game);
    let m1 = game.new_id("PL!S-sd1-001-SD");
    let m2 = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = m1;
    game.state.player1.stage.stage[1] = m2;
    game.state.mods.add_orientation_modifier(m1, "active");
    game.state.mods.add_orientation_modifier(m2, "active");
    fire_trigger(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        -1,
        "all members active → 必要ハート heart0 −1"
    );
}

#[test]
fn pl_s_bp7_020_l_waited_or_empty_stage_does_not_reduce_required_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = pl_s_bp7_020_l_in_live_zone(&mut game);
    let m1 = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = m1;
    game.state.mods.add_orientation_modifier(m1, "wait");
    fire_trigger(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        0,
        "a waited member breaks すべて…アクティブ"
    );
    game.state.player1.stage.stage[0] = -1;
    fire_trigger(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        0,
        "empty stage → すべてのメンバーがアクティブ is not met"
    );
}
