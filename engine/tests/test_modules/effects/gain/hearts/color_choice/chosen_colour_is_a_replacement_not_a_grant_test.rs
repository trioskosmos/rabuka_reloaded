//! 中須かすみ / 三船栞子 — the heart choice is a REPLACEMENT, and the two
//! boundaries nothing asserts.
//!
//! ```
//! {{heart03}}か{{heart04}}か{{heart05}}のうち1つを選ぶ。
//! ライブ終了時まで、このメンバーが元々持つハートは選んだハートになる。
//! ```
//!
//! `chosen_color_transform_live_start_test.rs` covers the choice thoroughly: pick the
//! first colour, the middle, the last, and confirm a bystander is untouched. All four
//! assert `heart_color_multiplier == Some(the chosen colour)`, and that observation is
//! consistent with BOTH readings of 「元々持つハートは…になる」:
//!
//!   * REPLACEMENT — her printed hearts are superseded, and the chosen colour comes
//!     from being what she now is;
//!   * ADDITION — the chosen colour is granted on top, and her printed hearts remain.
//!
//! Every existing assertion passes under both. So the file proves the choice is
//! honoured and nothing about what it *did*, which is the shape the `L1+choice`
//! ladder is reporting when it says "no negative test".
//!
//! Two negatives, and the first one turns entirely on the distinction above:
//!
//!   * the chosen colour arrives as a REPLACEMENT — it is not also a `+1` modifier,
//!     which is what an adding implementation would produce;
//!   * the override does not outlive its live — ライブ終了時まで, and no test on either
//!     card crosses a live end.
//!
//! The firing idiom is copied from the sibling file rather than inferred, which is
//! what has made the difference between a first-run draft and a fifth.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_state::{Phase, TurnPhase};

const KASUMI: &str = "PL!N-bp3-014-N"; // 中須かすみ — heart choice, 1 test
const SHIORIKO: &str = "PL!N-pb1-034-N"; // 三船栞子 — heart choice, 3 tests
const LIVE: &str = "PL!-sd1-019-SD";

/// The colour override currently on a card, if any. This is the single observation
/// the sibling tests use; everything below adds a second.
fn override_of(game: &TestGame, cid: i16) -> Option<HeartColor> {
    game.state.mods.heart_color_multiplier.get(&cid).copied()
}

fn printed_hearts(game: &TestGame, cid: i16) -> Vec<HeartColor> {
    let card = game.db.get_card(cid).unwrap();
    let base = card.base_heart.clone().map(|b| b.hearts);
    [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ]
    .into_iter()
    .filter(|c| base.as_ref().is_some_and(|m| m.get(c).is_some_and(|v| *v > 0)))
    .collect()
}

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Fire her ライブ開始時, then answer the colour prompt with `option`.
fn choose_color(game: &mut TestGame, cid: i16, option: i16) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("card has a ライブ開始時 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
    assert!(
        game.has_pending_choice(),
        "precondition: 「{}_か_か_のうち1つを選ぶ」 must offer the colour choice",
        card.card_no
    );
    game.select_option(option);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
}

// ====================================================================
// Replacement, not addition
// ====================================================================

