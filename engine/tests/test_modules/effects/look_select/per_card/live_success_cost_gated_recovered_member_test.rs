use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn hs_cl1_009_live_success_hasunosora_cost_four_recovers_revealed_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    let target = game.id("PL!HS-bp1-012-PR"); // 乙宗 梢, cost 4 (lower boundary)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(target);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Single valid candidate auto-resolves — no selection prompt.
    assert!(
        !game.has_pending_choice(),
        "single cost-4 candidate must auto-resolve without prompting"
    );

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "cost-4 蓮ノ空 member (lower boundary) retrieved"
    );
}

#[test]
fn hs_cl1_009_live_success_hasunosora_cost_nine_recovers_revealed_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    let target = game.id("PL!HS-pb1-015-R"); // セラス, cost 9 (upper boundary)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(target);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Single valid candidate auto-resolves — no selection prompt.
    assert!(
        !game.has_pending_choice(),
        "single cost-9 candidate must auto-resolve without prompting"
    );

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "cost-9 蓮ノ空 member (upper boundary) retrieved"
    );
}

#[test]
fn hs_cl1_009_live_success_cost_ten_no_prompt_or_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    let outside = game.id("PL!HS-PR-005-PR"); // 大沢瑠璃乃, cost 10 (> 9)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(outside);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.has_pending_choice(),
        "only out-of-range candidates -> no selection prompt"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&outside),
        "cost-10 member stays in revealed pool"
    );
}

#[test]
fn hs_cl1_009_live_success_wrong_group_cost_four_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-cl1-009-CL");
    // Cost-4 member of the WRONG group (μ's filler).
    let mus = game.new_id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(mus);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.has_pending_choice(),
        "non-蓮ノ空 candidate -> no selection prompt"
    );
}

#[test]
fn hs_bp6_032_live_success_cost_four_recovers_revealed_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-bp6-032-L");
    let target = game.id("PL!HS-bp1-009-R"); // 安養寺 姫芽, cost 4 (== boundary)

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(target);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Single valid candidate auto-resolves — no selection prompt.
    assert!(
        !game.has_pending_choice(),
        "single cost-4 candidate must auto-resolve without prompting"
    );

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "cost-4 member retrieved to hand"
    );
}

#[test]
fn hs_bp6_032_live_success_cost_above_four_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-bp6-032-L");
    let expensive = game.id("PL!HS-bp5-001-P"); // 日野下花帆, cost 11

    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.push(expensive);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.has_pending_choice(),
        "only cost>4 candidates -> no selection prompt"
    );
}
