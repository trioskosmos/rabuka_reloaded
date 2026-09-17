/// Tests for look_and_select with or_card_types + heart-count threshold (niche filter):
/// 4枚見る → ハート条件を満たすメンバー/ライブカードを1枚公開して手札に加えてもよい。
/// 残りを控え室に置く。Branches: member/live take, no-eligible auto-skip,
/// below-threshold rejection, both-types prompt, cost skip, remainder routing.
///
/// Card: 津島善子 (PL!S-pb1-015-N)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

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
    assert!(
        game.has_pending_choice(),
        "optional discard-from-hand cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard cost prompt (zone=hand, allow_skip)"
    );
    // Select index 0 (the filler card in hand)
    game.select_indices(&[0]);
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
