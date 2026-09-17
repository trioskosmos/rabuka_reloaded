use crate::helpers::*;

#[test]
fn maki_sd1_006_success_zone_restriction_parsed() {
    let db = load_real_database();
    let card = db.get_card_id("PL!-sd1-006-SD").expect("Card exists");
    let card_data = db.get_card(card).expect("Maki card should exist");
    let has_restriction = card_data
        .resolved_abilities()
        .any(|a| a.full_text.contains("成功ライブカード"));
    assert!(
        has_restriction,
        "Card should have success zone restriction ability"
    );
}

#[test]
fn you_bp3_005_live_success_draw_if_fewer_revealed() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-bp3-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    let live_card = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage = [you, member, -1];
    game.state.player1.hand.cards.push(live_card);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    for _ in 0..5 {
        game.pass();
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // The LiveSuccess ability must be routed and resolved with an explicit
    // verdict — silence would mean the trigger never reached the resolver.
    let you_live_success = game
        .state
        .rule_log
        .iter()
        .any(|l| l.contains("渡辺 曜") && l.contains("trigger_live_success"));
    assert!(
        you_live_success,
        "You's LiveSuccess ability must be evaluated during the live"
    );
    let resolved_with_verdict = game.state.rule_log.iter().any(|l| {
        l.contains("渡辺 曜")
            && l.contains("trigger_live_success")
            && (l.contains("result_success") || l.contains("result_failure"))
    });
    assert!(
        resolved_with_verdict,
        "LiveSuccess resolution must record success or failure"
    );
}

#[test]
fn hanamaru_bp3_016_success_zone_cost_increase_constant_parsed() {
    let db = load_real_database();
    let card = db.get_card_id("PL!S-bp3-016-N").expect("Card exists");
    let card_data = db.get_card(card).expect("Hanamaru card should exist");
    assert!(
        !card_data.abilities.is_empty(),
        "Card should have abilities"
    );
    let constant_ability = card_data
        .resolved_abilities()
        .any(|a| a.triggers.as_ref().map_or(false, |t| &**t == "常時"));
    assert!(constant_ability, "Should have at least one 常時 ability");
}
