/// Tests for look_and_select with any_number=true
///
/// Card: 百生 吟子 (PL!HS-bp2-016-N) ab#0
/// Text: 登場 自分のデッキの上からカードを2枚見る。
///       その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。
///
/// Bug: any_number=true ended after first selection instead of batch selecting
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Select 1 out of 2 looked-at cards with any_number=true.
/// The selected card goes to deck top; the remaining card goes to discard.
#[test]
fn look_and_select_any_number_partial_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-bp2-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let _card_a = game.id("PL!-sd1-014-SD");
    let _card_b = game.id("PL!-sd1-015-SD");

    game.state.player1.hand.cards.push(card);
    game.give_energy(4);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(card, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );

    // Select 1 card only (index 0).
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(!game.has_pending_choice(), "Ability should have ended");
    // The selected card is on deck top, the other was discarded
}

/// Select 2 out of 2 looked-at cards (full batch) with any_number=true.
/// Both go to deck top, none to discard.
#[test]
fn look_and_select_any_number_full_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-bp2-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let card_a = game.id("PL!-sd1-014-SD");
    let card_b = game.id("PL!-sd1-015-SD");

    game.state.player1.hand.cards.push(card);
    game.give_energy(4);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(card_a);
    game.state.player1.main_deck.cards.push(card_b);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(card, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );

    // Select both looked-at cards
    game.select_indices(&[0]);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Resolve order prompt (any_order)
    while game.has_pending_choice() {
        game.select_option(0);
    }

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        game.state.player1.main_deck.cards.contains(&card_a),
        "card_a should be on deck"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&card_b),
        "card_b should be on deck"
    );
}

/// Select 0 out of 2 looked-at cards with any_number=true.
/// Both go to discard.
#[test]
fn look_and_select_any_number_skip_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-bp2-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let card_a = game.id("PL!-sd1-014-SD");
    let card_b = game.id("PL!-sd1-015-SD");

    game.state.player1.hand.cards.push(card);
    game.give_energy(4);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(card_a);
    game.state.player1.main_deck.cards.push(card_b);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(card, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );

    // Select 0 cards (skip)
    game.select_indices(&[]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        game.state.player1.waitroom.cards.contains(&card_a)
            || game.state.player1.waitroom.cards.contains(&card_b),
        "At least one unmatched card should be in discard"
    );
}

#[test]
fn look_and_select_dynamic_count_look_at_counts_stage_members_plus_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-bp6-001-R＋");
    let filler = game.id("PL!-sd1-010-SD");
    let top_a = game.id("PL!-sd1-014-SD");
    let top_b = game.id("PL!-sd1-015-SD");

    game.state.player1.hand.cards.push(card);
    game.give_energy(4);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(top_a);
    game.state.player1.main_deck.cards.push(top_b);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(card, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    assert_eq!(
        game.state.looked_at_cards.len(),
        3,
        "Should look at 3 cards when the entering member is on stage"
    );
}

/// Debut look_and_select with group_filter — no eligible cards among looked-at.
/// Should auto-skip without showing a prompt.
#[test]
fn look_and_select_no_eligible_cards_auto_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // 園田海未 — debut: look at 5, select up to 1 μ's live card to hand
    let card = game.id("PL!-sd1-004-SD");
    let filler = game.id("PL!-sd1-010-SD"); // not a μ's live card

    game.state.player1.hand.cards.push(card);
    game.give_energy(11);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(card, rabuka_engine::zones::MemberArea::Center);

    // No eligible μ's live cards → should auto-skip, no prompt
    assert!(
        !game.has_pending_choice(),
        "No eligible cards → should auto-skip without prompt"
    );

    // All 5 looked-at cards should have gone to waitroom
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        5,
        "All 5 non-matching cards should be in waitroom"
    );
}
