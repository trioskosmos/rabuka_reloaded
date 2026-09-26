//! 若菜四季 `PL!SP-bp7-008-R` — the tightest self-watching pair in the deck.
//!
//! Two abilities on one card, where the SECOND is only reachable because the
//! FIRST ran:
//!
//!   * ab#0 起動 「このメンバーをウェイトにする：カードを1枚引く。」
//!   * ab#1 自動 「ウェイト状態のこのメンバーがエリアを移動したとき、このメンバーを
//!     アクティブにする。」
//!
//! ab#1's trigger is self-referential in the tightest way: it watches *its own
//! card's* area change, and its gate is *its own card's* orientation. The 起動 is
//! the only thing that sets that orientation, so ab#1 is unreachable without it
//! having activated first — and, just as importantly, must stay silent once the
//! 自動 has already consumed the waited state.
//!
//! `area_swap_grants_blade_heart_or_recovers_group_live_test.rs` pins the happy
//! path (起動 → waited, area move → reactivated). What it does not pin is the
//! other three transitions, and the report's own ranking agrees: this card is the
//! THINNEST entry in TEST_COVERAGE's "Cards pairing a jidou with another ability"
//! table at 1/5 direct tests and `L1+choice` — the only row in that table that
//! never reaches L2, because the negatives were never written.
//!
//! The three missing transitions, and why each can fail independently:
//!
//!   1. THE GATE. A move while she is ACTIVE must not fire ab#1 at all. Without
//!      this, an engine that ignored 「ウェイト状態のこのメンバー」 and fired on
//!      every self-move would pass the existing happy-path test.
//!   2. THE CYCLE. The 自動 is one-shot per waited state: it clears the gate, so
//!      a second move must not re-fire, and re-arming needs the 起動 again.
//!   3. THE DRAW. カードを1枚引く belongs to the 起動, paid at activation. If the
//!      自動 re-activation ever re-ran the pair, the draw would be duplicated —
//!      and a test that only checks the orientation would not see it.
//!
//! Movement is driven through the real ミリオン Haas |positional-change
//! activation path (`activate_mill_three_position_swap`), so the 自動 is reached
//! by the same route a player's move takes, not by a hand-pushed event.

use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::zones::MemberArea;

const SHIKI: &str = "PL!SP-bp7-008-R";

/// Stage 若菜四季 centred, with a deck deep enough that a draw can never be the
/// reason a count came out where it did.
///
/// The card id is bound and identity-pinned in each TEST BODY rather than here,
/// on purpose: TEST_COVERAGE's `Tests` column is `direct/in file`, and a card
/// bound only inside a helper reads as co-location rather than as a test that
/// drives it. The helper takes the id so the body stays the thing that names the
/// card.
fn stage_shiki(game: &mut TestGame, shiki: i16) {
    append_twenty_filler_cards(game);
    game.add_to_stage(MemberArea::Center, shiki);
    game.give_energy(10);
}

fn orientation(game: &TestGame, shiki: i16) -> Option<CardOrientation> {
    game.state
        .mods
        .orientation_modifiers
        .get(&shiki)
        .copied()
}

fn hand_len(game: &TestGame) -> usize {
    game.state.player1.hand.cards.len()
}

/// The area 若菜四季 currently occupies.
///
/// The three tests below move her more than once, and
/// `activate_mill_three_position_swap` swaps whatever occupies the area you pass
/// — it does not know who you mean. Hard-coding an area sequence silently stops
/// exercising ab#1 the moment she lands somewhere else, because a swap that does
/// not touch her moves nothing and the test passes vacuously. Reading her area
/// back each time is what keeps every step a real move.
fn shiki_area(game: &TestGame, shiki: i16) -> MemberArea {
    let stage = &game.state.player1.stage.stage;
    for (slot, occupant) in stage.iter().enumerate() {
        if *occupant == shiki {
            return match slot {
                0 => MemberArea::LeftSide,
                1 => MemberArea::Center,
                _ => MemberArea::RightSide,
            };
        }
    }
    panic!(
        "若菜四季 ({shiki}) is not on p1's stage at all: {stage:?} — the previous \
         step took her off the board instead of moving her"
    );
}

/// Swap 若菜四季 out of whichever area she is in, and assert she really moved.
///
/// Returns the area she started in so a caller can prove the move happened.
fn move_shiki(game: &mut TestGame, shiki: i16) -> MemberArea {
    let from = shiki_area(game, shiki);
    activate_mill_three_position_swap(game, from);
    game.drain_auto_ability_choices();
    let to = shiki_area(game, shiki);
    assert_ne!(
        to, from,
        "the swap must actually have moved 若菜四季 out of {from:?} and into \
         {to:?}; a no-op swap would make every ab#1 assertion below vacuous"
    );
    from
}

