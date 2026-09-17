use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

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
fn pl_s_bp7_020_l_milled_bottom_aqours_member_reduces_required_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = pl_s_bp7_020_l_in_live_zone(&mut game);
    let filler = game.id(FILLER);
    let aqours_member = game.id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(aqours_member);
    fire_trigger_nth(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時", 1);
    assert!(
        game.state.player1.waitroom.cards.contains(&aqours_member),
        "bottom card was milled to the waitroom"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        -1,
        "milled card WAS 『Aqours』 member → need heart0 −1"
    );
}

#[test]
fn pl_s_bp7_020_l_milled_bottom_non_aqours_member_does_not_reduce_required_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hpt = pl_s_bp7_020_l_in_live_zone(&mut game);
    let mus_member = game.id(FILLER);
    game.state.player1.main_deck.cards.push(mus_member);
    fire_trigger_nth(&mut game, hpt, AbilityTrigger::LiveStart, "ライブ開始時", 1);
    assert!(game.state.player1.waitroom.cards.contains(&mus_member));
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hpt, HeartColor::Heart00),
        0,
        "μ's member milled → NO reduction"
    );
}
