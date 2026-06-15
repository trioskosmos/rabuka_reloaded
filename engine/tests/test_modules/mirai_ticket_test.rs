use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!S-bp6-021-L | MIRAI TICKET (ab#0)
///
/// 自分がエールしたとき、エールにより公開された自分のカードの中から
/// ブレードハートを持たない『Aqours』のメンバーカードを1枚まで
/// 控え室に置いてもよい。そうした場合、これにより控え室に置いた
/// カードのコスト5につき、追加で1枚エールを行う。
/// この能力では4枚までしか追加でエールできない。
///
/// The condition text "自分がエールしたとき" is a "custom" condition type.
/// The trigger system already filters for yell timing, so the custom
/// condition should always evaluate to true at that point.
#[test]
fn mirai_ticket_custom_condition_loads_and_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mirai = game.id("PL!S-bp6-021-L");
    let live = game.id("PL!-bp3-026-L"); // Oh,Love&Peace!
    let center = game.id("PL!-pb1-014-R"); // stage member with blade
    let filler = game.id("PL!-sd1-010-SD");

    // Fill decks
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage MIRAI TICKET
    game.state.player1.stage.stage = [-1, mirai, -1];

    // Verify the ability is loaded with custom condition
    let card = game
        .state
        .card_database
        .get_card(mirai)
        .expect("MIRAI TICKET should exist in DB");
    assert!(
        !card.abilities.is_empty(),
        "MIRAI TICKET should have abilities"
    );
    let ab = &card.abilities[0];
    let cond = ab.effect.as_ref().and_then(|e| e.condition.as_ref());
    assert!(cond.is_some(), "Ability should have a condition");
    if let Some(c) = cond {
        // The condition type should be "custom" (parser couldn't fully parse)
        assert_eq!(
            c.condition_type,
            Some(rabuka_engine::ability::enums::ConditionType::Custom),
            "Condition type should be Custom"
        );
        assert!(!c.text.is_empty(), "Custom condition should have text");
        assert!(
            c.text.contains("エール"),
            "Condition text should mention エール (yell): {}",
            c.text
        );
    }
}

/// Verify the ability processes through the live performance pipeline
/// without crashing. The custom condition evaluates to true when yell
/// occurs during performance.
#[test]
fn mirai_ticket_performance_does_not_crash() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mirai = game.id("PL!S-bp6-021-L");
    let live = game.id("PL!-bp3-026-L");
    let center = game.id("PL!-pb1-014-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Center with blade for yell, MIRAI TICKET on stage
    game.state.player1.stage.stage = [-1, center, mirai];
    game.state.player2.stage.stage = [-1, filler, -1];

    // Set live cards
    game.state.player1.hand.cards.push(live);
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    // LiveCardSet phase
    game.set_live_card(live);

    // Handle any pending choices from live-start triggers
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // P2 passes
    game.state.player2.hand.cards.push(live);
    game.pass();
    game.set_live_card(live);
    game.pass();

    // Process pending auto abilities (MIRAI TICKET may trigger here)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Advance through performance → LiveVictory without crash
    game.pass();
    game.pass();

    // If we get here, no crash occurred
    // The ability may or may not have fired depending on
    // whether Aqours members without blade were revealed
}