/// PREMISE for the whole file: the 起動 both waits her AND draws. Pinned here so
/// the per-transition tests below can rely on "waited and drew" as the armed
/// state, and so a reader can see the pairing is real rather than assumed.
#[test]
fn shiki_kidou_waits_her_and_draws_exactly_one() {
    let mut game = TestGame::new(load_real_database());
    let shiki = game.id(SHIKI);
    game.assert_card_identity(shiki, SHIKI);
    stage_shiki(&mut game, shiki);

    let hand_before = hand_len(&game);
    let deck_before = game.state.player1.main_deck.cards.len();

    game.activate_ability(shiki);
    game.drain_auto_ability_choices();

    assert_eq!(
        orientation(&game, shiki),
        Some(CardOrientation::Wait),
        "ab#0 起動's cost must leave 若菜四季 ウェイト状態"
    );
    assert_eq!(
        hand_len(&game),
        hand_before + 1,
        "ab#0 起動 must draw exactly one card — this is the draw the cycle tests \
         below check does NOT repeat"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "the card came from the deck, so the hand delta names the draw"
    );
}

/// GAP 1 — THE GATE.
///
/// A self-move while she is ACTIVE must leave ab#1 completely uninvolved. This
/// is the negative the existing happy path lacks, and it is the one that
/// separates a correct gate from "fires on every area change".
#[test]
fn shiki_auto_does_not_fire_on_a_move_while_she_is_active() {
    let mut game = TestGame::new(load_real_database());
    let shiki = game.id(SHIKI);
    game.assert_card_identity(shiki, SHIKI);
    stage_shiki(&mut game, shiki);

    // Control: she starts ACTIVE, so the 起動 is never used and the gate is
    // never opened.
    assert_eq!(
        orientation(&game, shiki),
        None,
        "precondition: a freshly staged member carries no orientation modifier, \
         i.e. she is アクティブ"
    );

    let hand_before = hand_len(&game);

    // A real engine move: ミリオン Haas's positional-change activation swaps
    // 若菜四季 out of the slot she is actually in.
    move_shiki(&mut game, shiki);

    assert_eq!(
        orientation(&game, shiki),
        None,
        "ab#1 自動 must NOT fire for a member that was already アクティブ: the \
         gate is ウェイト状態のこのメンバー. An engine that ignored the gate would \
         leave an explicit Active marker here."
    );
    assert_eq!(
        hand_len(&game),
        hand_before,
        "a silent 自動 draws nothing — ab#0 起動 is the only thing that draws here"
    );
}

/// GAP 2 — THE CYCLE.
///
/// ab#1 CONSUMES the waited state, so it is one-shot: a second move while she is
/// now active must not re-fire, and only the 起動 can re-arm it.
#[test]
fn shiki_auto_fires_once_per_waited_state_and_the_kidou_re_arms_it() {
    let mut game = TestGame::new(load_real_database());
    let shiki = game.id(SHIKI);
    game.assert_card_identity(shiki, SHIKI);
    stage_shiki(&mut game, shiki);

    // Arm: 起動 waits her and draws.
    let hand_before_cycle = hand_len(&game);
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert_eq!(orientation(&game, shiki), Some(CardOrientation::Wait));
    assert_eq!(
        hand_len(&game),
        hand_before_cycle + 1,
        "precondition: the arming draw landed exactly once"
    );

    // First move: the 自動 consumes the waited state and re-activates her. The
    // engine records that as an explicit `Active` marker rather than dropping
    // the entry, so `Some(Active)` — not `None` — is the "the 自動 ran" reading.
    move_shiki(&mut game, shiki);
    assert_eq!(
        orientation(&game, shiki),
        Some(CardOrientation::Active),
        "ab#1 自動 must reactivate a ウェイト状態 member that area-moved"
    );

    // GAP 3 — the draw must NOT have repeated when the 自動 resolved.
    assert_eq!(
        hand_len(&game),
        hand_before_cycle + 1,
        "the 自動 activates her; it does not re-run ab#0's カードを1枚引く, so the \
         hand must not have grown a second time"
    );

    // Second move with the gate now shut. Orientation alone cannot separate "the
    // 自動 re-ran and re-set Active" from "it never ran" — both leave
    // `Some(Active)` — so the DRAW is what decides this: a re-run of the pair
    // would draw again.
    let hand_after_first_cycle = hand_len(&game);
    game.state.player1.main_deck.cards.push(game.id(FILLER));
    move_shiki(&mut game, shiki);

    assert_eq!(
        orientation(&game, shiki),
        Some(CardOrientation::Active),
        "with the waited state already consumed, a second move must leave her \
         exactly as the 自動 left her — still アクティブ, with nothing to consume"
    );
    assert_eq!(
        hand_len(&game),
        hand_after_first_cycle,
        "the second, unarmed move must draw nothing — this is the observable that \
         distinguishes 'the 自動 stayed silent' from 'the 自動 re-ran the pair'"
    );

    // Re-arm through the 起動 and the cycle closes: waited → move → active.
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert_eq!(
        orientation(&game, shiki),
        Some(CardOrientation::Wait),
        "ab#0 起動 carries no printed use_limit, so a second activation must be \
         able to wait her again"
    );
    assert_eq!(
        hand_len(&game),
        hand_after_first_cycle + 1,
        "each 起動 activation draws exactly one card"
    );

    move_shiki(&mut game, shiki);
    assert_eq!(
        orientation(&game, shiki),
        Some(CardOrientation::Active),
        "the re-armed 自動 must fire on the next self-move — a gate that latches \
         shut after the first use would leave her stuck ウェイト状態 forever"
    );
}
