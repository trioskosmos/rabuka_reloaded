//! PL!SP-bp2-021-N ウィーン・マルガレーテ (and her heart06 twin
//! PL!SP-bp2-015-N 平安名すみれ) — the yell-triggered 自動 that never actually yelled.
//!
//!   自動 ターン1回 「エールにより公開された自分のカードの中にブレードハートを持つ
//!   カードがないとき、ライブ終了時まで、heart03（heart06）を得る。」
//!
//! ## The gap
//!
//! The whole `no_blade_heart_reveal_gain` family claimed to cover these two, but
//! every test drove the real live with a **zero-score filler** live card. A live
//! card requiring nothing produces **no yell at all** — a probe showed
//! `revealed_cards = 0` and `yell_occurred = false` in every phase — so
//! `yell_occurred` never became true and the ability correctly never triggered.
//!
//! `wien_yell_no_blade_heart_gain_heart03_until_live_end_test.rs` even names a
//! test `wien_q112_positive_no_blade_heart_triggers_heart03`, asserts `heart == 0`,
//! and blames an "auto-trigger bug". There is no bug: there was no yell. The
//! other tests in that family set `yell_occurred` / `revealed_cards` **by hand**,
//! which proves the condition but never the real pipeline — so nothing verified
//! that a genuine yell actually reaches these jidou.
//!
//! ## What this file proves
//!
//! A real yell, from a live card with a real score requirement, reveals cards
//! that contain no blade heart, and the jidou grants its heart **during the
//! live** (the modifier is ライブ終了時まで, so it must be read before the live
//! ends — reading it afterwards correctly reads 0, which is what the old test
//! observed and misdiagnosed).

use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// A live card with a real score requirement, so performing it produces a genuine
/// yell. A 0-score filler produces none — that is the defect being corrected.
const LIVE_CARD: &str = "PL!S-bp3-020-L";
/// An energy card: no blade heart, so a deck of these makes the yelLED set
/// provably blade-heart-free.
const NO_BLADE_HEART_CARD: &str = "LL-E-001-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// Advance into p1's performance, which is where the yell window runs. The yelLED
/// cards are recorded in `initial_yell_revealed_cards`. `live_card` is the card
/// to perform; pass a 0-score filler to model the "no yell" negative.
fn advance_into_performance(game: &mut TestGame, live_card: &str) {
    for _ in 0..5 {
        game.pass();
    }
    let live = game.id(live_card);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    for _ in 0..3 {
        game.pass();
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();
    }
}

/// Build a live where p1 has a scored member and `watcher` on stage, and the deck
/// is filled with blade-heart-free cards.
fn setup(watcher: i16) -> TestGame {
    let mut game = TestGame::new(load_real_database());
    // A member with score, so the performance over-performs and yells.
    game.state.player1.stage.stage[0] = game.new_id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage[1] = watcher;
    let no_bh = game.id(NO_BLADE_HEART_CARD);
    let filler = game.id(FILLER);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(no_bh);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    game
}

#[test]
fn wien_real_yell_with_no_blade_heart_grants_heart03_during_the_live() {
    let game = TestGame::new(load_real_database());
    let wien = game.id("PL!SP-bp2-021-N");
    game.assert_card_identity(wien, "PL!SP-bp2-021-N");
    let mut game = setup(wien);

    assert_eq!(
        game.state.mods.get_heart_modifier(wien, HeartColor::Heart03),
        0,
        "precondition: no heart before the live"
    );

    advance_into_performance(&mut game, LIVE_CARD);

    // The premise: a REAL yell happened and revealed cards. Without these the
    // test would pass for the same reason the old one "failed".
    assert!(
        game.state.yell_occurred,
        "a real yell must have occurred — a 0-score live card never produces one, \
         which is exactly why this ability was never exercised"
    );
    assert!(
        !game.state.initial_yell_revealed_cards.is_empty(),
        "the yell revealed at least one card; got {:?}",
        game.state.initial_yell_revealed_cards
    );

    assert_eq!(
        game.state.mods.get_heart_modifier(wien, HeartColor::Heart03),
        1,
        "a real yell revealing no blade heart must grant heart03 DURING the live \
         (ライブ終了時まで). The old test read 0 and blamed an 'auto-trigger bug', \
         but its 0-score live card never yelled at all."
    );
}

#[test]
fn sumire_real_yell_with_no_blade_heart_grants_heart06_during_the_live() {
    let game = TestGame::new(load_real_database());
    let sumire = game.id("PL!SP-bp2-015-N");
    game.assert_card_identity(sumire, "PL!SP-bp2-015-N");
    let mut game = setup(sumire);

    advance_into_performance(&mut game, LIVE_CARD);

    assert!(
        game.state.yell_occurred,
        "precondition: a real yell occurred"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        1,
        "sumire's twin jidou must grant heart06 on the same real yell"
    );
}

#[test]
fn no_yell_no_heart_for_both_watchers() {
    // The negative the old fixture accidentally tested: with a 0-score live card
    // there is no yell, so neither watcher may grant. This is CORRECT behaviour
    // and must be pinned, so the positive tests cannot be satisfied by a fixture
    // that simply never yells.
    let mut game = TestGame::new(load_real_database());
    let wien = game.id("PL!SP-bp2-021-N");
    let sumire = game.id("PL!SP-bp2-015-N");
    game.state.player1.stage.stage = [
        game.new_id("PL!S-sd1-003-SD"),
        sumire,
        wien,
    ];
    let filler = game.id(FILLER);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_into_performance(&mut game, FILLER); // 0-score live => no yell

    assert!(
        !game.state.yell_occurred,
        "precondition: a 0-score live card produces no yell at all"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(wien, HeartColor::Heart03),
        0,
        "no yell → no heart03"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        0,
        "no yell → no heart06"
    );
}