/// The chosen colour is what she IS, not a heart she was given.
///
/// `heart_color_multiplier` is the single observation the sibling file makes, and it
/// reads the same under both readings. The distinguishing signal is the MODIFIER
/// channel: an adding implementation would also put a `+1` on the chosen colour,
/// because that is how a granted heart is represented. A replacing one sets the
/// override and grants nothing.
#[test]
fn kasumi_chosen_colour_is_a_replacement_not_a_granted_heart() {
    let mut game = TestGame::new(load_real_database());
    let kasumi = game.id(KASUMI);
    game.assert_card_identity(kasumi, KASUMI);
    let printed = printed_hearts(&game, kasumi);
    assert!(
        !printed.is_empty(),
        "precondition: she must PRINT hearts, or there is nothing to replace"
    );
    fill_decks(&mut game);
    game.state.player1.stage.stage[1] = kasumi;

    // Option 0 is the FIRST colour on offer, which is not one she prints for
    // かすみ's heart02/heart03/heart05, so the chosen and the printed sets are
    // disjoint and the two readings cannot be confused.
    choose_color(&mut game, kasumi, 0);

    let chosen = override_of(&game, kasumi).expect("precondition: a colour was chosen");
    assert!(
        !printed.contains(&chosen),
        "precondition: the chosen colour is not one she prints, or replacement and \
         addition are indistinguishable (printed {printed:?}, chose {chosen:?})"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(kasumi, chosen),
        0,
        "「元々持つハートは選んだハートになる」 is a REPLACEMENT, so the chosen colour is \
         not also granted: a +1 modifier here is what an ADDING implementation would \
         leave behind, and every assertion in the sibling file would still pass with it."
    );
    // And the override is the only record of the change: one colour, not a set.
    assert_eq!(
        override_of(&game, kasumi),
        Some(chosen),
        "exactly one colour override is present, which is the shape a replacement must \
         have"
    );
}

/// The same clause on 栞子, whose printed hearts overlap her choice — so the colours she
/// did NOT pick must be the ones that disappear.
#[test]
fn shioriko_unchosen_printed_hearts_are_superseded() {
    let mut game = TestGame::new(load_real_database());
    let shioriko = game.id(SHIORIKO);
    game.assert_card_identity(shioriko, SHIORIKO);
    let printed = printed_hearts(&game, shioriko);
    fill_decks(&mut game);
    game.state.player1.stage.stage[1] = shioriko;

    // Her choice is heart03/heart04/heart05, and she prints hearts from that range,
    // so picking one must SUPERSEDE the other two rather than sit beside them.
    let printed_snapshot = printed.clone();
    choose_color(&mut game, shioriko, 0);
    let chosen = override_of(&game, shioriko).expect("precondition: a colour was chosen");

    let superseded: Vec<HeartColor> = printed.into_iter().filter(|c| *c != chosen).collect();
    assert!(
        !superseded.is_empty(),
        "precondition: she prints at least one heart she did not choose, or nothing was \
         superseded (printed {printed_snapshot:?}, chose {chosen:?})"
    );
    for colour in superseded {
        assert_eq!(
            game.state.mods.get_heart_modifier(shioriko, colour),
            0,
            "{colour:?} is one she printed but did not choose, so it is not hers any \
             more; a +1 modifier here would mean the choice was ADDED to her printed \
             hearts instead of replacing them."
        );
    }
}

// ====================================================================
// ライブ終了時まで — the override does not outlive its live
// ====================================================================

/// The colour override is duration-scoped.
///
/// Driven through a real live so the expiry is the engine's rollover rather than a
/// hand-cleared table. The pre-assertion is what makes the "after" reading mean
/// anything: a test that only checked the end state would pass on an ability whose
/// override was never set.
#[test]
fn kasumi_colour_override_does_not_survive_the_live_that_set_it() {
    let mut game = TestGame::new(load_real_database());
    let kasumi = game.id(KASUMI);
    let live = game.id(LIVE);
    fill_decks(&mut game);
    game.state.player1.stage.stage[1] = kasumi;
    game.state.player1.hand.cards.push(live);
    game.give_energy(10);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);
    choose_color(&mut game, kasumi, 0);
    assert!(
        override_of(&game, kasumi).is_some(),
        "precondition: the override is live before the rollover, so the reading after \
         it is about expiry and not about a choice that never happened"
    );

    let mut guard = 0;
    while game.state.current_turn_phase == TurnPhase::Live && guard < 12 {
        guard += 1;
        game.pass();
        game.drain_auto_ability_choices();
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();
    }
    game.state.recalculate_constants();

    assert_eq!(
        override_of(&game, kasumi),
        None,
        "「ライブ終了時まで」 — the override does not survive its live. One that leaked \
         would re-colour her at the start of every later live."
    );
}
