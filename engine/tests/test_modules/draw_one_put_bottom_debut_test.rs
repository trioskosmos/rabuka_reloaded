/// Tests for the debut ability:
///   「カードを1枚引き、手札を1枚デッキの一番下に置く。」
///   (Draw 1 card, then place 1 card from your hand on the bottom of your deck.)
///
/// Cards with this exact text (card_count = 3):
///   PL!S-bp5-014-N  | 渡辺 曜 (cost 4)
///   PL!S-sd1-017-SD | 小原鞠莉 (cost 4)
///   PL!S-sd1-018-SD | 黒澤ルビィ (cost 4)
///
/// Parsed effect (from cards/abilities.json):
///   sequential [
///     draw_card(deck→hand, count 1),
///     move_cards(hand→deck_bottom, count 1, card_type card)
///   ]
use crate::helpers::*;
use rabuka_engine::ability::enums::ActionType;
use rabuka_engine::zones::MemberArea;

/// Fill both players' decks with `count` distinct filler copies.
/// Returns the P1 deck snapshot (index 0 = top) so tests can assert order.
fn fill_decks_distinct(game: &mut TestGame, count: usize) -> Vec<i16> {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    let mut snapshot = Vec::with_capacity(count);
    for _ in 0..count {
        let f = game.new_id("PL!-sd1-010-SD");
        snapshot.push(f);
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
    snapshot
}

/// Drain any incidental auto-ability prompts (shouldn't occur for these cards).
fn drain_auto(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => break,
        }
    }
}

// =========================================================================
// Happy path / core behavior
// =========================================================================

/// Basic flow: play 渡辺曜 → draw 1 → prompt to choose 1 hand card →
/// the chosen card lands on the bottom of the deck, order above preserved.
#[test]
fn you_draw_one_put_one_on_bottom_basic() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let card_a = game.id("PL!-sd1-001-SD"); // member
    let card_b = game.id("PL!-sd1-020-SD"); // live

    game.add_to_hand(you);
    game.add_to_hand(card_a);
    game.add_to_hand(card_b);
    let deck = fill_decks_distinct(&mut game, 20);
    let drawn_top = deck[0];

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    // Debut fired: draw already happened. Hand = [card_a, card_b, drawn_top].
    assert!(
        game.has_pending_choice(),
        "hand has 3 cards after draw → selection prompt expected"
    );
    game.assert_select_card("hand", 1, false);
    game.select_indices(&[0]); // card_a → deck bottom

    let hand = &game.state.player1.hand.cards;
    let p1_deck = &game.state.player1.main_deck.cards;
    assert_eq!(hand.len(), 2, "hand: -1 play, +1 draw, -1 bottom = 2");
    assert!(hand.contains(&card_b), "card_b stays in hand");
    assert!(hand.contains(&drawn_top), "drawn card stays in hand");
    assert!(!hand.contains(&card_a), "card_a moved out of hand");
    assert_eq!(p1_deck.len(), 20, "deck: -1 draw, +1 bottom = unchanged");
    assert_eq!(p1_deck.last(), Some(&card_a), "card_a on deck bottom");

    let actual_head: Vec<i16> = p1_deck[..19].iter().copied().collect();
    assert_eq!(actual_head, deck[1..20], "order above bottom preserved");
    assert!(!game.has_pending_choice(), "ability fully resolved");
}

/// The newly drawn card itself can be the one placed on the bottom.
#[test]
fn you_drawn_card_can_be_placed_on_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let card_b = game.id("PL!-sd1-020-SD");

    game.add_to_hand(you);
    game.add_to_hand(card_b);
    let deck = fill_decks_distinct(&mut game, 20);
    let drawn_top = deck[0];

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    // Hand after draw = [card_b, drawn_top].
    assert!(game.has_pending_choice(), "2 cards → prompt expected");
    game.select_indices(&[1]); // put the drawn card on the bottom

    let p1_deck = &game.state.player1.main_deck.cards;
    assert_eq!(p1_deck.last(), Some(&drawn_top), "drawn card on bottom");
    assert_eq!(p1_deck.len(), 20);
    let actual_head: Vec<i16> = p1_deck[..19].iter().copied().collect();
    assert_eq!(actual_head, deck[1..20], "order above bottom preserved");
    let hand: Vec<i16> = game.state.player1.hand.cards.iter().copied().collect();
    assert_eq!(hand, vec![card_b], "only card_b remains in hand");
    assert!(!game.has_pending_choice());
}

