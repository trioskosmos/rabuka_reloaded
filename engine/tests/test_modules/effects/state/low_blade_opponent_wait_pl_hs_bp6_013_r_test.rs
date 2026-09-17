use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

fn pl_hs_bp6_013_r_wait_flow(game: &mut TestGame, _trig: &str, trigger: AbilityTrigger) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!HS-bp6-013-R");
    game.state.player1.stage.stage[1] = me;
    let victim = game.new_id(FILLER);
    game.state.player2.stage.stage[1] = victim;
    let ability_id = {
        let card = game.db.get_card(me).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref().is_some_and(|t| t.contains("登場")))
            .expect("bp6-013-R lacks a 登場-triggered ability");
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(game.db.get_card(me).unwrap().card_no.to_string()),
        Some(me),
        None,
        None,
    );
    game.state.activating_card = Some(me);
    game.state.process_pending_auto_abilities(&pid);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    victim
}

#[test]
fn pl_hs_bp6_013_r_live_start_waits_low_blade_opponent_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let victim = pl_hs_bp6_013_r_wait_flow(&mut game, "ライブ開始時", AbilityTrigger::LiveStart);
    assert_eq!(
        game.state.mods.get_orientation_modifier(victim).as_deref(),
        Some("wait"),
        "opponent low-blade member waited"
    );
}
