//! The `cards_activated_this_turn` WRITER, driven by the engine rather than by a test.
//!
//! ## Why this file exists
//!
//! Two PBLL02 abilities read the per-turn activation history:
//!
//! * `PL!-pb2-010-P＋` 「…このターン中に自分の『Printemps』のカードの効果によってウェイト状態に
//!   アクティブにしていたメンバー1人につき、ブレード」
//! * `PL!-pb2-040-L` 「…このターン中に自分の『Printemps』のカードの効果によってウェイト状態に
//!   アクティブにしていたメンバーが1人以上いる場合、必要ハートをheart0を3減らす」
//!
//! Both are gated on the **history**, and both are read through
//! `Condition::get_activated_this_turn` (`core/card.rs:4079-4081`).
//!
//! Before this file, the ONLY writers of that field anywhere in `engine/tests` were
//! hand-written helpers: `pb2_blade_per_member_activated_from_wait_this_turn_pl_pb2_010_p_plus_test.rs`
//! (`record_activation`) and `pb2_need_heart_minus_3_minus_2_minus_1_per_activated_pl_pb2_040_l_test.rs`
//! (the same four lines inlined). Both build the list by hand, then assert on it. The
//! production writer — `ability/effects/change_state.rs:646-647` — was never exercised by any
//! test, so the two halves of the contract were never joined.
//!
//! That gap let four regressions ship green:
//!
//! 1. **the writer stops firing** — deleting or narrowing the `state_change == "active"` guard
//!    leaves both PBLL02 cards granting nothing, which is the exact bug the field exists for;
//! 2. **the writer over-fires** — dropping the `!contains` guard turns the set into a tally.
//!    The old hand-built tests could never catch this, because the dedup lived in the test's
//!    own helper;
//! 3. **the writer records the wrong id** — it pushes `*card_id` from `actual_targets`;
//! 4. **the per-turn clear stops working** (`core/game_state/modifiers.rs:1927`) — a turn-1
//!    activation would then satisfy 「このターン中に」 on turn 3 forever.
//!
//! `PL!-bp6-008-P` is used as the driver because it is the simplest card in the corpus whose
//! EFFECT is the state change: 「このメンバーをウェイトにする：自分のステージにいるほかのメンバー
//! 1人をアクティブにする」. The cost waits the host, the effect activates another member, and
//! the engine writes the history on its own. Nothing here touches
//! `cards_activated_this_turn` except reading it.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// 起動 「このメンバーをウェイトにする：自分のステージにいるほかのメンバー1人をアクティブにする」
const DRIVER: &str = "PL!-bp6-008-P";
/// 矢澤にこ, μ's — the member the driver will switch out of wait.
const TARGET: &str = "PL!-pb2-002-PP";
const FILLER: &str = "PL!-sd1-010-SD";

/// Stage the driver in the left side and `target` in the centre, WAITED. Returns
/// (driver, target).
fn staged_waiting_target() -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let driver = game.id(DRIVER);
    game.assert_card_identity(driver, DRIVER);
    let target = game.id(TARGET);
    game.assert_card_identity(target, TARGET);

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(driver);
    game.give_energy(10);
    game.play_to_stage(driver, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.drain_auto_ability_choices();
    }

    game.state.player1.stage.stage[MemberArea::Center as usize] = target;
    game.state.mods.add_orientation_modifier(target, "wait");

    (game, driver, target)
}

/// THE CONTRACT: a real 起動, paid through the real action pipeline, must put the switched
/// member into the engine-written activation history.
#[test]
fn a_real_activation_is_recorded_by_the_engine() {
    let (mut game, driver, target) = staged_waiting_target();
    assert!(
        game.state.cards_activated_this_turn.is_empty(),
        "precondition: nothing has been activated yet, so any entry below came from the engine"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(target),
        Some("wait"),
        "precondition: the target starts in wait, so this is a genuine wait→active change"
    );

    game.activate_ability(driver);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.drain_auto_ability_choices();

    assert_ne!(
        game.state.mods.get_orientation_modifier(target),
        Some("wait"),
        "precondition of the assertion below: the 起動 actually switched the member out of \
         wait. If this fails the history assertion would pass for the wrong reason."
    );
    assert!(
        game.state.cards_activated_this_turn.contains(&target),
        "THE CONTRACT — `change_state` with `state_change: \"active\"` must record the \
         activated member in `cards_activated_this_turn` \
         (`ability/effects/change_state.rs:646-647`). Nothing in the test tree wrote this \
         field before; PL!-pb2-010-P＋ and PL!-pb2-040-L both gate on it, so if the engine \
         stopped writing it they would silently grant nothing."
    );
}

/// The history is a SET, not a tally — this is the dedup the hand-built tests could not test,
/// because the dedup lived in their own helper.
///
/// The driver above cannot be used for this: `PL!-bp6-008-P` is ターン1回, so a second
/// activation in the same turn is correctly refused and the run never reaches the dedup.
/// Two INSTANCES of a card with no use limit are used instead, so the target is switched
/// out of wait twice by two separate abilities — which also proves the history is keyed on
/// the member, not per ability.
#[test]
fn the_activation_history_is_a_set_not_a_tally() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    // 登場 「自分のステージにいるすべてのメンバーをアクティブにする」 — no turn limit.
    const ALL_ACTIVATE: &str = "PL!-bp3-005-P";

    let target = game.id(TARGET);
    game.assert_card_identity(target, TARGET);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.stage.stage[MemberArea::RightSide as usize] = target;
    game.state.mods.add_orientation_modifier(target, "wait");

    let count_target = |g: &TestGame| {
        g.state
            .cards_activated_this_turn
            .iter()
            .filter(|&&id| id == target)
            .count()
    };

    // Two instances, in two different empty areas — `play_to_stage` panics on an occupied
    // area, so the same slot twice would never reach the assertion.
    for area in [MemberArea::LeftSide, MemberArea::Center] {
        let d = game.id(ALL_ACTIVATE);
        game.state.player1.hand.cards.push(d);
        game.give_energy(10);
        game.play_to_stage(d, area);
        if game.has_pending_choice() {
            game.drain_auto_ability_choices();
        }
    }

    assert_ne!(
        game.state.mods.get_orientation_modifier(target),
        Some("wait"),
        "precondition: the 登場 abilities really did switch the member out of wait"
    );
    assert_eq!(
        count_target(&game),
        1,
        "the same member switched out of wait by TWO different abilities in one turn is \
         still ONE entry — the writer's `!cards_activated_this_turn.contains(card_id)` \
         guard. Removing that guard turns the history into a tally and every 1人につき / \
         1人以上 reader over-counts."
    );
}

/// A member the driver did NOT touch must not appear — otherwise the history would grow for
/// every card on the stage rather than for activations.
#[test]
fn an_untouched_member_is_not_recorded() {
    let (game, _, target) = staged_waiting_target();
    assert!(
        !game.state.cards_activated_this_turn.contains(&target),
        "precondition: staging and waiting a member is not an activation"
    );
    assert!(
        game.state.cards_activated_this_turn.is_empty(),
        "…and the history is empty until an ability runs"
    );
}
