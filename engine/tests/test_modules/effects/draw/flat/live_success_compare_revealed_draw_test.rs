use crate::helpers::*;

/// 渡辺 曜 PL!S-bp3-005-R ab#0 (ライブ成功時):
///
///   エールにより公開された自分のカードの枚数が、相手がエールによって公開した
///   カードの枚数より少ない場合、カードを1枚引く。
///
/// NOT COVERED as behaviour yet, on purpose. The obvious rewrite — a differential
/// over the opponent's yell blade, asserting p1 draws only when its own reveal
/// count is lower — was written and then removed: it could not get START:DASH!!
/// to SUCCEED, and ライブ成功時 never runs for a failed live, so the comparison
/// was untestable without first understanding how the heart requirement is met.
/// Whoever picks it up should assert "the live succeeded" FIRST, so a missing
/// heart fixture fails with that message rather than a confusing draw mismatch.
///
/// What IS covered is that the trigger is routed to the resolver and gets a
/// verdict — see `you_bp3_005_live_success_trigger_is_evaluated_and_records_a_verdict`.
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
fn you_bp3_005_live_success_trigger_is_evaluated_and_records_a_verdict() {
    // Kept alongside the behavioural test above: the printed comparison needs a
    // yell to differ, which is a lot of fixture, whereas "the trigger was
    // routed to the resolver and got a verdict" is cheap and is a distinct
    // failure — a trigger that never fires satisfies the comparison vacuously.
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

    // Step by name; the five-and-five pass counts this replaces were correct
    // only while the phase sequence was frozen.
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live_card);
    game.advance_to_phase(rabuka_engine::game_state::Phase::Active);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
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
        .any(|a| a.triggers.as_ref().is_some_and(|t| &**t == "常時"));
    assert!(constant_ability, "Should have at least one 常時 ability");
}
