use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

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

#[test]
fn pl_bp4_004_r_success_score_six_activates_two_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-bp4-004-R");
    let live6 = game.id("PL!SP-bp1-027-L");
    game.add_to_stage(MemberArea::Center, umi);
    game.state.player1.success_live_card_zone.add_card(live6);
    for _ in 0..5 {
        let e = game.new_id("LL-E-001-SD");
        game.state.player1.energy_zone.cards.push(e);
    }
    game.state.player1.energy_zone.set_active_count(2);
    fire_trigger(&mut game, umi, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4,
        "score total 6 ≥ 6 → activate 2"
    );
}

#[test]
fn pl_bp4_004_r_success_score_five_activates_no_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-bp4-004-R");
    let live5 = game.id("PL!S-PR-024-PR");
    game.add_to_stage(MemberArea::Center, umi);
    game.state.player1.success_live_card_zone.add_card(live5);
    for _ in 0..4 {
        let e = game.new_id("LL-E-001-SD");
        game.state.player1.energy_zone.cards.push(e);
    }
    game.state.player1.energy_zone.set_active_count(1);
    fire_trigger(&mut game, umi, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "score total 5 < 6 → nothing"
    );
}
