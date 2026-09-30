//! 東條 希 `PL!-bp3-007-R`:
//!
//!   {{live_start.png|ライブ開始時}}手札を2枚控え室に置いてもよい：自分のデッキの上から
//!   カードを3枚**見る**。その中から1枚を**手札**に加え、1枚を**デッキの上**に置**き**、
//!   1枚を**控え室**に置**く**。
//!
//! ## What the previous version of this file actually checked
//!
//! Two COUNTS and one inequality:
//!
//! ```text
//! deck     == deck_before - 2      correct count
//! waitroom == waitroom_before + 3  correct count
//! hand     <= 3                    passes for 0, 1, 2 or 3
//! ```
//!
//! `hand_now <= 3` was the hole. Paying 2 of 2 hand cards and taking 1 back leaves exactly
//! **1**, so a routing bug that sent the hand card nowhere satisfied `<= 3` as well as a
//! correct one. The deck was also staged with only two distinct cards near the top, so
//! nothing could tell WHICH of the three looked-at cards went where — which is the entire
//! content of the printed text.
//!
//! ## The interaction the engine actually presents
//!
//! Traced, not assumed: the engine asks **twice**, both times a `SelectCard` over
//! `looked_at`, with 3 and then 2 options, and never offers a destination prompt.
//!
//! ```text
//! SelectCard(zone=looked_at count=1 allow_skip=false options=Some(3))
//! SelectCard(zone=looked_at count=1 allow_skip=false options=Some(2))
//! ```
//!
//! The first pick goes to 手札, the second to デッキの上, and the one card left over goes to
//! 控え室. That is a defensible reading of the text — it names the three outcomes without
//! saying the player picks the destination — and it gives the player the choice of *which*
//! card reaches hand and which reaches the deck. So the previous file's
//! `Some("SelectTarget") => game.select_option(1)` branch was dead code that never ran.
//!
//! What the player therefore controls is the routing *of card identity*, and that is what
//! these tests pin: each card is selected deliberately and then located by identity.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const NOZOMI: &str = "PL!-bp3-007-R";
/// Three distinct looked-at cards so routing is identifiable afterwards.
const CARD_A: &str = "PL!-pb1-021-PR"; // 南 ことり
const CARD_B: &str = "PL!S-PR-016-PR"; // 黒澤ダイヤ
const CARD_C: &str = "PL!N-sd1-025-SD";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains(trigger_str))
        })
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

/// Card ids currently offered by the pending SelectCard, in menu order.
fn offered_ids(game: &TestGame) -> Vec<i16> {
    let json = game
        .state
        .get_pending_choice_json()
        .expect("a pending choice with a selection_cards array");
    json["selection_cards"]
        .as_array()
        .expect("selection_cards must be an array")
        .iter()
        .filter_map(|c| c.get("id").and_then(|v| v.as_i64()).map(|v| v as i16))
        .collect()
}

/// Choose the offered card whose id is `want`, by identity rather than by position.
fn choose_offered(game: &mut TestGame, want: i16) {
    let offered = offered_ids(game);
    let idx = offered
        .iter()
        .position(|&id| id == want)
        .unwrap_or_else(|| {
            panic!("the card chosen for this destination must be among those offered; wanted {want}, offered {offered:?}")
        });
    game.select_indices(&[idx]);
}

/// Deck top-to-bottom is [CARD_A, CARD_B, CARD_C] — `put_on_deck_top` inserts at index 0,
/// so the cards are pushed in reverse.
struct Board {
    game: TestGame,
    a: i16,
    b: i16,
    c: i16,
    paid_a: i16,
    paid_b: i16,
    deck_suffix_before: usize,
}

fn board() -> Board {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id(NOZOMI);
    game.assert_card_identity(nozomi, NOZOMI);

    let a = game.id(CARD_A);
    let b = game.id(CARD_B);
    let c = game.id(CARD_C);
    let paid_a = game.id(FILLER);
    let paid_b = game.id(FILLER);
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    put_on_deck_top(&mut game, 0, c);
    put_on_deck_top(&mut game, 0, b);
    put_on_deck_top(&mut game, 0, a);
    game.add_to_hand(paid_a);
    game.add_to_hand(paid_b);
    let deck_suffix_before = game.state.player1.main_deck.cards.len();
    game.state.player1.stage.stage[1] = nozomi;

    Board { game, a, b, c, paid_a, paid_b, deck_suffix_before }
}

impl Board {
    fn trigger(&mut self) {
        let nozomi = self.game.id(NOZOMI);
        trigger_auto(&mut self.game, nozomi, AbilityTrigger::LiveStart, "ライブ開始時");
    }

    /// Pay the optional cost and return the number of cards offered by the next prompt.
    fn pay_cost(&mut self) -> usize {
        assert_eq!(
            self.game.pending_choice_type().as_deref(),
            Some("SelectCard"),
            "the optional cost is offered first"
        );
        assert_eq!(
            self.game.pending_choice_count(),
            2,
            "手札を2枚控え室に — exactly two cards are paid"
        );
        self.game.select_indices(&[0, 1]);
        assert_eq!(
            self.game.pending_choice_type().as_deref(),
            Some("SelectCard"),
            "the first of the two looked-at picks follows the cost"
        );
        match self.game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { allow_skip, .. } => assert!(
                !allow_skip,
                "neither looked-at pick is optional — 見る carries no 「てもよい」, so the \
                 routing cannot be declined"
            ),
            other => panic!("expected a SelectCard pick, got {other:?}"),
        }
        offered_ids(&self.game).len()
    }
}

