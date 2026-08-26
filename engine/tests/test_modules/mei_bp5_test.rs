/// Tests for PL!SP-bp5-007-R (米女メイ) ab#0
///
/// Ability (登場):
///   手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。
///   その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。
///   残りを控え室に置く。
///
/// Parser outputs: look_and_select, count=3, max=true, optional=true,
///   reveal=true, per_group=true, per_group_count=1
use crate::helpers::*;

/// Pay the optional hand-discard cost. Observed on every Mei debut with a
/// hand card: SelectCard zone=hand count=1 allow_skip=true.
fn discard_cost_if_pending(game: &mut TestGame) {
    assert!(
        game.has_pending_choice(),
        "optional discard cost must be offered (hand card present)"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard skippable discard-cost prompt"
    );
    game.select_indices(&[0]);
}

fn discard_before(game: &TestGame) -> usize {
    game.state.player1.waitroom.cards.len()
}

fn select_and_finish(game: &mut TestGame, count: usize) {
    for i in 0..count {
        assert!(
            game.has_pending_choice(),
            "look_and_select pick #{} must be prompted",
            i + 1
        );
        assert_eq!(
            game.pending_choice_type().as_deref(),
            Some("SelectCard"),
            "expected SelectCard looked_at prompt"
        );
        game.select_indices(&[0]);
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// Setup: Mei + 1 filler in hand, 5 deck cards from 3 different series
fn setup_max_three() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mei = game.id("PL!SP-bp5-007-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(mei);
    game.state.player1.hand.cards.push(filler);

    // Deck: 5 cards from different series to allow selecting 3 (max 1 per series)
    // Index 0: Love Live! Superstar!! (Mei itself)
    // Index 1: Love Live! (sd1 filler)
    // Index 2-4: same as above, but won't be picked together due to per-group
    let sup = game.new_id("PL!SP-bp5-007-R");
    let ll1 = game.new_id("PL!-sd1-010-SD");
    let ll2 = game.new_id("PL!-sd1-010-SD");
    let ll3 = game.new_id("PL!-sd1-010-SD");
    let ll4 = game.new_id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(sup); // [0] Superstar!!
    game.state.player1.main_deck.cards.push(ll1); // [1] Love Live!
    game.state.player1.main_deck.cards.push(ll2); // [2] Love Live! (same series as [1])
    game.state.player1.main_deck.cards.push(ll3); // [3] Love Live!
    game.state.player1.main_deck.cards.push(ll4); // [4] Love Live!

    game.give_energy(15);
    game.play_to_stage(mei, rabuka_engine::zones::MemberArea::LeftSide);
    game
}

/// Select 2 cards from different series (max per-group = 1 each series)
#[test]
fn mei_bp5_select_two_from_different_series() {
    let mut game = setup_max_three();
    let disc_start = discard_before(&game);

    discard_cost_if_pending(&mut game);

    // Pick [0] = Superstar!! and then next remaining [0] = Love Live! (different series → allowed)
    select_and_finish(&mut game, 2);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        disc_start + 4, // 1 cost + 3 unselected = 4
        "Discard: 1 cost + 3 remaining"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2, // 2 start - 1 stage - 1 cost + 2 selected = 2
        "2 cards in hand from different series"
    );
}

/// Select 0 cards (skip), verify 0 added, all 5 + 1 cost go to discard
#[test]
fn mei_bp5_select_zero_cards() {
    let mut game = setup_max_three();
    let disc_start = discard_before(&game);

    discard_cost_if_pending(&mut game);
    select_and_finish(&mut game, 0);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        disc_start + 6, // 1 cost + 5 all = 6
        "All 5 + 1 cost discarded when skipping"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0, // 2 start - 1 stage - 1 cost = 0
        "No cards in hand when skipping"
    );
}

/// Select 1 card, verify 1 in hand, 4 + 1 cost go to discard
#[test]
fn mei_bp5_select_one_card() {
    let mut game = setup_max_three();
    let disc_start = discard_before(&game);

    discard_cost_if_pending(&mut game);
    select_and_finish(&mut game, 1);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        disc_start + 5, // 1 cost + 4 remaining = 5
        "Discard: 1 cost + 4 unselected"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1, // 2 - 1 stage - 1 cost + 1 selected = 1
        "1 card in hand"
    );
}

/// Per-group constraint: selecting 2 cards from the same series is rejected
#[test]
fn mei_bp5_per_group_rejects_two_from_same_group() {
    let mut game = setup_max_three();

    discard_cost_if_pending(&mut game);

    // Try to pick 2 cards from Love Live! series (indices [1, 2]).
    // Observed: SelectCard zone=looked_at count=3 is prompted and the
    // engine rejects the same-group pair.
    assert!(
        game.has_pending_choice(),
        "look_and_select prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard looked_at prompt"
    );
    let result = game.try_select_indices(&[1, 2]);
    assert!(
        result.is_err(),
        "Per-group should reject 2 cards from same series"
    );
    // After rejection, no further choices (ability terminated).
    assert!(
        !game.has_pending_choice(),
        "No pending choice after rejection"
    );
}

/// Q235: Multi-name card should be selectable
#[test]
fn mei_bp5_q235_debut_look_and_select_with_multiname() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mei = game.id("PL!SP-bp5-007-R");
    let filler = game.id("PL!-sd1-010-SD");
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");

    game.state.player1.hand.cards.push(mei);
    game.state.player1.hand.cards.push(filler);

    // Deck: multiname (LL-bp1 series) + 1 love live filler + 3 more fillers
    game.state.player1.main_deck.cards.push(multiname); // [0] LL-bp1 series
    game.state.player1.main_deck.cards.push(filler); // [1] sd1 series
    game.state.player1.main_deck.cards.push(filler); // [2-4] same series as [1]
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    game.give_energy(15);
    game.play_to_stage(mei, rabuka_engine::zones::MemberArea::LeftSide);

    discard_cost_if_pending(&mut game);

    // Pick at most 1 per series: [0] (LL-bp1) + remaining [0] (sd1) = 2 from different series
    select_and_finish(&mut game, 2);

    let multiname_in_hand = game.state.player1.hand.cards.contains(&multiname);
    assert!(
        multiname_in_hand,
        "Multi-name card should be selectable and added to hand"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&multiname),
        "Multi-name card should no longer be in the deck"
    );
}
