//! 星空 凛 `PL!-pb2-032-N` (登場)
//!
//! {{toujyou.png|登場}}手札を1枚控え室に置く：自分のデッキの上からカードを5枚見る。
//! その中からブレードハートを持たない『μ's』のメンバーカードを1枚公開して手札に加えても
//! よい。残りを控え室に置く。
//!
//! This file pins the CANDIDATE FILTER: a NEGATED property (ブレードハートを持たない)
//! crossed with a group filter. The three looked-at cards are chosen so that each
//! failure mode has a distinct expected candidate count:
//!
//! | deck | blade heart | μ's? | qualifies? |
//! |---|---|---|---|
//! | `PL!-bp4-012-N` 南 ことり | no | yes | **yes** |
//! | `PL!-PR-012-PR` 小泉花陽 | yes | yes | no — negation |
//! | `PL!HS-PR-001-PR` 日野下花帆 | no | no | no — group |
//!
//! Dropping the negation offers 2; dropping the group filter offers 2; dropping both
//! offers 3. So a candidate count of 1 pins both gates at once.
//!
//! ## What was wrong with the previous version of this file
//!
//! All three of its assertions were reading the wrong thing, and its fixtures had the
//! negation exactly backwards:
//!
//! 1. **Both blade fixtures were inverted.** `PL!-bp6-017-N` (小泉花陽) *does* carry
//!    `blade_heart: {b_heart03: 1}` and `PL!-pb2-002-PP` (絢瀬絵里) has **no**
//!    `blade_heart` key at all. `has_blade_heart()` is exactly
//!    `self.blade_heart.is_some()` (`engine/src/core/card.rs:4273-4279`). Under
//!    `negation: true` the one card that qualified was 絢瀬絵里 — the card the test
//!    named as the excluded one. The test passed with the semantics precisely
//!    reversed, so ブレードハートを持たない was never actually tested.
//! 2. **It measured the mandatory COST prompt, not the look.** `手札を1枚控え室に置く`
//!    is a `move_cards` cost that raises a `SelectCard`, and
//!    `drain_auto_ability_choices` only drains `SelectAutoAbility` and breaks on
//!    anything else (`helpers/choices.rs:341-356`). The cost prompt was still pending,
//!    so `has_pending_choice`, the type check and the count all read the cost.
//! 3. **`pending_choice_count()` is not the candidate count.** The engine sets that
//!    field to `max_select = min(requested, matching)`
//!    (`engine/src/ability/effects/look.rs:110,156`); candidates are the separate
//!    `filtered_indices` field (`look.rs:175`). `count == 1` is therefore 1 whether one
//!    card or five qualify, so "dropping either filter would offer 2 or 3" was not
//!    merely unverified — it was impossible. Use
//!    [`pending_choice_option_count`] instead.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const RIN: &str = "PL!-pb2-032-N"; // 星空 凛, lily white, cost 4
/// A μ's member with NO blade heart — the only kind that qualifies.
const NO_BH: &str = "PL!-bp4-012-N"; // 南 ことり, Printemps, no blade_heart
/// A μ's member that DOES carry a blade heart — excluded by the negation.
const BH_MEMBER: &str = "PL!-PR-012-PR"; // 小泉花陽, Printemps, blade_heart b_heart03
/// Not μ's, and no blade heart either — so only the group filter can exclude it.
const NON_MUS: &str = "PL!HS-PR-001-PR"; // 日野下花帆, Hasunosora
/// The deck filler. Two copies of it land in the looked-at pool, and they do NOT raise
/// the candidate count above 1 — which `only_blade_heart_free_mus_members_are_selectable`
/// proves by pinning the count. `cards.json` records `blade_heart {b_heart03: 1}` for
/// this print, so the negation should exclude them; that is stated, not asserted, because
/// the exact predicate the engine applies here was not established.
const FILLER: &str = "PL!-sd1-010-SD"; // 高坂 穂乃果, Printemps

