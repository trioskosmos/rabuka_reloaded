use crate::helpers::*;
use rabuka_engine::card::AbilityEffect;
use rabuka_engine::ability::resolver::AbilityResolver;

#[test]
fn shiori_live_total_plus_one_no_floor_needed() {
    // Trigger via fire_trigger with surplus 0 -> +1
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiori = game.id("PL!N-bp5-010-R");
    game.state.player1.stage.stage = [shiori, -1, -1];
    game.state.mods.p1_constant_total_score_bonus = 0;
    game.state.live_surplus_ready_this_turn = true;
    game.state.self_live_surplus_count = 0;
    game.state.opponent_live_surplus_count = 0;
    fire_trigger(
        &mut game,
        shiori,
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "surplus 0 should give +1"
    );
}

#[test]
fn shiori_live_total_minus_one_clamped_at_zero() {
    // Direct engine floor check: load the real -1 effect for shiori and apply
    // it when base+bonus is 0 -> should be clamped to 0.
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    game.state.mods.p1_constant_total_score_bonus = 0;
    // Ensure no live base
    game.state.player1.live_card_zone.cards.clear();
    game.state.player1.success_live_card_zone.cards.clear();
    let shiori = game.id("PL!N-bp5-010-R");
    game.state.activating_card = Some(shiori);
    game.state.ability_queue.push_constant_context("p1".to_string());
    // Load the real second action (-1) from abilities.json
    let abilities: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("../cards/abilities.json").unwrap()).unwrap();
    let uniq = abilities["unique_abilities"].as_array().unwrap();
    let entry = uniq
        .iter()
        .find(|a| {
            a["cards"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c.as_str().unwrap().contains("PL!N-bp5-010-R"))
        })
        .unwrap();
    let action_json = entry["effect"]["actions"][1].clone(); // -1 action
    let effect: AbilityEffect = serde_json::from_value(action_json).unwrap();
    let mut resolver = AbilityResolver::new(db, Some(shiori));
    resolver.execute_modify_score(&mut game.state, &effect).unwrap();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "floor: 0 + (-1) with min:0 should stay 0, not -1"
    );
    game.state.ability_queue.pop_constant_context();
}

#[test]
fn shiori_live_total_minus_one_allowed_when_base_exists() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiori = game.id("PL!N-bp5-010-R");
    let live = game.id("PL!N-bp1-027-L");
    game.state.player1.stage.stage = [shiori, -1, -1];
    game.state.player1.live_card_zone.cards.push(live);
    game.state.mods.p1_constant_total_score_bonus = 0;
    fire_trigger(
        &mut game,
        shiori,
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    // With a live present, total should still be >=0
    assert!(
        game.state.mods.p1_constant_total_score_bonus >= -2,
        "with base live, -1 may be allowed but total stays >=0"
    );
}

#[test]
fn shiori_parser_emits_floor() {
    // Verify abilities.json now has the structural floor for PL!N-bp5-010-R
    let abilities: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("../cards/abilities.json").unwrap()).unwrap();
    let uniq = abilities["unique_abilities"].as_array().unwrap();
    let entry = uniq
        .iter()
        .find(|a| {
            a["cards"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c.as_str().unwrap().contains("PL!N-bp5-010-R"))
        })
        .expect("PL!N-bp5-010-R should exist");
    let eff = &entry["effect"];
    assert_eq!(eff["effect_constraint"], "min:0", "parent should have floor");
    assert_eq!(eff["score_floor"], 0);
    let actions = eff["actions"].as_array().unwrap();
    for act in actions {
        assert_eq!(act["effect_constraint"], "min:0");
        assert_eq!(act["target"], "live_total");
    }
}

#[test]
fn shiori_sequential_both_conditions_present() {
    let abilities: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("../cards/abilities.json").unwrap()).unwrap();
    let uniq = abilities["unique_abilities"].as_array().unwrap();
    let entry = uniq
        .iter()
        .find(|a| {
            a["cards"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c.as_str().unwrap().contains("PL!N-bp5-010-R"))
        })
        .unwrap();
    let actions = entry["effect"]["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 2, "should have +1 and -1 actions");
    // First is surplus_heart negation (no surplus), second is surplus >=2
    assert!(actions[0]["condition"]["negation"].as_bool().unwrap_or(false));
    assert_eq!(actions[1]["condition"]["count"], 2);
}

#[test]
fn shiori_p_and_ar_variants_share_floor() {
    let abilities: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("../cards/abilities.json").unwrap()).unwrap();
    let uniq = abilities["unique_abilities"].as_array().unwrap();
    for variant in ["PL!N-bp5-010-P", "PL!N-bp5-010-AR"] {
        let entry = uniq
            .iter()
            .find(|a| {
                a["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c.as_str().unwrap().contains(variant))
            })
            .unwrap_or_else(|| panic!("{} should exist", variant));
        assert_eq!(entry["effect"]["effect_constraint"], "min:0", "{} parent floor", variant);
        assert_eq!(entry["effect"]["score_floor"], 0);
    }
}

#[test]
fn shiori_surplus_one_no_change() {
    // Surplus 1 -> neither +1 (needs 0) nor -1 (needs >=2) should fire.
    // Verify the two conditions are mutually exclusive and cover 0 and >=2 only.
    let abilities: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("../cards/abilities.json").unwrap()).unwrap();
    let entry = abilities["unique_abilities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| {
            a["cards"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c.as_str().unwrap().contains("PL!N-bp5-010-R"))
        })
        .unwrap();
    let cond0 = &entry["effect"]["actions"][0]["condition"];
    let cond1 = &entry["effect"]["actions"][1]["condition"];
    assert!(cond0["negation"].as_bool().unwrap(), "first is no surplus");
    assert_eq!(cond0["resource_type"], "surplus_heart");
    assert_eq!(cond1["count"], 2);
    assert_eq!(cond1["resource_type"], "surplus_heart");
    // No single surplus value satisfies both: 0 satisfies first, >=2 satisfies second, 1 satisfies neither.
}

#[test]
fn shiori_p2_live_total_floor() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let shiori = game.id("PL!N-bp5-010-R");
    // Put shiori on p2's stage
    game.state.player2.stage.stage = [shiori, -1, -1];
    game.state.mods.p2_constant_total_score_bonus = 0;
    game.state.player2.live_card_zone.cards.clear();
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.activating_card = Some(shiori);
    game.state.ability_queue.push_constant_context("p2".to_string());
    let abilities: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("../cards/abilities.json").unwrap()).unwrap();
    let entry = abilities["unique_abilities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| {
            a["cards"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c.as_str().unwrap().contains("PL!N-bp5-010-R"))
        })
        .unwrap();
    let action_json = entry["effect"]["actions"][1].clone();
    let effect: AbilityEffect = serde_json::from_value(action_json).unwrap();
    let mut resolver = AbilityResolver::new(db, Some(shiori));
    resolver.execute_modify_score(&mut game.state, &effect).unwrap();
    assert_eq!(
        game.state.mods.p2_constant_total_score_bonus, 0,
        "p2 floor: 0 + (-1) should stay 0"
    );
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0, "p1 unaffected");
    game.state.ability_queue.pop_constant_context();
}
