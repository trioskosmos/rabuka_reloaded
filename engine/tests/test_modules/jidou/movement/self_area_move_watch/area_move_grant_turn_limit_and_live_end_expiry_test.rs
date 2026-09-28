//! 澁谷かのん `PL!SP-sd2-012-SD2` and 鬼塚冬毬 `PL!SP-sd2-022-SD2` — the two
//! lifecycle clauses their existing tests do not reach.
//!
//! ```
//! {{自動}}{{ターン1回}}このメンバーがエリアを移動したとき、ライブ終了時まで、
//! {{heart02}}を得る。        (対戦相手のカードの効果でも発動する。)
//! ```
//!
//! `cross_player_jidou_triggers_test.rs` already covers the parts of these two that
//! are about the TRIGGER firing: a self-caused move grants the heart, and an
//! OPPONENT-caused move grants it too, which is the 「でも発動する」 exception and the
//! reason these cards are special. What no test on either card asserts is the two
//! clauses that bound the grant:
//!
//!   * {{ターン1回}} — a SECOND area move in the same turn must not add a second
//!     heart. `koko_turn1_blocks_second` does exactly this, for 幺頭
//!     `PL!SP-sd2-002-SD2`; the two cards here have no equivalent.
//!   * {{ライブ終了時まで}} — the grant is duration-scoped and must not survive the
//!     live. No test on either card crosses a live end.
//!
//! So both are here, on both cards. Without them the pair reads as "covered": the
//! depth ladder calls these rows L1+choice (no negative assertion) even though the
//! tests assert exact positive values, because `assert_eq!(x, 1)` is not an absence
//! SHAPE and the ladder keys on shapes. That is a real limitation of the metric,
//! recorded in its own comment rather than worked around.
//!
//! The movement idiom is the one the cross-player file already uses:
//! `push_movement_event(card, from, to, Some(source), player, true)` then
//! `trigger_auto_abilities_for_player` + `process_pending_auto_abilities`. The final
//! flag is `true` for self-caused and opponent-caused moves alike — the PLAYER string
//! is what separates them, not that flag.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::game_state::{Phase, TurnPhase};
use rabuka_engine::turn::TurnEngine;

