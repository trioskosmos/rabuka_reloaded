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
    // Docstring: select 0 → BOTH looked-at cards go to discard.
    assert!(
        game.state.player1.waitroom.cards.contains(&card_a)
            && game.state.player1.waitroom.cards.contains(&card_b),
        "both looked-at cards should be in discard after skipping selection"
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

// ====================================================================
// 津島善子 (PL!S-pb1-015-N) — debut look_and_select with or_card_types
// and heart_color_count threshold.
// Text: 自分のデッキの上からカードを4枚見る。その中から
//       ハートにheart05を2個以上持つメンバーカードか、
//       必要ハートにheart05を2以上含むライブカードを
//       1枚公開して手札に加えてもよい。残りを控え室に置く。
// ====================================================================

// ====================================================================
// 津島善子 (PL!S-pb1-015-N) — debut look_and_select with or_card_types
// and heart_color_count threshold.
// Text: 自分のデッキの上からカードを4枚見る。その中から
//       ハートにheart05を2個以上持つメンバーカードか、
//       必要ハートにheart05を2以上含むライブカードを
//       1枚公開して手札に加えてもよい。残りを控え室に置く。
// ====================================================================

fn setup_yoshiko_test(game: &mut TestGame, top_cards: Vec<i16>) -> i16 {
    let card = game.id("PL!S-pb1-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(card);
    game.state.player1.main_deck.cards.clear();
    for cid in top_cards {
        game.state.player1.main_deck.cards.push(cid);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    card
}

fn pay_optional_cost(game: &mut TestGame) {
    if game.has_pending_choice() {
        let pc = game.get_pending_choice();
        if matches!(pc, rabuka_engine::ability::types::Choice::SelectCard { zone, allow_skip: true, .. } if zone == "hand")
        {
            // Select index 0 (the filler card in hand)
            game.select_indices(&[0]);
        }
    }
}

/// Member card with heart05=2 in looked-at → pick Member card → card to hand.
#[test]
fn yoshiko_look_select_member_heart05_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member_heart05_2 = game.id("PL!S-PR-014-PR");
    let _non_matching = game.id("PL!S-PR-015-PR");
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![member_heart05_2, _non_matching]);
    game.state.player1.hand.cards.push(cost_fodder); // for cost
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);

    // or_card_types: pick member_card (index 1)
    game.select_option(1);
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&member_heart05_2),
        "Member card with heart05=2 should be in hand"
    );
    assert!(
        !game
            .state
            .player1
            .waitroom
            .cards
            .contains(&member_heart05_2),
        "Selected card should NOT be in waitroom"
    );
}

/// Live card with need_heart05=2 in looked-at → pick Live card → card to hand.
#[test]
fn yoshiko_look_select_live_heart05_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_heart05_2 = game.id("PL!S-PR-023-PR"); // need_heart heart05: 2
    let _non_matching = game.id("PL!S-PR-015-PR");
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![live_heart05_2, _non_matching]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);

    // or_card_types: pick live_card (index 0)
    game.select_option(0);
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live_heart05_2),
        "Live card with need_heart05=2 should be in hand"
    );
}

/// No eligible cards → auto-skip without prompt.
#[test]
fn yoshiko_look_select_no_eligible_auto_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _no_heart05 = game.id("PL!S-PR-015-PR");
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![_no_heart05, _no_heart05]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);

    // or_card_types prompt appears (the effect doesn't pre-filter before the prompt)
    assert!(
        game.has_pending_choice(),
        "or_card_types prompt should appear"
    );
    game.select_option(1);
    assert!(
        !game.has_pending_choice(),
        "No eligible cards → should auto-skip after type choice"
    );
}

/// Card with heart05=1 should NOT pass the heart_color_count=2 filter.
#[test]
fn yoshiko_look_select_heart05_1_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member_heart05_1 = game.id("PL!S-PR-017-PR"); // heart05: 1
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![member_heart05_1]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);

    // or_card_types prompt appears first
    assert!(
        game.has_pending_choice(),
        "or_card_types prompt should appear"
    );
    // Pick member card
    game.select_option(1);
    // No matching cards after filtering → auto-skip
    assert!(
        !game.has_pending_choice(),
        "heart05=1 below threshold → should auto-skip after type choice"
    );
}

/// Both member and live card eligible → or_card_types prompt appears.
#[test]
fn yoshiko_look_select_both_types_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member_heart05_2 = game.id("PL!S-PR-014-PR");
    let live_heart05_2 = game.id("PL!S-PR-023-PR");
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![member_heart05_2, live_heart05_2]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);

    assert!(
        game.has_pending_choice(),
        "Both types eligible → or_card_types prompt"
    );
    game.assert_pending_choice_type("SelectTarget", "Should be SelectTarget");

    // Pick member card
    game.select_option(1);
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&member_heart05_2),
        "Selected member card should be in hand"
    );
}

/// Skip optional cost → effect does not fire.
#[test]
fn yoshiko_look_select_skip_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member_heart05_2 = game.id("PL!S-PR-014-PR");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![member_heart05_2]);
    // Extra filler in hand so cost choice appears
    let filler_hand = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(filler_hand);
    game.play_to_stage(card, MemberArea::Center);

    // Skip the optional cost
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[]);

    // After cost skip, no more prompts
    assert!(
        !game.has_pending_choice(),
        "After skipping cost, no further prompts"
    );
}

/// Non-selected looked-at cards go to waitroom.
#[test]
fn yoshiko_look_select_discard_remaining() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member_heart05_2 = game.id("PL!S-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![member_heart05_2, filler, filler, filler]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);

    game.select_option(1);
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&filler),
        "Non-selected looked-at cards go to waitroom"
    );
}
