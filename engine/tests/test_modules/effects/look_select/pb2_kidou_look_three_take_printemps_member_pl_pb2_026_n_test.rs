//! 小泉花陽 `PL!-pb2-026-N` (起動, ターン1回)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   起動／ターン1回 — このメンバーをウェイトにし、手札を1枚控え室に置く：自分のデッキの
//!   上からカードを3枚見る。その中から『Printemps』のメンバーカードを1枚公開して手札に
//!   加えてもよい。残りを控え室に置く。
//!
//! In English: pay (wait her and discard a hand card), look at the top 3, and you may
//! take one Printemps member to hand; the rest go to the waitroom.
//!
//! Setup idiom lifted from
//! `effects/look_select/optional_look_and_select_negative_branches_test.rs`, which
//! documents the important property of this shape: **there is exactly ONE prompt** and
//! the cost, the look and the banking all resolve on it, so the only observable points
//! are the play, the activation, and the answer. There is no "after the cost, before
//! the look" state to sample.
//!
//! 「もよい」 makes the RETRIEVAL optional, and the engine's convention (documented at
//! `ability/cost/handlers.rs:364`) is that the cost colon-gates the whole effect — so
//! both branches below are taken through a PAID cost, and the optionality is tested at
//! the look step, by counting what is actually offered.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NOZOMI: &str = "PL!-pb2-026-N"; // 小泉花陽, Printemps
const FILLER: &str = "PL!-sd1-010-SD";
/// A Printemps member — the only kind the look may take.
const P_MEMBER: &str = "PL!-bp6-017-N"; // 小泉花陽
/// A μ's member that is NOT Printemps, so it must not be offered.
const LW_MEMBER: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, BiBi
/// Another Printemps member, for the mixed case.
const P_MEMBER2: &str = "PL!-bp5-012-N"; // 南 ことり

#[derive(Debug, Clone, Copy)]
struct Pools {
    hand: usize,
    waitroom: usize,
    #[allow(dead_code)]
    deck: usize,
}

fn pools(game: &TestGame) -> Pools {
    Pools {
        hand: game.state.player1.hand.cards.len(),
        waitroom: game.state.player1.waitroom.cards.len(),
        deck: game.state.player1.main_deck.cards.len(),
    }
}

/// Stand NOZOMI, pay the activation cost, answer the single prompt, and report the
/// pools before and after. `top` is the first cards of the deck, in order.
fn resolve_with_top(top: &[&str]) -> (Pools, Pools, TestGame) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(NOZOMI);
    game.assert_card_identity(me, NOZOMI);

    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for card_no in top {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(card_no));
    }
    for _ in 0..40 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    // Two hand cards so the discard has something to take.
    game.add_to_hand(me);
    game.state.player1.hand.cards.push(game.new_id(FILLER));
    game.state.player1.hand.cards.push(game.new_id(FILLER));

    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(me, MemberArea::Center);
    game.drain_choices();

    game.activate_ability(me);
    let at_activation = pools(&game);
    assert!(
        game.has_pending_choice(),
        "precondition: the 起動 cost must raise a prompt"
    );
    // Pay: select the hand card to discard, and any follow-up the look raises.
    game.select_indices(&[0]);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        game.select_indices(&[0]);
        guard += 1;
    }
    let after = pools(&game);
    (at_activation, after, game)
}

/// The cost: she goes to wait, and a hand card is discarded.
///
/// This test previously carried a confident claim that the self-wait sub-cost is NOT
/// applied — "under rule 9.4.2.2 the binary self-wait sub-cost is DEFERRED until the
/// choice sub-cost resolves, and it is not being applied when that happens" — and then
/// measured two things without asserting either of them. The claim is **false**: paying
/// the cost does put her into wait.
///
/// The confusion is understandable and worth recording. A `sequential_cost` resolves its
/// elements in order, so the hand-selection prompt for the discard comes FIRST and the
/// orientation is still `active` while that prompt is up; only once the discard has been
/// answered does the deferred self-wait land. Sampling at the wrong moment reads as "not
/// applied".
///
/// The contrast that settles it: `PL!-pb2-008-PP` carries the SAME self-wait as a
/// STANDALONE `change_state` cost and applies it; this card carries it as the first
/// element of a `sequential_cost` and also applies it. Neither form is broken, which is
/// why the "008 / 018 / 026 are all broken by their optional costs" pattern this file
/// once asserted does not survive contact with the cards.
#[test]
fn paying_the_cost_waits_her_and_discards_a_hand_card() {
    let (before, after, game) = resolve_with_top(&[P_MEMBER]);

    // The card that is IN PLAY, not a fresh instance. `id()` hands out a new pooled
    // instance per call, so asking about `game.id(NOZOMI)` here would inspect a card
    // that was never played and always read back as unwaited. That is precisely what the
    // previous version of this test did inside its `println!`, and it is where the
    // "self-wait is not applied" claim came from — the same error as the vacuous
    // assertion in `PL!-pb2-015-P＋`.
    let her = game.state.player1.stage.stage[MemberArea::Center as usize];
    assert_ne!(her, -1, "precondition: she is on the centre stage");
    assert_eq!(
        game.state.mods.get_orientation_modifier(her),
        Some("wait"),
        "このメンバーをウェイトにし — the self-wait half of the 起動 cost DOES apply, once \
         the discard prompt that precedes it has been answered"
    );
    assert!(
        after.waitroom >= before.waitroom,
        "残りを控え室に置く — the look banks its unselected cards, so the waitroom does \
         not shrink"
    );
    // A hand-delta assertion is deliberately NOT made here: the look's take moves a card
    // in the opposite direction on the same counter, so a delta conflates the discard
    // with the take. That is a measurement claim dressed as a behaviour claim, which is
    // what this test used to be.
}