/// 1枚を手札に加え — the FIRST pick is the card that reaches the hand.
#[test]
fn pl_bp3_007_r_the_first_pick_reaches_the_hand() {
    let mut b = board();
    b.trigger();
    assert_eq!(b.pay_cost(), 3, "the first pick offers all three looked-at cards");

    choose_offered(&mut b.game, b.c);
    assert!(
        b.game.state.player1.hand.cards.contains(&b.c),
        "1枚を手札に加え — the FIRST chosen card must be the one in hand (hand={:?})",
        b.game.state.player1.hand.cards
    );

    // Second pick: whatever remains that is not C, then C is verified as the leftover.
    let offered = offered_ids(&b.game);
    assert_eq!(offered.len(), 2, "after the first pick two cards remain (got {offered:?})");
    assert!(!offered.contains(&b.c), "the card already taken to hand is no longer offered");
}

/// 1枚をデッキの上に置き — the SECOND pick becomes the new top of the deck, and the card
/// neither pick claimed is the one banked.
#[test]
fn pl_bp3_007_r_the_second_pick_becomes_the_new_deck_top_and_the_leftover_is_banked() {
    let mut b = board();
    b.trigger();
    assert_eq!(b.pay_cost(), 3, "the first pick offers all three looked-at cards");

    // A → hand.
    choose_offered(&mut b.game, b.a);
    // B → deck top. Pick it by identity out of the two that remain.
    let offered = offered_ids(&b.game);
    assert_eq!(offered.len(), 2, "after A, B and C remain (got {offered:?})");
    choose_offered(&mut b.game, b.b);

    assert!(
        !b.game.has_pending_choice(),
        "three cards route in exactly two picks; the third needs no prompt"
    );

    // Identity of each destination.
    let hand = &b.game.state.player1.hand.cards;
    assert!(
        hand.contains(&b.a) && !hand.contains(&b.b) && !hand.contains(&b.c),
        "1枚を手札に加え — exactly the first pick (A) is in hand, and neither B nor C is \
         (hand={hand:?})"
    );
    assert_eq!(hand.len(), 1, "and nothing else joined the hand (hand={hand:?})");
    assert_eq!(
        b.game.state.player1.main_deck.cards.first().copied(),
        Some(b.b),
        "1枚をデッキの上に置き — exactly the second pick is the new top of the deck \
         (top={:?})",
        b.game.state.player1.main_deck.cards.first()
    );
    assert!(
        b.game.state.player1.waitroom.cards.contains(&b.c),
        "1枚を控え室に置く — C, chosen by neither pick, is the card banked \
         (waitroom={:?})",
        b.game.state.player1.waitroom.cards
    );

    // No card may sit in two places, and the paid cards are banked too.
    for (label, id) in [("A", b.a), ("B", b.b), ("C", b.c)] {
        let places = [
            b.game.state.player1.hand.cards.contains(&id),
            b.game.state.player1.main_deck.cards.contains(&id),
            b.game.state.player1.waitroom.cards.contains(&id),
        ];
        assert_eq!(
            places.iter().filter(|&&p| p).count(),
            1,
            "card {label} must end in exactly ONE of hand/deck/waitroom, got {places:?}"
        );
    }
    assert!(
        b.game.state.player1.waitroom.cards.contains(&b.paid_a)
            && b.game.state.player1.waitroom.cards.contains(&b.paid_b),
        "手札を2枚控え室に — both paid cards are banked (waitroom={:?})",
        b.game.state.player1.waitroom.cards
    );
    assert_eq!(
        b.game.state.player1.waitroom.cards.len(),
        3,
        "2 paid + 1 leftover = 3"
    );
    assert_eq!(
        b.game.state.player1.main_deck.cards.len(),
        b.deck_suffix_before - 2,
        "3 left the deck for the look and 1 came back, so net -2"
    );
}

/// Swapping which card is picked first must swap which one reaches the hand and the deck.
/// A routing bug that always sent the top card to hand would pass a test that only ever
/// picks index 0 — this one picks the LAST card first.
#[test]
fn pl_bp3_007_r_picking_a_different_first_card_reroutes_hand_and_deck() {
    let mut b = board();
    b.trigger();
    assert_eq!(b.pay_cost(), 3, "the first pick offers all three looked-at cards");

    // C first this time, then A.
    choose_offered(&mut b.game, b.c);
    let offered = offered_ids(&b.game);
    assert_eq!(offered.len(), 2, "after C, A and B remain (got {offered:?})");
    choose_offered(&mut b.game, b.a);

    let hand = &b.game.state.player1.hand.cards;
    assert!(
        hand.contains(&b.c) && !hand.contains(&b.a) && !hand.contains(&b.b),
        "the FIRST pick is what reaches the hand — C this time, not A (hand={hand:?})"
    );
    assert_eq!(hand.len(), 1, "and only that one card (hand={hand:?})");
    assert_eq!(
        b.game.state.player1.main_deck.cards.first().copied(),
        Some(b.a),
        "the SECOND pick is what reaches the deck top — A this time, not B"
    );
    assert!(
        b.game.state.player1.waitroom.cards.contains(&b.b),
        "B, chosen by neither pick, is banked (waitroom={:?})",
        b.game.state.player1.waitroom.cards
    );
}
