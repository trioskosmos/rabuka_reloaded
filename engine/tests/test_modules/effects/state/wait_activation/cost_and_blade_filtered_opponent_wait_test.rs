use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR";
const BIG_RUBY: &str = "PL!S-bp5-009-R";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains(trigger_str))
        })
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn sayaka_hs_pb1_010_debut_with_cost_ten_member_waits_cost_four_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-010-R"); // cost 2
    let big = game.id(BIG_RUBY); // cost 15
    let cheap = game.id(FILLER); // cost 4

    game.state.player1.stage.stage[0] = big;
    game.state.player1.stage.stage[1] = sayaka;
    game.state.player2.stage.stage[0] = cheap;

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        is_waited(&game, cheap),
        "my stage has a cost-15 member → opponent's cost-4 member is waited"
    );
}

#[test]
fn sayaka_hs_pb1_010_debut_without_cost_ten_member_leaves_opponent_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-010-R"); // herself: cost 2
    let cheap = game.id(FILLER);

    game.state.player1.stage.stage[1] = sayaka;
    game.state.player2.stage.stage[0] = cheap;

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !is_waited(&game, cheap),
        "no cost≥10 member on MY stage → condition unmet, opponent untouched"
    );
}

#[test]
fn sayaka_hs_pb1_010_debut_with_cost_ten_member_excludes_cost_five_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-010-R");
    let big = game.id(BIG_RUBY);
    let pricey = game.id(CLEAN_KOTORI); // cost 5 > 4

    game.state.player1.stage.stage[0] = big;
    game.state.player1.stage.stage[1] = sayaka;
    game.state.player2.stage.stage[0] = pricey;

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !is_waited(&game, pricey),
        "condition met but opponent's cheapest member costs 5 > 4 → nobody waited"
    );
}

fn is_waited(game: &TestGame, cid: i16) -> bool {
    game.state.mods.get_orientation_modifier(cid) == Some("wait")
}

#[test]
fn natsumi_sp_bp7_009_live_start_in_center_waits_only_low_original_blade_opponent_not_from_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp7-009-R");
    let low = game.id(FILLER); // original blade 1
    let high = game.id(BIG_RUBY); // original blade 5

    game.state.player1.stage.stage[1] = natsumi; // CENTER required
    game.state.player2.stage.stage = [low, high, -1];

    trigger_auto(
        &mut game,
        natsumi,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        is_waited(&game, low),
        "opponent member with 元々ブレード1 ≤ 2 is waited"
    );
    assert!(
        !is_waited(&game, high),
        "original blade 5 exceeds the limit → untouched"
    );

    // Position gate: from a SIDE the live-start does nothing at all.
    game.state.mods.add_orientation_modifier(low, "active");
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[0] = natsumi;
    trigger_auto(
        &mut game,
        natsumi,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        !is_waited(&game, low),
        "（センター限定）: not in center → no effect even with eligible targets"
    );
}
