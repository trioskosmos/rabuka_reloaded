use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn make_test_game() -> TestGame {
    let db = load_real_database();
    TestGame::new(db)
}

// =========================================================================
// LL-bp2-001-R+ — 渡辺 曜&鬼塚夏美&大沢瑠璃乃 (ab#0)
// 手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚につき、1少なくなる。
//
// PARSER FIX: emits a location_condition { location: "hand" } so the engine
//   knows this ability only activates when the card is in hand.
// ENGINE FIX: calculate_play_cost_reduction's stage card path checks the
//   effect's condition — if it requires hand, skip the effect (card is on
//   stage, not in hand). Also handles per_unit calculation properly.
// =========================================================================

/// Card in hand with 2 other cards: cost = base - (3-1)*1 = base - 2.
#[test]
fn hand_card_cost_reduced_by_hand_count_minus_1() {
    let mut game = make_test_game();
    let card = game.id("LL-bp2-001-R+");
    let filler = game.id("PL!-sd1-010-SD");

    // Card in hand with 2 other cards (total hand = 3)
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    game.state.recalculate_constants();

    // Get the card's base cost
    let db_card = game.db.get_card(card).unwrap();
    let base_cost = db_card.cost.unwrap();
    let hand_count = game.state.player1.hand.cards.len();
    let expected_reduction = (hand_count.saturating_sub(1) * 1) as u32;

    // Verify the cost modifier stored by recalculate_constants
    // (modifiers.rs stores per-card cost bonuses for display/condition eval)
    let cost_mod = game
        .state
        .mods
        .constant_cost_bonuses
        .get(&card)
        .copied()
        .unwrap_or(0);
    // cost_mod is i32, stored as -2 for "subtract 2 from cost"
    assert_eq!(
        cost_mod,
        -(expected_reduction as i32),
        "card's cost should be reduced by hand_count-1 = {} (base {}, hand {}, got {})",
        expected_reduction,
        base_cost,
        hand_count,
        cost_mod
    );
}

/// Card in hand alone (only itself): cost = base - (1-1)*1 = base (no reduction).
#[test]
fn hand_card_alone_no_reduction() {
    let mut game = make_test_game();
    let card = game.id("LL-bp2-001-R+");
    let filler = game.id("PL!-sd1-010-SD");

    // Only the card itself in hand
    game.state.player1.hand.cards.push(card);
    fill_decks(&mut game, filler);
    game.state.recalculate_constants();

    let hand_count = game.state.player1.hand.cards.len();
    assert_eq!(hand_count, 1, "hand should have exactly 1 card (this card)");

    let cost_mod = game
        .state
        .mods
        .constant_cost_bonuses
        .get(&card)
        .copied()
        .unwrap_or(0);
    assert_eq!(cost_mod, 0, "no reduction when card is alone in hand");
}

/// Card on stage with the ability: should NOT give cost reduction to OTHER cards
/// being played from hand. Verifies the engine fix in calculate_play_cost_reduction.
#[test]
fn on_stage_card_does_not_reduce_other_cards_cost() {
    let mut game = make_test_game();
    let card = game.id("LL-bp2-001-R+");
    let other = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Card on stage, another card in hand (to attempt to play)
    game.state.player1.stage.stage = [filler, card, filler];
    game.state.player1.hand.cards.push(other);
    fill_decks(&mut game, filler);

    let db_other = game.db.get_card(other).unwrap();
    let base_cost = db_other.cost.unwrap(); // cost=4

    // Calculate play cost reduction for `other` while `card` is on stage
    let reduction = rabuka_engine::ability::util::calculate_play_cost_reduction(
        &game.state.player1.stage,
        &game.state.player1.success_live_card_zone.cards,
        game.state.player1.hand.cards.len(),
        other,
        &game.db,
    );

    assert_eq!(
        reduction, 0,
        "card on stage with hand-cost-reduction ability should NOT reduce other cards' cost (base {}, reduction {})",
        base_cost, reduction
    );
}

/// Card on stage + card in hand with extra cards: the stage card's ability
/// should NOT affect the hand card's cost calculation.
#[test]
fn on_stage_card_ignores_hand_for_other_card_cost() {
    let mut game = make_test_game();
    let card = game.id("LL-bp2-001-R+");
    let other = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, card, filler];
    game.state.player1.hand.cards.push(other);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);

    // 4 cards in hand: other + 3 fillers. Stage card should NOT give reduction.
    let reduction = rabuka_engine::ability::util::calculate_play_cost_reduction(
        &game.state.player1.stage,
        &game.state.player1.success_live_card_zone.cards,
        game.state.player1.hand.cards.len(),
        other,
        &game.db,
    );

    assert_eq!(
        reduction, 0,
        "no cost reduction from stage card even with many hand cards"
    );
}
