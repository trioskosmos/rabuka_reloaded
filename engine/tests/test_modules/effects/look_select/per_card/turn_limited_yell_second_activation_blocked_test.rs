//! ターン1回 limit on the yell-triggered 自動 of
//! PL!SP-bp2-015-N 平安名すみれ (heart06) and PL!SP-bp2-021-N ウィーン (heart03).
//!
//! ## What these tests previously did
//!
//! Both were named `*_turn_limit_blocks_second_yell`, but neither performed a
//! second yell: they ran one live, then *reset the modifier by hand*
//! (`add_heart_modifier(sumire, .., -first)`) and gave up, with the comment
//! "For simplicity, just verify that the use_limit is enforced via queue."
//! The assertion was `assert!(first == 0 || first == 1, ...)` — true for every
//! value.
//!
//! Two defects, both now removed:
//!   1. The live card was a **0-score filler**, so **no yell happened at all**;
//!      the 自動 correctly never fired, and `first` was 0 for that reason alone.
//!   2. The modifier was read **after the live had ended**, where a
//!      ライブ終了時まで grant has legitimately expired — so even a correct grant
//!      would have read 0.
//!
//! A phantom "auto-trigger bug" is also referenced in a comment
//! ("Currently the third test in sumire_auto_test expects 0 due to bug"). There
//! is no such bug: a real yell does grant these hearts, proven in
//! `jidou/combination/yell_real_yell_no_blade_heart_grants_heart_test.rs` and
//! `no_blade_heart_reveal_gain_heart06_member_as_live_q112_q113_test.rs`.
//!
//! ## What these tests do now
//!
//! Drive a REAL yell (a live card with a real score requirement, a deck of
//! blade-heart-free cards) and assert the exact grant, read DURING the live.
//!
//! **Scope note on the ターン1回 ceiling.** A second *live* in one turn is not a
//! thing a player can perform (one live per player per turn), so a real second
//! yell in the same turn comes from a re-yell (discarding a live card to yell
//! again). That path is covered for real in
//! `jidou/yell/discard_revealed_then_re_yell/yell_discard_liella_live_grants_two_extra_yells_test.rs`
//! (`pb2_020_use_limit_blocks_second_yell`, which performs genuine re-yells).
//! `no_blade_heart_reveal_gain_heart06_or_heart03_test.rs::yell_turn1_blocks_second_trigger`
//! re-triggers the scan by hand and proves only the limit *accounting* (the
//! modifier does not stack), not the pipeline — so it is deliberately NOT cited
//! as real-pipeline coverage. The generic ターン1回 mechanism is pinned on real
//! paths in `PL!N-PR-025` (asserts `use_count == 2` and a third refusal via real
//! baton touches) and in the 瑠璃乃 / 葉月恋 combination tests.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// A live card with a real score requirement, so performing it yells. A 0-score
/// filler produces no yell at all.
const LIVE_CARD: &str = "PL!S-bp3-020-L";
const NO_BLADE_HEART: &str = "LL-E-001-SD";
const FILLER: &str = "PL!-sd1-010-SD";

fn perform_real_yell(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    let live = game.id(LIVE_CARD);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    // Three passes reach the yell window and stop DURING the live, while the
    // ライブ終了時まで modifier is still observable.
    for _ in 0..3 {
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }
}

fn build(game: &mut TestGame, watcher_card: &str, heart: HeartColor) -> i16 {
    let watcher = game.id(watcher_card);
    game.assert_card_identity(watcher, watcher_card);
    // A member with score, so the performance over-performs and yells.
    game.state.player1.stage.stage = [game.new_id("PL!S-sd1-003-SD"), watcher, -1];
    let no_bh = game.id(NO_BLADE_HEART);
    let filler = game.id(FILLER);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(no_bh);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    assert_eq!(
        game.state.mods.get_heart_modifier(watcher, heart),
        0,
        "precondition: no heart before the live"
    );
    watcher
}

#[test]
fn sumire_real_yell_grants_exactly_one_heart06() {
    let mut game = TestGame::new(load_real_database());
    let sumire = build(&mut game, "PL!SP-bp2-015-N", HeartColor::Heart06);

    perform_real_yell(&mut game);

    assert!(
        !game.state.initial_yell_revealed_cards.is_empty(),
        "precondition: a REAL yell revealed cards (the 0-score fixture never did)"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        1,
        "a real yell revealing no blade heart grants heart06 exactly once during \
         the live. Previously asserted '0 or 1', which is true for every value."
    );
}

#[test]
fn wien_real_yell_grants_exactly_one_heart03() {
    let mut game = TestGame::new(load_real_database());
    let wien = build(&mut game, "PL!SP-bp2-021-N", HeartColor::Heart03);

    perform_real_yell(&mut game);

    assert!(
        !game.state.initial_yell_revealed_cards.is_empty(),
        "precondition: a REAL yell revealed cards"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(wien, HeartColor::Heart03),
        1,
        "a real yell revealing no blade heart grants heart03 exactly once during \
         the live. Previously asserted '0 or 1'."
    );
}