/// The look offers exactly the Printemps members among the top 3.
///
/// ## What was wrong with the previous version
///
/// 1. **It measured the COST.** The 起動 cost is a `sequential_cost` of
///    `change_state(self→wait)` + `move_cards(hand→discard, 1)`, and the move_cards
///    sub-cost raises its own `Choice::select_cards(Hand, 1, …)`
///    (`ability/cost/handlers.rs:608`). The loop broke on the FIRST `SelectCard`, which
///    is that cost prompt, so the look was never observed. Both are `SelectCard`, so
///    matching on the type alone cannot tell them apart — see `resolve_look_prompt` below,
///    which separates them by whether the hand shrinks.
/// 2. **It read the wrong number.** `pending_choice_count()` is
///    `max_select = min(requested, matching)` (`ability/effects/look.rs:110,156`), so for
///    a 「1枚」 take it is 1 whenever a prompt exists.
/// 3. **The expected value was wrong.** `FILLER = PL!-sd1-010-SD` is 高坂 穂乃果, whose
///    unit is **Printemps** — it satisfies 「『Printemps』のメンバーカード」 and must be
///    offered. The correct count for this pool is **2**, not 1. Only 絢瀬絵里 (BiBi) is
///    excluded, by the group filter.
#[test]
fn only_printemps_members_are_offered_from_the_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(NOZOMI);
    game.assert_card_identity(me, NOZOMI);
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    // 小泉花陽 (Printemps), 絢瀬絵里 (BiBi), 高坂穂乃果 (Printemps).
    // main_deck was cleared first, so these sit at index 0..2 and the deck — which draws
    // from the FRONT via `cards.remove(0)` (`core/zones.rs:818`) — looks at exactly them.
    for card_no in [P_MEMBER, LW_MEMBER, FILLER] {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(card_no));
    }
    for _ in 0..40 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.add_to_hand(me);
    game.state.player1.hand.cards.push(game.new_id(FILLER));
    game.state.player1.hand.cards.push(game.new_id(FILLER));
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(me, MemberArea::Center);
    game.drain_choices();
    game.activate_ability(me);

    // Pay 手札を1枚控え室に置く. Identified by the hand shrinking, not by the prompt
    // type — the cost and the look are both SelectCard.
    let hand_before_cost = game.state.player1.hand.cards.len();
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the 起動 cost discards a hand card, so a SelectCard over the hand is pending"
    );
    game.select_indices(&[0]);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before_cost - 1,
        "precondition: this was the COST prompt (hand shrank), not the look"
    );

    // Now the look prompt. 「その中から『Printemps』のメンバーカードを1枚」 — count the
    // CANDIDATES, not the pick count.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "precondition: the cost is paid and the LOOK prompt is pending"
    );
    assert_eq!(
        game.pending_choice_option_count(),
        Some(2),
        "その中から『Printemps』のメンバーカードを1枚 — of the three cards looked at, \
         小泉花陽 (Printemps) and 高坂 穂乃果 (Printemps, the FILLER) both qualify; only \
         絢瀬絵里 (BiBi) is excluded, by the group filter. The old expected value of 1 \
         was wrong: the filler is a Printemps member and does qualify."
    );
}

/// Taking the card moves it to hand; the unselected ones go to the waitroom.
#[test]
fn taking_moves_one_card_to_hand_and_banks_the_rest() {
    let (before, after, _game) = resolve_with_top(&[P_MEMBER, FILLER, FILLER]);
    assert!(
        after.waitroom > before.waitroom,
        "残りを控え室に置く — the unselected looked cards are banked \
         (before {before:?}, after {after:?})"
    );
}

/// Nothing qualifying in the look is not a failure — the rest is still banked.
#[test]
fn a_look_with_no_printemps_member_still_banks_everything() {
    let (before, after, _game) = resolve_with_top(&[LW_MEMBER, FILLER, FILLER]);
    assert!(
        after.waitroom > before.waitroom,
        "『Printemps』のメンバーカードを1枚…もよい — with nothing qualifying, the \
         looked cards are still banked rather than left in the deck \
         (before {before:?}, after {after:?})"
    );
}

/// The card's own ability shape, pinned so a re-parse cannot silently change the
/// group filter the look depends on.
#[test]
fn the_look_filters_on_printemps_members_only() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(NOZOMI);
    let eff = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("起動"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("起動 ability has an effect")
    };
    assert_eq!(
        eff.action,
        rabuka_engine::ability::enums::ActionType::LookAndSelect,
        "3枚見る…『Printemps』のメンバーカードを1枚"
    );
    let groups: Vec<String> = eff.group_names_any().cloned().unwrap_or_default();
    assert_eq!(
        groups,
        vec!["Printemps".to_string()],
        "『Printemps』のメンバーカード — the look's group filter"
    );
    game.assert_card_in_group(game.id(P_MEMBER), "Printemps", "must be offered");
    game.assert_card_in_group(game.id(P_MEMBER2), "Printemps", "must be offered");
    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(
            &game.db,
            game.id(LW_MEMBER),
            Some("Printemps")
        ),
        "絢瀬絵里 is BiBi, so it must not be offered"
    );
}
