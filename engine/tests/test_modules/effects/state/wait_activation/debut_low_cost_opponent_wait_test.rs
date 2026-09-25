use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn sp_pb2_029_debut_opponent_cost2_member_is_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-pb2-029-N");
    // PL!SP-PR-007-PR is a KALEIDOSCORE member with cost 2.
    let cheap = game.id("PL!SP-PR-007-PR");
    game.state.player1.stage.stage[0] = me;
    game.state.player2.stage.stage[0] = cheap;

    // Dual-trigger ability (登場/ライブ開始時): fire the debut window directly.
    let ability_id = {
        let card = game.db.get_card(me).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref().is_some_and(|t| t.contains("登場")))
            .expect("pb2-029 lacks a 登場-triggered ability");
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Debut,
        pid.clone(),
        Some(game.db.get_card(me).unwrap().card_no.to_string()),
        Some(me),
        None,
        None,
    );
    game.state.activating_card = Some(me);
    game.state.process_pending_auto_abilities(&pid);

    assert_eq!(
        game.state.mods.orientation_modifiers.get(&cheap).copied(),
        Some(rabuka_engine::core::game_modifiers::CardOrientation::Wait),
        "the opponent's cost-2 member should be rested"
    );
}
