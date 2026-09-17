/// Tests for look_and_select with any_number=true
///
/// Card: 百生 吟子 (PL!HS-bp2-016-N) ab#0
/// Text: 登場 自分のデッキの上からカードを2枚見る。
///       その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。
///
/// Bug: any_number=true ended after first selection instead of batch selecting
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn debut_ginko() -> (TestGame, [i16; 4]) {
    let mut game = TestGame::new(load_real_database());
    let cards = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent_card = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent_card].into();
    let ginko = game.id("PL!HS-bp2-016-N");
    game.add_to_hand(ginko);
    game.give_energy(4);
    game.play_to_stage(ginko, MemberArea::Center);
    assert!(game.state.player1.stage.stage.contains(&ginko));
    assert!(game.state.player1.hand.cards.is_empty());
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..2]);
    (game, cards)
}
#[test]
fn look_and_select_any_number_partial_selection() {
    let (mut game, [a, b, c, d]) = debut_ginko();
    game.select_indices(&[1]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a]);
    assert!(game.state.player1.hand.cards.is_empty());
}

/// Select 2 out of 2 looked-at cards (full batch) with any_number=true.
/// Both go to deck top, none to discard.
#[test]
fn look_and_select_any_number_full_selection() {
    let (mut game, [a, b, c, d]) = debut_ginko();
    game.select_indices(&[0]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, a, c, d]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
}

/// Select 0 out of 2 looked-at cards with any_number=true.
/// Both go to discard.
#[test]
fn look_and_select_any_number_skip_all() {
    let (mut game, [a, b, c, d]) = debut_ginko();
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, b]);
    assert!(game.state.player1.hand.cards.is_empty());
}

#[test]
fn look_and_select_dynamic_count_look_at_counts_stage_members_plus_two() {
    assert_kaho_debut_selection(0);
}

#[test]
fn kaho_two_own_members_look_four_and_keep_one() {
    assert_kaho_debut_selection(1);
}

#[test]
fn kaho_full_own_stage_looks_five_and_keeps_one() {
    assert_kaho_debut_selection(2);
}

fn assert_kaho_debut_selection(allies: usize) {
    let mut game = TestGame::new(load_real_database());
    let cards: [i16; 7] = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent_deck = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent_deck].into();
    for area in 0..3 {
        let opponent = game.id("PL!-sd1-014-SD");
        game.state.player2.stage.stage[area] = opponent;
    }
    let opponent_stage = game.state.player2.stage.stage;
    for area in [MemberArea::LeftSide, MemberArea::RightSide].into_iter().take(allies) {
        let ally = game.id("PL!-sd1-015-SD");
        game.add_to_stage(area, ally);
    }
    let kaho = game.id("PL!HS-bp6-001-R＋");
    game.add_to_hand(kaho);
    game.give_energy(4);
    game.play_to_stage(kaho, MemberArea::Center);
    let inspected = allies + 3;
    assert!(game.state.player1.stage.stage.contains(&kaho));
    game.assert_select_card("looked_at", 1, false);
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..inspected]);
    game.select_indices(&[inspected - 1]);
    assert!(!game.has_pending_choice());
    let mut expected_deck = vec![cards[inspected - 1]];
    expected_deck.extend_from_slice(&cards[inspected..]);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), expected_deck.as_slice());
    let mut actual_discard = game.state.player1.waitroom.cards.to_vec();
    let mut expected_discard = cards[..inspected - 1].to_vec();
    actual_discard.sort_unstable();
    expected_discard.sort_unstable();
    assert_eq!(actual_discard, expected_discard);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player2.stage.stage, opponent_stage);
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent_deck]);
    assert!(game.state.player2.waitroom.cards.is_empty());
}

#[test]
fn ginko_inspection_refresh_preserves_existing_top_and_recovers_waitroom() {
    let mut game = TestGame::new(load_real_database());
    let top = game.id("PL!-sd1-010-SD");
    let recycled = game.id("PL!-sd1-010-SD");
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards = vec![top].into();
    game.add_to_discard(recycled);
    game.state.player2.main_deck.cards = vec![opponent].into();
    let ginko = game.id("PL!HS-bp2-016-N");
    game.add_to_hand(ginko);
    game.give_energy(4);
    game.play_to_stage(ginko, MemberArea::Center);
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[top, recycled]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    game.select_indices(&[0]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[top]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[recycled]);
    assert!(game.state.player1.hand.cards.is_empty());
    assert!(game.state.player1.stage.stage.contains(&ginko));
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

#[test]
fn ginko_inspecting_exact_deck_size_does_not_refresh_waitroom() {
    let mut game = TestGame::new(load_real_database());
    let a = game.id("PL!-sd1-010-SD");
    let b = game.id("PL!-sd1-010-SD");
    let waiting = game.id("PL!-sd1-010-SD");
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards = vec![a, b].into();
    game.add_to_discard(waiting);
    game.state.player2.main_deck.cards = vec![opponent].into();
    let ginko = game.id("PL!HS-bp2-016-N");
    game.add_to_hand(ginko);
    game.give_energy(4);
    game.play_to_stage(ginko, MemberArea::Center);
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[a, b]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[waiting]);
    game.select_indices(&[1]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[a, b]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[waiting]);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

/// Debut look_and_select with group filter — 園田海未 (PL!-sd1-004-SD):
/// 「デッキの上から5枚見る。その中から『μ's』のライブカードを1枚公開して
///  手札に加えてもよい。残りを控え室に置く。」
/// No eligible μ's live card among the looked-at five → auto-skip (no prompt),
/// and ALL five go to the waitroom.
#[test]
fn look_and_select_no_eligible_cards_auto_skips() {
    let mut game = TestGame::new(load_real_database());
    let cards: [i16; 7] = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let umi = game.id("PL!-sd1-004-SD");
    game.add_to_hand(umi);
    game.give_energy(11);
    game.play_to_stage(umi, MemberArea::Center);
    assert!(!game.has_pending_choice(), "no eligible card → auto-skip");
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &cards[..5],
        "all five looked-at non-matching cards go to the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &cards[5..],
        "deck untouched below the looked-at five"
    );
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

/// Same printed ability, eligible branch: a μ's live card among the five is
/// taken to hand (公開 = revealed selection), the rest to the waitroom.
#[test]
fn look_and_select_takes_group_live_card_to_hand() {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!-sd1-020-SD");
    let mut cards: Vec<i16> = (0..5).map(|_| game.id("PL!-sd1-010-SD")).collect();
    cards.insert(2, live);
    game.state.player1.main_deck.cards = cards.clone().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let umi = game.id("PL!-sd1-004-SD");
    game.add_to_hand(umi);
    game.give_energy(11);
    game.play_to_stage(umi, MemberArea::Center);
    game.assert_select_card("looked_at", 1, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..5]);
    // Frontend protocol: indices are positions WITHIN filtered_indices
    // (only the μ's live card passes the group filter → position 0).
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[live]);
    let mut wr = game.state.player1.waitroom.cards.to_vec();
    let mut expected = [&cards[..2], &cards[3..5]].concat();
    wr.sort_unstable();
    expected.sort_unstable();
    assert_eq!(wr, expected);
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &cards[5..],
        "deck untouched below the looked-at five"
    );
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

