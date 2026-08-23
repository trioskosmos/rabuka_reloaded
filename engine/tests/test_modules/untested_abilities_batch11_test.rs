/// Untested-abilities batch 11 — depth=none gaps in the cl1/sd2/pb2 sets:
/// - PL!HS-cl1-006-CL (登場): gain 3 blades until live end
/// - PL!SP-sd2-008-SD2 (常時): heart03 while a cost-13+ member is on stage
/// - PL!SP-pb2-029-N (登場/ライブ開始時): rest a member of cost ≤ 2
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(trig))
            .unwrap_or_else(|| panic!("card {} lacks a '{trig}' ability", card.card_no));
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

// ====================================================================
// PL!HS-cl1-006-CL (登場):
// 「ライブ終了時まで、{{blade}}{{blade}}{{blade}}を得る。」
// ====================================================================

#[test]
fn cl1_006_debut_gains_three_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-cl1-006-CL");
    game.state.player1.stage.stage[1] = me;

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        3,
        "debut grants +3 blades until live end"
    );
}

// ====================================================================
// PL!SP-sd2-008-SD2 (常時):
// 「自分のステージにコスト13以上のメンバーがいるかぎり、{{heart_03.png|heart03}}を得る。」
// Constant ability on the member itself — re-evaluated by recalculate_constants.
// ====================================================================

#[test]
fn sd2_008_constant_heart_while_cost13_member_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-sd2-008-SD2");
    let big = game.id("PL!HS-bp5-004-R"); // cost 15
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = big;

    game.state.recalculate_constants();

    const H03: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart03;
    let h03 = game.state.mods.get_heart_modifier(me, H03);
    assert!(h03 > 0, "cost-15 member on stage -> self gains heart03 (got {h03})");
}

#[test]
fn sd2_008_constant_heart_without_expensive_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-sd2-008-SD2");
    let small = game.id(FILLER); // cost 4
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = small;

    game.state.recalculate_constants();

    const H03: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart03;
    let h03 = game.state.mods.get_heart_modifier(me, H03);
    assert_eq!(
        h03, 0,
        "no cost-13+ member -> no heart03 granted"
    );
}

// ====================================================================
// PL!SP-pb2-029-N (登場/ライブ開始時):
// 「相手のステージにいるコスト2以下のメンバー1人をウェイトにする。」
// ====================================================================

#[test]
fn pb2_029_debut_rests_low_cost_member() {
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