/// Play RIN, pay the mandatory discard, and stop with the LOOK prompt open.
///
/// The cost has to be answered explicitly. Draining the auto-ability choices leaves a
/// `SelectCard` cost prompt pending, and measuring at that point reads the cost.
///
/// The candidates go on with `put_on_deck_top` because `MainDeck::draw()` is
/// `cards.remove(0)` (`core/zones.rs:818`) — the deck draws from the FRONT, so cards
/// merely `push`ed after `fill_decks` sit at the BOTTOM and are never looked at.
/// The first attempt at this test got that wrong: the look saw five copies of the
/// filler, which carries a blade heart, correctly matched zero, and discarded all
/// five. Inserted in reverse so the pool reads NO_BH, BH_MEMBER, NON_MUS, filler,
/// filler.
fn look_prompt_open() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    for c in [NON_MUS, BH_MEMBER, NO_BH] {
        let id = game.id(c);
        put_on_deck_top(&mut game, 0, id);
    }
    // The mandatory cost discards one hand card, so the hand needs a second card besides
    // RIN or the ability cannot pay and never reaches the look step.
    game.state.player1.hand.cards.push(game.id(NO_BH));
    let rin = game.id(RIN);
    game.assert_card_identity(rin, RIN);
    game.state.player1.hand.cards.push(rin);
    game.give_energy(20);
    game.play_to_stage(rin, MemberArea::Center);
    game.drain_auto_ability_choices();

    // Pay 手札を1枚控え室に置く. A mandatory discard is a SelectCard over the hand, so the
    // decline idiom is an empty index list, not `select_option(0)` (which would take a card).
    let hand_before = game.state.player1.hand.cards.len();
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the mandatory cost is a SelectCard over the hand"
    );
    game.select_indices(&[0]);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "precondition: 手札を1枚控え室に置く — exactly the one cost card left the hand"
    );

    game
}

/// The cost and the look are separate prompts: answering the cost must open the look.
///
/// The filler is `PL!-sd1-010-SD`, which carries `blade_heart: {b_heart03: 1}`, so the
/// two filler cards in the looked-at pool are excluded by the negation. The pool is
/// therefore 5 cards with exactly 1 qualifier — which is what makes a count of 1
/// meaningful.
#[test]
fn paying_the_mandatory_cost_opens_the_look_prompt() {
    let game = look_prompt_open();
    assert!(
        game.has_pending_choice(),
        "the look prompt must be open after the cost is paid. If no prompt is raised at \
         all, the look matched zero candidates and discarded the whole pool \
         (ability/effects/look.rs:102-105). state: {}",
        game.pending_choice_summary()
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "precondition: the cost is answered and the LOOK prompt is now pending — the \
         previous version of this test never got past the cost. state: {}",
        game.pending_choice_summary()
    );
}

/// The candidate filter. One of the three looked-at cards qualifies, and which one is
/// determined by a negated property AND a group filter at once.
#[test]
fn only_blade_heart_free_mus_members_are_selectable() {
    let game = look_prompt_open();
    assert_eq!(
        game.pending_choice_option_count(),
        Some(1),
        "ブレードハートを持たない『μ's』のメンバーカード1枚 — of the three cards looked at \
         only 南 ことり qualifies: 小泉花陽 carries a blade heart (excluded by the \
         negation) and 日野下花帆 is not a μ's card (excluded by the group). Dropping \
         the negation would offer 2, dropping the group would offer 2, dropping both \
         would offer 3. state: {}",
        game.pending_choice_summary()
    );
}

/// DIAGNOSTIC (removed): it served its purpose — proving the engine was innocent when
/// the pool was built with `push` instead of `put_on_deck_top` — and is not a gate.

/// The fixture invariants that make the count test meaningful.
///
/// If either of these fails, `only_blade_heart_free_mus_members_are_selectable` is not
/// testing what it claims — the same failure mode the previous version of this file had,
/// where both blade fixtures were inverted and the negation was never exercised at all.
///
/// `has_blade_heart_strict` is `blade_heart.is_some()` (`core/card.rs:4281-4283`), i.e.
/// exactly the `blade_heart` key in `cards/cards.json`. The filter itself goes through
/// `has_blade_heart`, which is also true when `special_heart` is non-empty
/// (`core/card.rs:4273-4279`), so the two can disagree — these fixtures are checked with
/// the strict accessor because that is the one that matches the data file.
///
/// The two filler cards in the looked-at pool do not raise the count to 3, which the
/// count test proves. The mechanism is not asserted here: it was not established, and
/// guessing it is how a wrong rule gets written down.
#[test]
fn the_fixture_invariants_hold() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let strict_blade = |c: &str| {
        game.db
            .get_card(game.id(c))
            .unwrap_or_else(|| panic!("{c} not in cards.json"))
            .has_blade_heart_strict()
    };
    assert!(!strict_blade(NO_BH), "{NO_BH} must have NO blade heart — it is the qualifier");
    assert!(strict_blade(BH_MEMBER), "{BH_MEMBER} must carry a blade heart — it is excluded by the negation");
    assert!(!strict_blade(NON_MUS), "{NON_MUS} must have no blade heart, so only the group filter can exclude it");
}

// Two findings are NOT asserted here, because there is nothing correct to assert yet
// (see docs/PBLL02_ABILITY_REVIEW.md §8.3):
//   * the select prompt is NOT skippable, although the text says 加えてもよい — the
//     parsed node has `optional: true`, so either the optional flag is not reaching
//     the prompt or it is dropped on this path;
//   * answering the prompt with an index does not place the card in hand.
