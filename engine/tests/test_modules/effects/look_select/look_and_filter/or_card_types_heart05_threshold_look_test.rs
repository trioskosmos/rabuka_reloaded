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

    // Answer what cannot be declined, decline what can: a mandatory prompt

    // left parked mid-ability, and the absence assertion that usually

    // follows is exactly what a wrong answer produces.

    game.drain_choices();



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

    // Answer what cannot be declined, decline what can: a mandatory prompt

    // left parked mid-ability, and the absence assertion that usually

    // follows is exactly what a wrong answer produces.

    game.drain_choices();



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

    // Answer what cannot be declined, decline what can: a mandatory prompt

    // left parked mid-ability, and the absence assertion that usually

    // follows is exactly what a wrong answer produces.

    game.drain_choices();



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

/// The LIVE branch's threshold. 津島善子's text reads
/// 「ハートにheart05を2個以上持つメンバーカード**か、**必要ハートにheart05を2以上含む
/// ライブカードを…」 — two branches, two different heart SOURCES, and both carry a count
/// of 2.
///
/// The member branch has a below-threshold test (`yoshiko_look_select_heart05_1_rejected`,
/// 渡辺 曜 with base heart05=1). The live branch had none: its only fixture,
/// `PL!S-PR-023-PR`, sits well above the bound, so a live printing need-heart05 = 1 would
/// wrongly qualify and nothing in the tree would notice. `PL!HS-bp2-019-L` Bloom the smile,
/// Bloom the dream! prints need heart05 = 1.
#[test]
fn yoshiko_look_select_live_with_need_heart05_one_is_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Pin the premise from the card data: this live's REQUIRED heart05 is 1, so it sits
    // one below the printed 「必要ハートにheart05を2以上」.
    let low_need_live = game.id("PL!HS-bp2-019-L");
    game.assert_card_identity(low_need_live, "PL!HS-bp2-019-L");
    assert_eq!(
        game.db
            .get_card(low_need_live)
            .and_then(|c| c.need_heart.as_ref())
            .and_then(|h| h.hearts.get(&rabuka_engine::card::parse_heart_color("heart05")))
            .copied(),
        Some(1),
        "precondition: Bloom the smile must print need heart05 = 1, one below the threshold, \
         or this test would pass on a card that should have qualified anyway"
    );

    let cost_fodder = game.id("PL!-sd1-010-SD");
    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![low_need_live]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);

    assert!(
        game.has_pending_choice(),
        "the or_card_types prompt precedes the filter, so it opens even with nothing eligible"
    );
    // Pick the LIVE branch (index 0).
    game.select_option(0);
    assert!(
        !game.has_pending_choice(),
        "必要ハートにheart05を2以上含む — a live printing need heart05 = 1 is below the bound, \
         so the live branch has no candidate and must resolve without a selection"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&low_need_live),
        "the sub-threshold live must not reach hand (hand={:?})",
        game.state.player1.hand.cards
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&low_need_live),
        "残りを控え室に置く — the looked-at live that could not be taken must still be \
         accounted for, not silently lost (hand={:?} waitroom={:?})",
        game.state.player1.hand.cards, game.state.player1.waitroom.cards
    );
}

/// A card that fails the filter must be *accounted for*, not merely absent from hand.
///
/// `yoshiko_look_select_no_eligible_auto_skip` and `yoshiko_look_select_heart05_1_rejected`
/// only assert that no prompt remains. A regression that DROPPED the looked-at card
/// altogether — vanishing it from every zone rather than routing it to the waitroom —
/// would satisfy both. The printed text commits the card to 控え室, so that is asserted here.
#[test]
fn yoshiko_look_select_ineligible_looked_at_card_is_routed_to_the_waitroom_not_lost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // 渡辺 曜: base heart05 = 1, one below the member branch's 2.
    let below = game.id("PL!S-PR-017-PR");
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![below]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);
    pay_optional_cost(&mut game);
    game.select_option(1); // member branch
    assert!(
        !game.has_pending_choice(),
        "precondition: nothing meets 2個以上, so the look resolves without a selection"
    );

    assert!(
        !game.state.player1.hand.cards.contains(&below),
        "the sub-threshold member must not be taken (hand={:?})",
        game.state.player1.hand.cards
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&below),
        "残りを控え室に置く — the looked-at card must land in the waitroom, not disappear. \
         (hand={:?} waitroom={:?} deck_len={})",
        game.state.player1.hand.cards,
        game.state.player1.waitroom.cards,
        game.state.player1.main_deck.cards.len()
    );
}

/// Declining the optional cost must not fire the ability at all.
///
/// `yoshiko_look_select_skip_cost` only checks that no prompt is left. If the ability ran
/// anyway and auto-resolved — a qualifying member going straight to hand with no prompt to
/// decline — the test would still pass. 「〜てもよい」 declined means the look never happens,
/// so the deck must be untouched.
#[test]
fn yoshiko_declining_the_optional_cost_leaves_the_deck_untouched() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // 桜内梨子: base heart05 = 2, so it WOULD qualify if the ability ran.
    let qualifying = game.id("PL!S-PR-014-PR");
    let cost_fodder = game.id("PL!-sd1-010-SD");

    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    let card = setup_yoshiko_test(&mut game, vec![qualifying]);
    game.state.player1.hand.cards.push(cost_fodder);
    game.play_to_stage(card, MemberArea::Center);

    let deck_before = game.state.player1.main_deck.cards.len();
    assert!(game.has_pending_choice(), "the optional cost is offered first");
    game.select_indices(&[]); // decline

    assert!(
        !game.has_pending_choice(),
        "declining the cost must end the ability with nothing further to answer"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&qualifying),
        "declining 「〜てもよい」 means the look never happens, so the qualifying member must \
         stay in the deck (hand={:?})",
        game.state.player1.hand.cards
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&qualifying),
        "the card must still be in the deck, undealt (deck_len {} vs {} before)",
        game.state.player1.main_deck.cards.len(),
        deck_before
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "a declined optional cost must not move a single card"
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

    // Answer what cannot be declined, decline what can: a mandatory prompt

    // left parked mid-ability, and the absence assertion that usually

    // follows is exactly what a wrong answer produces.

    game.drain_choices();



    assert!(
        game.state.player1.waitroom.cards.contains(&filler),
        "Non-selected looked-at cards go to waitroom"
    );
}