/// Hand with exactly 1 card after the draw → the move auto-resolves with no prompt.
#[test]
fn you_single_card_hand_auto_resolves_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    game.add_to_hand(you);
    let deck = fill_decks_distinct(&mut game, 20);
    let drawn_top = deck[0];

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    // Hand after draw = [drawn_top] → exactly 1 → auto-select to bottom.
    assert!(
        !game.has_pending_choice(),
        "exactly 1 eligible card → auto-resolved, no prompt"
    );
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "the only hand card was placed on the bottom"
    );
    let p1_deck = &game.state.player1.main_deck.cards;
    assert_eq!(p1_deck.len(), 20);
    assert_eq!(p1_deck.last(), Some(&drawn_top), "drawn card on bottom");
    let actual_head: Vec<i16> = p1_deck[..19].iter().copied().collect();
    assert_eq!(actual_head, deck[1..20]);
}

/// With many cards in hand, only exactly 1 is moved to the bottom; the rest stay.
#[test]
fn you_only_one_hand_card_moved_rest_stay() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let mut keepers = Vec::new();
    for _ in 0..4 {
        let f = game.new_id("PL!-sd1-010-SD");
        keepers.push(f);
        game.add_to_hand(f);
    }
    let target = game.new_id("PL!-sd1-001-SD");
    game.add_to_hand(you);
    game.add_to_hand(target);
    let deck = fill_decks_distinct(&mut game, 20);

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    // Hand = [keeper0..3, target, drawn_top].
    let target_idx = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .position(|&c| c == target)
        .expect("target in hand");
    assert!(game.has_pending_choice(), "6 cards → prompt expected");
    game.assert_select_card("hand", 1, false);
    game.select_indices(&[target_idx]);

    let hand = &game.state.player1.hand.cards;
    assert_eq!(hand.len(), 5, "6 - 1 = 5");
    for k in &keepers {
        assert!(hand.contains(k), "keeper still in hand");
    }
    assert!(hand.contains(&deck[0]), "drawn card still in hand");
    assert!(!hand.contains(&target));
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&target),
        "exactly the selected card is on the bottom"
    );
}

// =========================================================================
// Choice inspection
// =========================================================================

/// The hand→bottom move is mandatory: allow_skip must be false, count 1, zone hand.
#[test]
fn you_hand_move_is_mandatory_not_skippable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    game.add_to_hand(you);
    game.add_to_hand(game.new_id("PL!-sd1-001-SD"));
    game.add_to_hand(game.new_id("PL!-sd1-020-SD"));
    fill_decks_distinct(&mut game, 20);

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    assert!(game.has_pending_choice());
    game.assert_select_card("hand", 1, false);

    // No skip action is offered for the mandatory move.
    let actions = game.generated_actions();
    assert!(
        !actions
            .iter()
            .any(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoiceSkip),
        "mandatory hand→bottom move must not offer a skip"
    );
}

/// card_type "card" means all card types (member / live / energy) are selectable.
#[test]
fn you_selection_includes_all_card_types() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let member = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-020-SD");
    let energy = game.id("LL-E-001-SD");

    game.add_to_hand(you);
    game.add_to_hand(member);
    game.add_to_hand(live);
    game.add_to_hand(energy);
    fill_decks_distinct(&mut game, 20);

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    assert!(game.has_pending_choice());
    game.assert_selection_contains("PL!-sd1-001-SD", "高坂 穂乃果");
    game.assert_selection_contains("PL!-sd1-020-SD", "きっと青春が聞こえる");
    game.assert_selection_contains("LL-E-001-SD", "エネルギーカード");

    let json = game
        .state
        .get_pending_choice_json()
        .expect("pending choice JSON");
    let cards = json["selection_cards"]
        .as_array()
        .expect("selection_cards present");
    assert_eq!(
        cards.len(),
        4,
        "all 4 hand cards (3 types + drawn) eligible"
    );
}

// =========================================================================
// Deck / draw edge cases
// =========================================================================

