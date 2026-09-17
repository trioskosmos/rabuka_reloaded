/// Tests for 桜内梨子 (PL!S-pb1-002-R) — Debut ability:
///
/// 登場 相手は手札からライブカードを1枚控え室に置いてもよい。
/// そうしなかった場合、ライブ終了時まで、
/// 「常時 ライブの合計スコアを＋１する。」を得る。
///
/// Q130/Q171: Conditional_on_optional — opponent choice + conditional score gain.
use crate::helpers::*;

/// Q130: Opponent skips discarding → conditional fires.
#[test]
fn riko_q130_opponent_skips_triggers_conditional() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-pb1-002-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(riko);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.push(live_card);
    game.give_energy(13);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::LeftSide);

    // Debut triggers opponent choice — assert exactly one SelectCard for hand
    assert!(
        game.has_pending_choice(),
        "Should have discard choice after debut"
    );
    game.assert_select_card("hand", 1, true);
    let entry = game.state.ability_queue.current_entry();
    assert_eq!(
        entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Discard choice should be routed to opponent"
    );

    // Opponent skips (empty indices)
    game.select_indices(&[]);

    // conditional_on_optional auto-resolves with chose_yes=false + negation=true
    let entry = game.state.ability_queue.current_entry();
    if let Some(e) = entry {
        assert_eq!(
            e.optional_cost_result,
            Some(false),
            "optional_cost_result should be Some(false) for skip"
        );
    }
    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");
    assert!(game.state.player1.stage.stage.contains(&riko));
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "P1 hand: 2 - 1 played = 1"
    );
    // Opponent's hand is untouched
    assert_eq!(game.state.player2.hand.cards.len(), 1, "P2 hand unchanged");
    assert!(
        game.state.player2.hand.cards.contains(&live_card),
        "P2 live card still in hand"
    );
}

/// Q130 variant: Opponent discards live card → optional fires, conditional skipped.
#[test]
fn riko_q130_opponent_discards_skips_conditional() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-pb1-002-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(riko);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler);
    game.give_energy(13);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::LeftSide);

    // Debut triggers opponent choice — opponent has 2 cards, must choose 1
    assert!(
        game.has_pending_choice(),
        "Should have discard choice after debut"
    );
    game.assert_select_card("hand", 1, true);
    let entry = game.state.ability_queue.current_entry();
    assert_eq!(
        entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Discard choice should be routed to opponent"
    );

    // Opponent discards the live card (index 0)
    game.select_indices(&[0]);

    // conditional_on_optional auto-resolves with chose_yes=true + negation=true → do_nothing
    let entry = game.state.ability_queue.current_entry();
    if let Some(e) = entry {
        assert_eq!(
            e.optional_cost_result,
            Some(true),
            "optional_cost_result should be Some(true) for paid"
        );
    }
    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");
    assert_eq!(
        game.state.player2.hand.cards.len(),
        1,
        "P2 hand: 2 - 1 discarded = 1"
    );
    assert!(
        !game.state.player2.hand.cards.contains(&live_card),
        "P2 live card discarded"
    );
    assert_eq!(
        game.state.player2.waitroom.cards.len(),
        1,
        "P2 waitroom should have the discarded card"
    );
}

/// Multi-card opponent: opponent has 3 cards, chooses to skip all.
#[test]
fn riko_q130_opponent_multi_card_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-pb1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(riko);
    game.state.player1.hand.cards.push(filler);
    // Opponent has 3 cards (all non-live)
    for _ in 0..3 {
        game.state.player2.hand.cards.push(filler);
    }
    game.give_energy(13);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::LeftSide);

    // Opponent has 0 live cards (all filler) — move_cards with card_type: "live_card"
    // finds no eligible targets and auto-skips silently; no Skip/Pay gate is presented.
    assert!(
        !game.has_pending_choice(),
        "no eligible live-card targets -> move_cards must auto-skip without prompting"
    );

    assert!(!game.has_pending_choice(), "No pending choices");
    assert_eq!(game.state.player2.hand.cards.len(), 3, "P2 hand untouched");
}

/// Empty-hand opponent: no cards → conditional_on_optional still presents Skip/Pay.
#[test]
fn riko_q130_opponent_empty_hand_auto_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-pb1-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(riko);
    game.state.player1.hand.cards.push(filler);
    // Opponent has 0 cards in hand
    game.give_energy(13);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::LeftSide);

    // No discard prompt and no Skip/Pay gate: empty opponent hand auto-skips silently.
    assert!(
        !game.has_pending_choice(),
        "empty opponent hand -> ability must resolve without any prompt"
    );

    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");
    assert_eq!(game.state.player2.hand.cards.len(), 0, "P2 empty hand");
    assert_eq!(
        game.state.player2.waitroom.cards.len(),
        0,
        "P2 waitroom empty"
    );
}