/// Declined branch of the same printed ability: skip the 「てもよい」 pick →
/// nothing returns, ALL five looked-at cards go to the waitroom (残りを控え室に置く
/// applies even on decline — the remainder directive is explicit).
#[test]
fn look_and_select_skip_still_discards_remainder_to_waitroom() {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!-sd1-020-SD");
    let mut cards: Vec<i16> = (0..5).map(|_| game.id("PL!-sd1-010-SD")).collect();
    cards.insert(2, live);
    game.state.player1.main_deck.cards = cards.clone().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let umi = game.id("PL!-sd1-004-SD");
    game.add_to_hand(umi);
    game.give_energy(11);
    game.play_to_stage(umi, MemberArea::Center);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.hand.cards.is_empty());
    let mut wr = game.state.player1.waitroom.cards.to_vec();
    let mut expected = cards.clone();
    expected.truncate(5);
    wr.sort_unstable();
    expected.sort_unstable();
    assert_eq!(wr, expected, "declined pick still sends 残り to the waitroom");
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &cards[5..]);
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

// ====================================================================
// 絢瀬 絵里 (PL!-sd1-011-SD) — debut look_and_select with optional
// hand-discard cost. Text: 登場 手札を1枚控え室に置いてもよい：
// 自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、
// 残りを控え室に置く。
// ====================================================================

/// Accepted-cost branch: exactly 3 cards leave the deck, one is taken to
/// hand, the other two land in the waitroom. Pinned by card identity, not
/// by counts.
#[test]
fn eli_cost_look_three_takes_one_to_hand() {
    let mut game = TestGame::new(load_real_database());
    let a = game.id("PL!-sd1-010-SD");
    let b = game.id("PL!-sd1-020-SD");
    let c = game.id("PL!-sd1-014-SD");
    let d = game.id("PL!-sd1-015-SD");
    game.state.player1.main_deck.cards = vec![a, b, c, d].into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let eli = game.id("PL!-sd1-011-SD");
    game.add_to_hand(eli);
    let cost = game.id("PL!-sd1-020-SD");
    game.add_to_hand(cost);
    game.give_energy(12);
    game.play_to_stage(eli, MemberArea::Center);
    // Cost prompt first: discard 1 from hand (only the cost card is in hand).
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[0]);
    assert!(
        game.state.player1.stage.stage.contains(&eli),
        "eli stays on stage (only the cost card leaves)"
    );
    assert!(!game.state.player1.hand.cards.contains(&cost));
    // Look prompt over the top three.
    game.assert_select_card("looked_at", 1, false);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[a, b, c]);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert!(
        game.state.player1.hand.cards.contains(&a),
        "selected card added to hand"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &[cost, b, c],
        "cost card + unselected remainder hit the waitroom in order"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &[d],
        "deck untouched below the looked-at three"
    );
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

/// Declined-cost branch: skipping 「てもよい」 means NO look at all — deck,
/// hand and waitroom stay untouched.
#[test]
fn eli_declined_cost_skips_look_entirely() {
    let mut game = TestGame::new(load_real_database());
    let cards: [i16; 4] = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let eli = game.id("PL!-sd1-011-SD");
    game.add_to_hand(eli);
    let cost = game.id("PL!-sd1-020-SD");
    game.add_to_hand(cost);
    game.give_energy(12);
    game.play_to_stage(eli, MemberArea::Center);
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice(), "declined cost → effect never starts");
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[cost]);
    assert!(
        game.state.player1.stage.stage.contains(&eli),
        "eli stays on stage; nothing else moved"
    );
    assert!(game.state.player1.waitroom.cards.is_empty(), "declined cost discards nothing");
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &cards);
    assert!(game.state.looked_at_cards.is_empty());
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

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