/// Empty deck + non-empty waitroom: the draw refreshes from the waitroom (Q104 /
/// Rule 10.2.1), then the hand→bottom move still resolves.
#[test]
fn you_empty_deck_refreshes_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let card_a = game.id("PL!-sd1-001-SD");
    game.add_to_hand(you);
    game.add_to_hand(card_a);

    // No deck for either player.
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    // 10 fillers in P1 waitroom → refresh material.
    for _ in 0..10 {
        game.state
            .player1
            .waitroom
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    // Draw refreshed 10 → 9; hand now [card_a, drawn]. Prompt appears.
    assert!(
        game.has_pending_choice(),
        "2 hand cards after refresh-draw → prompt expected"
    );
    game.select_indices(&[0]); // card_a → bottom

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "hand = just the refreshed draw"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        10,
        "deck = 10 refreshed - 1 drawn + 1 placed"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&card_a),
        "card_a on bottom of refreshed deck"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "waitroom fully consumed by refresh"
    );
    assert!(!game.has_pending_choice());
}

/// Empty deck AND empty waitroom: the draw silently fails, but the hand→bottom
/// move is still executed (the sequential steps are independent).
#[test]
fn you_empty_deck_and_waitroom_draw_fails_move_still_happens() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let card_a = game.id("PL!-sd1-001-SD");
    game.add_to_hand(you);
    game.add_to_hand(card_a);

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player2.waitroom.cards.clear();

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    // Draw added nothing; hand = [card_a] → exactly 1 → auto-place on bottom.
    assert!(!game.has_pending_choice(), "auto-resolved");
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "card_a moved out of hand"
    );
    let deck_cards: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();
    assert_eq!(
        deck_cards,
        vec![card_a],
        "card_a is the only card in deck (bottom)"
    );
    assert!(!game.has_pending_choice());
}

/// Deck has exactly 1 card: it is drawn (deck → empty), then the hand card is
/// pushed onto the now-empty deck bottom.
#[test]
fn you_draw_last_deck_card_then_place_on_empty_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let card_a = game.id("PL!-sd1-001-SD");
    let top = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(you);
    game.add_to_hand(card_a);
    game.state.player1.main_deck.cards.push(top);
    game.state.player2.main_deck.cards.clear();

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);

    assert!(game.has_pending_choice(), "hand = [card_a, top] → prompt");
    game.select_indices(&[0]); // card_a → bottom of empty deck

    let p1_deck = &game.state.player1.main_deck.cards;
    assert_eq!(p1_deck.len(), 1, "1 placed on empty deck");
    assert_eq!(p1_deck.last(), Some(&card_a), "card_a on bottom");
    let hand: Vec<i16> = game.state.player1.hand.cards.iter().copied().collect();
    assert_eq!(
        hand,
        vec![top],
        "the drawn last deck card is now the only hand card"
    );
    assert!(!game.has_pending_choice());
}

// =========================================================================
// Opponent isolation / cost
// =========================================================================

/// The ability only touches P1's zones; P2's hand and deck are untouched.
#[test]
fn you_opponent_zones_unaffected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    let card_a = game.id("PL!-sd1-001-SD");
    let card_b = game.id("PL!-sd1-020-SD");
    game.add_to_hand(you);
    game.add_to_hand(card_a);
    game.add_to_hand(card_b);
    let deck = fill_decks_distinct(&mut game, 20);

    let p2_hand: Vec<i16> = game.state.player2.hand.cards.iter().copied().collect();
    let p2_deck: Vec<i16> = game.state.player2.main_deck.cards.iter().copied().collect();

    game.give_energy(4);
    game.play_to_stage(you, MemberArea::Center);
    drain_auto(&mut game);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    assert_eq!(
        game.state
            .player2
            .hand
            .cards
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        p2_hand,
        "P2 hand unchanged"
    );
    assert_eq!(
        game.state
            .player2
            .main_deck
            .cards
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        p2_deck,
        "P2 deck unchanged"
    );
    assert!(!deck.contains(&card_a) || game.state.player1.main_deck.cards.contains(&card_a));
}

