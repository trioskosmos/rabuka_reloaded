/// Tests for PL!HS-pb1-025-L | 抱きしめる花びら (ab#0):
///
/// ライブ開始時: 自分の控え室に『蓮ノ空』のメンバーカードが10枚以上ある場合、
/// ライブ終了時まで、自分のステージにいる『蓮ノ空』のメンバー1人は、
/// {{heart_04.png|heart04}}を得る。
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Helper: fill both decks with filler cards.
fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Helper: push N hasunosora member cards into player1's waitroom.
fn setup_waitroom(game: &mut TestGame, count: usize) {
    let hasu = game.id("PL!HS-PR-001-PR");
    for _ in 0..count {
        game.state.player1.waitroom.cards.push(hasu);
    }
}

/// Helper: trigger LiveStart ability on a card.
fn trigger_live_start(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .cloned()
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

/// Helper: get heart04 modifier for a card.
fn heart04_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart04)
}

/// Condition met (10+ in waitroom) → hasunosora member gains heart04.
/// When only 1 valid target exists, the effect auto-applies without a choice.
#[test]
fn hasunosora_pb1_condition_met_gains_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanasaka = game.id("PL!HS-pb1-025-L");
    let hasu = game.id("PL!HS-PR-001-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, hasu, -1];
    game.state.player1.hand.cards.push(hanasaka);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    setup_waitroom(&mut game, 10);
    game.give_energy(5);

    trigger_live_start(&mut game, hanasaka);

    // When only 1 valid target, effect auto-applies without prompt
    assert_eq!(
        heart04_mod(&game, hasu),
        1,
        "hasunosora member should gain +1 heart04"
    );
}

/// Condition NOT met (<10 in waitroom) → no effect.
#[test]
fn hasunosora_pb1_condition_not_met_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanasaka = game.id("PL!HS-pb1-025-L");
    let hasu = game.id("PL!HS-PR-001-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, hasu, -1];
    game.state.player1.hand.cards.push(hanasaka);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    setup_waitroom(&mut game, 9);
    game.give_energy(5);

    trigger_live_start(&mut game, hanasaka);

    assert!(
        !game.has_pending_choice(),
        "condition not met (9 < 10) → no effect"
    );
    assert_eq!(heart04_mod(&game, hasu), 0, "no heart04 should be gained");
}

/// Non-hasunosora member should NOT be targetable.
#[test]
fn hasunosora_pb1_non_hasu_not_targeted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanasaka = game.id("PL!HS-pb1-025-L");
    let non_hasu = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, non_hasu, -1];
    game.state.player1.hand.cards.push(hanasaka);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    setup_waitroom(&mut game, 10);
    game.give_energy(5);

    trigger_live_start(&mut game, hanasaka);

    // No valid target → no pending choice
    assert!(
        !game.has_pending_choice(),
        "no hasunosora target → no select target choice"
    );
    assert_eq!(
        heart04_mod(&game, non_hasu),
        0,
        "non-hasunosora member should not get heart04"
    );
}

/// Multiple hasunosora members on stage → only 1 gets heart04 (target_count=1).
#[test]
fn hasunosora_pb1_only_one_gets_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanasaka = game.id("PL!HS-pb1-025-L");
    let hasu_a = game.id("PL!HS-PR-001-PR");
    let hasu_b = game.id("PL!HS-PR-002-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hasu_a, hasu_b, -1];
    game.state.player1.hand.cards.push(hanasaka);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    setup_waitroom(&mut game, 10);
    game.give_energy(5);

    trigger_live_start(&mut game, hanasaka);

    assert!(
        game.has_pending_choice(),
        "should have select target choice"
    );

    // Select first hasunosora member
    game.select_indices(&[0]);

    assert_eq!(
        heart04_mod(&game, hasu_a),
        1,
        "selected member should gain heart04"
    );
    assert_eq!(
        heart04_mod(&game, hasu_b),
        0,
        "unselected member should NOT gain heart04"
    );
}