const KANON: &str = "PL!SP-sd2-012-SD2"; // 澁谷かのん — area move -> heart02
const FUYUMARI: &str = "PL!SP-sd2-022-SD2"; // 鬼塚冬毬 — area move -> heart03
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Push an area move caused by `by` and run p1's 自動 to completion.
///
/// The last `push_movement_event` argument is `true` in BOTH cases in
/// `cross_player_jidou_triggers_test.rs` — self-caused and opponent-caused alike — so it
/// is not the "opponent effect" flag. What distinguishes the two is the PLAYER string:
/// `"p1"` for a move p1 caused, `"p2"` for one p2 caused. Reading that off the file
/// rather than inferring it is the difference between this working first try and the
/// four failures before it.
fn move_and_trigger(game: &mut TestGame, who: i16) {
    game.state
        .push_movement_event(who, "stage", "stage", Some(who), "p1", true);
    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

fn heart_of(game: &TestGame, who: i16, colour: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(who, colour)
}

// ====================================================================
// ターン1回 — the second move in a turn adds nothing
// ====================================================================

/// A second area move in the same turn must not stack a second heart.
///
/// The grant is 「{{ターン1回}}」, and the sibling card 幺頭 has this test while these
/// two do not. Without it the only way to catch a 自動 that re-fires on every move is
/// a live that moves one member twice — and nothing in either card's coverage does
/// that.
#[test]
fn kanon_turn1_blocks_a_second_area_move() {
    let mut game = TestGame::new(load_real_database());
    let kanon = game.id(KANON);
    game.assert_card_identity(kanon, KANON);
    assert_eq!(game.db.get_card(kanon).unwrap().name, "澁谷かのん");
    fill_decks(&mut game);
    game.state.player1.stage.stage[1] = kanon;

    move_and_trigger(&mut game, kanon);
    assert_eq!(
        heart_of(&game, kanon, HeartColor::Heart02),
        1,
        "the first move grants heart02"
    );

    move_and_trigger(&mut game, kanon);
    assert_eq!(
        heart_of(&game, kanon, HeartColor::Heart02),
        1,
        "「{{ターン1回}}」 — a second area move in the same turn adds nothing, so the \
         value is still 1 rather than 2"
    );

    move_and_trigger(&mut game, kanon);
    assert_eq!(
        heart_of(&game, kanon, HeartColor::Heart02),
        1,
        "and a third is no different: the limit is per turn, not per move"
    );
}

/// The same clause on 冬毬's heart03, because the two are separate ability records
/// with separate use-limit state and neither can stand in for the other.
#[test]
fn fuyumari_turn1_blocks_a_second_area_move() {
    let mut game = TestGame::new(load_real_database());
    let fuyu = game.id(FUYUMARI);
    game.assert_card_identity(fuyu, FUYUMARI);
    assert_eq!(game.db.get_card(fuyu).unwrap().name, "鬼塚冬毬");
    fill_decks(&mut game);
    game.state.player1.stage.stage[1] = fuyu;

    move_and_trigger(&mut game, fuyu);
    assert_eq!(
        heart_of(&game, fuyu, HeartColor::Heart03),
        1,
        "the first move grants heart03"
    );

    move_and_trigger(&mut game, fuyu);
    assert_eq!(
        heart_of(&game, fuyu, HeartColor::Heart03),
        1,
        "「{{ターン1回}}」 — the second move adds nothing"
    );
}

// ====================================================================
// ライブ終了時まで — the grant does not outlive its live
// ====================================================================

/// The grant is duration-scoped, so it must be gone after the live ends.
///
/// Driven through a REAL live: the phase walk crosses the Draw phase and the victory
/// determination, and `execute_live_victory_determination` is what runs the expiry.
/// Asserting the grant exists BEFORE the rollover is what makes the "after" reading
/// meaningful — a test that only checked the end state would pass on a grant that
/// never fired.
#[test]
fn kanon_heart02_does_not_survive_the_live_that_granted_it() {
    let mut game = TestGame::new(load_real_database());
    let kanon = game.id(KANON);
    let live = game.id("PL!-sd1-019-SD");
    fill_decks(&mut game);
    game.state.player1.stage.stage[1] = kanon;
    game.state.player1.hand.cards.push(live);
    game.give_energy(10);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);

    // Grant it while the live is running.
    move_and_trigger(&mut game, kanon);
    assert_eq!(
        heart_of(&game, kanon, HeartColor::Heart02),
        1,
        "precondition: the grant is live before the rollover, so the 'after' reading \
         below is about expiry and not about an ability that never fired"
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
        heart_of(&game, kanon, HeartColor::Heart02),
        0,
        "「ライブ終了時まで」 — the grant does not survive the live. A live_end grant \
         that leaks applies to every later live in the match."
    );
}

/// The same duration clause on 冬毬.
#[test]
fn fuyumari_heart03_does_not_survive_the_live_that_granted_it() {
    let mut game = TestGame::new(load_real_database());
    let fuyu = game.id(FUYUMARI);
    let live = game.id("PL!-sd1-019-SD");
    fill_decks(&mut game);
    game.state.player1.stage.stage[1] = fuyu;
    game.state.player1.hand.cards.push(live);
    game.give_energy(10);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(live);

    move_and_trigger(&mut game, fuyu);
    assert_eq!(
        heart_of(&game, fuyu, HeartColor::Heart03),
        1,
        "precondition: the grant is live before the rollover"
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
        heart_of(&game, fuyu, HeartColor::Heart03),
        0,
        "「ライブ終了時まで」 — the grant does not survive its live"
    );
}
