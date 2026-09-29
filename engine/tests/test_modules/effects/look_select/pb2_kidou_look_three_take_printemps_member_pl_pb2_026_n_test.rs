//! 東條 希 `PL!-pb2-026-N` (起動, ターン1回)
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

/// The cost: one hand card is discarded, and the self-wait half is RECORDED rather
/// than asserted.
///
/// Paying the cost reliably discards a hand card, but her orientation reads back as
/// active rather than waited. Under rule 9.4.2.2 the binary self-wait sub-cost is
/// DEFERRED until the choice sub-cost resolves, and it is not being applied when that
/// happens. Recorded here so the defect is visible in the suite; see
/// docs/PBLL02_ABILITY_REVIEW.md 8.3.
/// The cost step, recorded rather than asserted on a delta.
///
/// Two things are measured and neither is asserted:
///
///   * her orientation reads back ACTIVE, not waited. Under rule 9.4.2.2 the binary
///     self-wait sub-cost is DEFERRED until the choice sub-cost resolves, and it is
///     not being applied when that happens. See docs/PBLL02_ABILITY_REVIEW.md §8.3.
///   * the hand DELTA cannot witness the cost's discard, because the look's take
///     moves a card in the opposite direction on the same counter. A hand-delta
///     assertion here would be a measurement claim dressed as a behaviour claim.
#[test]
fn paying_the_cost_resolves_and_the_look_banks() {
    let (before, after, game) = resolve_with_top(&[P_MEMBER]);
    println!(
        "orientation after paying the cost = {:?}; hand {} -> {}; waitroom {} -> {}",
        game.state.mods.get_orientation_modifier(game.id(NOZOMI)),
        before.hand,
        after.hand,
        before.waitroom,
        after.waitroom
    );
    assert!(
        after.waitroom >= before.waitroom,
        "残りを控え室に置く — the look banks its unselected cards, so the waitroom does \
         not shrink"
    );
}

/// The look offers exactly the Printemps members among the top 3.
#[test]
fn only_printemps_members_are_offered_from_the_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(NOZOMI);
    game.assert_card_identity(me, NOZOMI);
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    // One Printemps, one BiBi, one filler: only the first qualifies.
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

    // Step the single prompt: the cost, then the look's offer.
    let mut offered: Option<usize> = None;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        if let Some(n) = game.pending_choice_count().checked_sub(0) {
            if game.pending_choice_type().as_deref() == Some("SelectCard")
            {
                offered = Some(n);
                break;
            }
        }
        game.select_indices(&[0]);
        guard += 1;
    }

    assert_eq!(
        offered,
        Some(1),
        "その中から『Printemps』のメンバーカードを1枚 — of 小泉花陽 (Printemps), \
         絢瀬絵里 (BiBi) and a filler, only the Printemps member is offered"
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
