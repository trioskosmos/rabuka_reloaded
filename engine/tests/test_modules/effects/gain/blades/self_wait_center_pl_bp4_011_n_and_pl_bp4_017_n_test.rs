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
fn pl_bp4_011_n_paid_self_wait_grants_two_blades_only_to_center_mus_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!-bp4-011-N");
    game.state.player1.stage.stage[0] = me;
    let center = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage[1] = center;
    let left_mu = game.new_id("PL!-sd1-007-SD");
    game.state.player1.stage.stage[2] = left_mu;
    fire_live_start(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "pay-optional-cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget pay_optional_cost gate"
    );
    game.select_option(1);
    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "cost waits this member"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(center),
        2,
        "center-area μ's member gains +2 blades"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(left_mu),
        0,
        "non-center member gains nothing"
    );
}

#[test]
fn pl_bp4_017_n_paid_self_wait_grants_one_blade_to_center_mus_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!-bp4-017-N");
    game.state.player1.stage.stage[0] = me;
    let center = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage[1] = center;
    fire_live_start(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "pay-optional-cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget pay_optional_cost gate"
    );
    game.select_option(1);
    assert_eq!(
        game.state.mods.get_blade_modifier(center),
        1,
        "twin grants +1 blade to center-area μ's member"
    );
}