/// Cost 4 is required to play the member; with only 3 energy the play fails.
#[test]
fn you_insufficient_energy_cannot_play() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp5-014-N");
    game.add_to_hand(you);
    fill_decks_distinct(&mut game, 20);
    game.give_energy(3);

    let err = game
        .try_play_to_stage(you, MemberArea::Center)
        .expect_err("cost 4 with only 3 energy must fail");
    assert!(
        err.contains("energy") || err.contains("cost"),
        "error should mention energy/cost, got: {err}"
    );
    assert!(
        game.state.player1.hand.cards.contains(&you),
        "card stays in hand after failed play"
    );
}

// =========================================================================
// All 3 cards share the identical behavior
// =========================================================================

/// 小原鞠莉 (PL!S-sd1-017-SD) has the same debut behavior.
#[test]
fn mari_draw_one_put_one_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-sd1-017-SD");
    let card_a = game.id("PL!-sd1-001-SD");
    let card_b = game.id("PL!-sd1-020-SD");
    game.add_to_hand(mari);
    game.add_to_hand(card_a);
    game.add_to_hand(card_b);
    let deck = fill_decks_distinct(&mut game, 20);
    let drawn_top = deck[0];

    game.give_energy(4);
    game.play_to_stage(mari, MemberArea::Center);
    drain_auto(&mut game);

    assert!(game.has_pending_choice());
    game.assert_select_card("hand", 1, false);
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&card_a),
        "mari: selected card on bottom"
    );
    assert!(game.state.player1.hand.cards.contains(&drawn_top));
    assert_eq!(game.state.player1.hand.cards.len(), 2);
}

/// 黒澤ルビィ (PL!S-sd1-018-SD) has the same debut behavior.
#[test]
fn ruby_draw_one_put_one_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-sd1-018-SD");
    let card_a = game.id("PL!-sd1-001-SD");
    let card_b = game.id("PL!-sd1-020-SD");
    game.add_to_hand(ruby);
    game.add_to_hand(card_a);
    game.add_to_hand(card_b);
    let deck = fill_decks_distinct(&mut game, 20);
    let drawn_top = deck[0];

    game.give_energy(4);
    game.play_to_stage(ruby, MemberArea::Center);
    drain_auto(&mut game);

    assert!(game.has_pending_choice());
    game.assert_select_card("hand", 1, false);
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&card_a),
        "ruby: selected card on bottom"
    );
    assert!(game.state.player1.hand.cards.contains(&drawn_top));
    assert_eq!(game.state.player1.hand.cards.len(), 2);
}

/// Parser audit: all 3 cards share the exact same parsed effect structure.
#[test]
fn all_three_cards_parse_to_identical_sequential() {
    let db = load_real_database();
    for card_no in ["PL!S-bp5-014-N", "PL!S-sd1-017-SD", "PL!S-sd1-018-SD"] {
        let tid = db.get_card_id(card_no).unwrap();
        let card = db.get_card(tid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("登場"))
            .unwrap_or_else(|| panic!("{card_no} should have a 登場 ability"));

        assert_eq!(
            ab.effect.as_ref().map(|e| e.action),
            Some(ActionType::Sequential),
            "{card_no}: effect is sequential"
        );
        let effect = ab.effect.as_ref().unwrap();
        let actions = effect
            .compound
            .actions
            .as_ref()
            .expect("sequential has actions");
        assert_eq!(actions.len(), 2, "{card_no}: two sub-actions");

        // Step 1: draw 1 card from deck to hand.
        assert_eq!(
            actions[0].action,
            ActionType::DrawCard,
            "{card_no}: step 1 draw"
        );
        assert_eq!(actions[0].count_any(), Some(1), "{card_no}: draw 1");
        assert_eq!(
            actions[0].source_any(),
            Some("deck"),
            "{card_no}: from deck"
        );
        assert_eq!(
            actions[0].destination_any(),
            Some("hand"),
            "{card_no}: to hand"
        );

        // Step 2: move 1 card from hand to deck_bottom.
        assert_eq!(
            actions[1].action,
            ActionType::MoveCards,
            "{card_no}: step 2 move_cards"
        );
        assert_eq!(actions[1].count_any(), Some(1), "{card_no}: move 1");
        assert_eq!(
            actions[1].source_any(),
            Some("hand"),
            "{card_no}: from hand"
        );
        assert_eq!(
            actions[1].destination_any(),
            Some("deck_bottom"),
            "{card_no}: to deck bottom"
        );
    }
}
