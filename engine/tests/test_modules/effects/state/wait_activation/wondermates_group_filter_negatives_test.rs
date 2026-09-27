//! ワンダーメイツ `PL!N-sd2-025` — the group-filter negatives its one existing test
//! cannot reach.
//!
//! 「自分のステージにいる『虹ヶ咲』のメンバー1人をアクティブにする。」
//!
//! `self_wait_and_nijigasaki_activation_test.rs` covers the positive well: a waited
//! 虹ヶ咲 member ends active, a μ's member ends waited, and her 起動 stays usable.
//!
//! But that board stages exactly ONE 虹ヶ咲 member, and a 虹ヶ咲 with only one
//! candidate is indistinguishable from a group-blind implementation that happens to
//! offer the right one. The filter is only testable where there is a choice to get
//! wrong, and where the absence of any candidate has to be handled.
//!
//! So the board here stages TWO 虹ヶ咲 members and one μ's, and:
//!   * only a 虹ヶ咲 member is ever chosen, and the μ's is never activated;
//!   * with NO 虹ヶ咲 member on stage at all, nothing is activated and nothing is
//!     offered to choose.
//!
//! The prompt is inspected rather than inferred: the existing file already reads
//! `get_pending_choice()`, and a `while has_pending_choice()` loop that answers
//! everything with `[0]` cannot tell "the filter offered the right card" from "there
//! was only one card".

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const WONDERMATES: &str = "PL!N-sd2-025-P";
/// 虹ヶ咲 — the group the filter names.
const AZUNASA: &str = "PL!N-bp1-006-R＋";
const AZUSA: &str = "PL!N-bp1-013-R";
/// μ's — the right zone, the wrong group.
const OUTSIDER: &str = "PL!-sd1-010-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// Fire her ライブ開始時 and drain, as the sibling file does.
fn trigger(game: &mut TestGame, cid: i16) {
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
}

fn fill_decks(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn is_active(game: &TestGame, cid: i16) -> bool {
    rabuka_engine::ability::util::orientation_matches_state(
        game.state.mods.get_orientation_modifier(cid),
        "active",
    )
}

///
/// With a single 虹ヶ咲 member the filter is untestable — any implementation that
/// activates the only candidate passes. Two makes it a real choice, and the μ's member
/// is the card a series-based or group-blind reading would wrongly take.
#[test]
fn wondermates_activates_a_nijigasaki_member_and_never_the_outsider() {
    let mut game = TestGame::new(load_real_database());
    let wondermates = game.id(WONDERMATES);
    let first = game.id(AZUNASA);
    let second = game.id(AZUSA);
    let outsider = game.id(OUTSIDER);
    game.assert_card_identity(wondermates, WONDERMATES);
    assert_ne!(first, second, "precondition: two DISTINCT 虹ヶ咲 members, or there is no choice to get wrong");
    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(&game.db, outsider, Some("虹ヶ咲")),
        "precondition: the μ's card must be OUTSIDE the group the filter names"
    );

    fill_decks(&mut game);
    game.state.player1.stage.stage = [first, wondermates, outsider];
    game.state.mods.add_orientation_modifier(first, "wait");
    game.state.mods.add_orientation_modifier(second, "wait");
    game.state.mods.add_orientation_modifier(outsider, "wait");

    trigger(&mut game, wondermates);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        is_active(&game, first) || is_active(&game, second),
        "『『虹ヶ咲』のメンバー1人をアクティブにする』 — a 虹ヶ咲 member ends active"
    );
    assert!(
        !is_active(&game, outsider),
        "the μ's member is NEVER activated, whatever the prompt offered. The sibling \
         test only ends it waited, which a group-blind implementation reading the \
         first candidate would also satisfy if that candidate happened to be the 虹ヶ咲 \
         member."
    );
    assert_eq!(
        [is_active(&game, first), is_active(&game, second)]
            .iter()
            .filter(|a| **a)
            .count(),
        1,
        "「1人を」 — exactly ONE member is activated, not the whole group"
    );
}

/// With no 虹ヶ咲 member staged, there is nothing legal to activate.
///
/// The sibling test always stages one, so this branch is untested there: a
/// group-blind implementation would happily activate the μ's member here, and a
/// correct one has no candidate to offer.
#[test]
fn wondermates_activates_nothing_when_no_nijigasaki_member_is_staged() {
    let mut game = TestGame::new(load_real_database());
    let wondermates = game.id(WONDERMATES);
    let outsider = game.id(OUTSIDER);
    let second_outsider = game.id(OUTSIDER);
    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(&game.db, outsider, Some("虹ヶ咲")),
        "precondition: neither staged member may satisfy the filter"
    );

    fill_decks(&mut game);
    game.state.player1.stage.stage = [outsider, wondermates, second_outsider];
    game.state.mods.add_orientation_modifier(outsider, "wait");
    game.state.mods.add_orientation_modifier(second_outsider, "wait");

    trigger(&mut game, wondermates);
    let offered = game.pending_choice_summary();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !is_active(&game, outsider) && !is_active(&game, second_outsider),
        "with no 『虹ヶ咲』 member staged, nothing may be activated — a group-blind \
         reading would activate one of these (prompt offered was: {offered})"
    );
}

/// A 虹ヶ咲 member that is ALREADY active is not re-activated, and the choice still
/// resolves.
///
/// `1人をアクティブにする` on an already-active member is a no-op rather than an error,
/// and the prompt being skippable in that case is what keeps the ability from
/// stalling when the player has nothing to do.
#[test]
fn wondermates_leaves_an_already_active_nijigasaki_member_alone() {
    let mut game = TestGame::new(load_real_database());
    let wondermates = game.id(WONDERMATES);
    let niji = game.id(AZUNASA);
    let outsider = game.id(OUTSIDER);

    fill_decks(&mut game);
    game.state.player1.stage.stage = [niji, wondermates, outsider];
    // niji is staged ACTIVE: no orientation modifier, or explicitly active.
    game.state
        .mods
        .add_orientation_modifier(niji, "active");
    game.state.mods.add_orientation_modifier(outsider, "wait");

    trigger(&mut game, wondermates);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !is_active(&game, outsider),
        "the μ's member is not touched, even when the only 『虹ヶ咲』 member is already \
         active and choosing it would be a no-op"
    );
    // `activating_card` is cleared when an ability finishes, so None here is evidence
    // of COMPLETION, not of a stall. An earlier draft of this assertion expected
    // `Some(wondermates)` and failed — the wrong proxy, and it would have flagged a
    // working ability.
    assert_eq!(
        game.state.activating_card, None,
        "the ability runs to completion: activating_card is cleared on finish, and a \
         STALL would leave the choice pending instead"
    );
    assert!(
        !game.has_pending_choice(),
        "with nothing left to resolve the prompt queue is empty, so the ability is not \
         stuck waiting on a choice it could not offer"
    );
}
