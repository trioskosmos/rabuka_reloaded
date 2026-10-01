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

/// A real yell reveals one card per point of BLADE, all distinct — the rule, asserted.
///
/// ## The count is the blade total, not five
///
/// This started as an `#[ignore]`d `println!` diagnostic, and it was showing the wrong thing on
/// two counts. Chasing Q182 I saw a yelLED set of `[11329, 11329, 11329, 11329, 11329]` — one
/// id five times — and wrote it up as "the widest issue in the sweep". It was a harness
/// artefact: instances past the 11-copy pool alias to the template id
/// (`tests/helpers/mod.rs:234`) and that version filled the deck by pushing ONE id sixty times.
/// It also never yelled at all — its stage had no score, so `yell_occurred` was false and the
/// set it "diagnosed" was empty. A `println!` diagnostic cannot fail, so it sat there
/// documenting a bug that was not there.
///
/// The count is **not five**. `TurnEngine::reveal_yell_cards` (`turn/live.rs`) reveals
/// `total_blade` cards, and its own comment carries the rule reference: "Q40: Yell must
/// complete ALL checks — even if hearts are already satisfied, the full blade-count of yell
/// cards is always revealed." So a yell checks as many cards as you have BLADE. On the board
/// below, 松浦果南 (blade 3) + ウィーン (blade 1) = 4, and a yell correctly reveals 4.
///
/// I got that wrong first — I asserted five from the board game rather than reading this
/// engine's rule, and the failure is what surfaced `reveal_yell_cards`. The test now pins the
/// blade total, which is a far stronger claim than a constant: it holds the engine to the rule
/// rather than to a number that happens to be right today.
///
/// The lesson the old diagnostic actually taught is kept as a precondition: **before
/// concluding anything about the contents of a revealed / drawn / selected set, confirm the deck
/// was built from distinct instances.** The deck here is built from six different card NUMBERS,
/// interleaved — `new_id` is not sufficient on its own, since past the 11-copy pool it aliases
/// to the template id just like `id`.
#[test]
fn a_real_yell_reveals_one_card_per_blade_and_all_of_them_are_distinct() {
    // The board `setup()` builds, inlined so the deck can be made distinct: 松浦果南 (blade 3)
    // so the performance yells, plus a watcher (blade 1). `setup()` fills the deck with 60
    // copies of ONE id, which is the very thing under test, so it is not used here.
    let mut game = TestGame::new(load_real_database());
    let scored = game.new_id("PL!S-sd1-003-SD");
    let watcher = game.id("PL!SP-bp2-021-N");
    game.assert_card_identity(scored, "PL!S-sd1-003-SD");
    game.assert_card_identity(watcher, "PL!SP-bp2-021-N");
    game.state.player1.stage.stage[0] = scored;
    game.state.player1.stage.stage[1] = watcher;
    game.give_energy(15);

    // The expected count is the stage's blade total, read from the CARD DATA rather than
    // hard-coded, so the test states the rule instead of a number.
    let expected_blade: i32 = [scored, watcher]
        .iter()
        .map(|&id| game.db.get_card(id).map(|c| i32::from(c.blade)).unwrap_or(0))
        .sum();
    assert_eq!(
        expected_blade, 4,
        "precondition: 松浦果南 (blade 3) + ウィーン (blade 1); if these prints changed, this \
         test is measuring a different board"
    );

    // Rebuild the deck from SIX different card numbers, INTERLEAVED rather than blocked, so
    // that any prefix of the deck holds pairwise-distinct ids. A blocked deck (ten copies of
    // A, then ten of B) would hand a distinct-looking test a run of identical cards at the
    // top, which measures the deck layout instead of the rule.
    let deck_cycle = [
        NO_BLADE_HEART_CARD,
        "PL!-PR-014-PR",
        "PL!-PR-017-PR",
        "PL!-bp3-012-N",
        "PL!-bp3-012-PR",
        FILLER,
    ];
    game.state.player1.main_deck.cards.clear();
    let deck_ids: Vec<i16> = deck_cycle.iter().map(|n| game.id(n)).collect();
    for _ in 0..10 {
        for &cid in &deck_ids {
            game.state.player1.main_deck.cards.push(cid);
        }
    }
    let mut sorted_deck = deck_ids.clone();
    sorted_deck.sort_unstable();
    sorted_deck.dedup();
    assert_eq!(
        sorted_deck.len(),
        deck_cycle.len(),
        "precondition: every deck entry must be a distinct instance, or this test measures \
         the aliasing instead of the rule (ids={deck_ids:?})"
    );

    advance_into_performance(&mut game, LIVE_CARD);

    assert!(
        game.state.yell_occurred,
        "precondition: the stage scores, so a performance yell happened"
    );
    let yelled = &game.state.initial_yell_revealed_cards;
    assert_eq!(
        yelled.len() as i32, expected_blade,
        "a yell reveals one card per point of blade (reveal_yell_cards, Q40), and completes \
         all checks even once hearts are satisfied. Got {yelled:?}"
    );
    let mut distinct = yelled.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        yelled.len(),
        "with an interleaved deck, every revealed card is a different instance — a repeat \
         here means the deck was aliased or the draw re-used an entry, not that the yell is \
         broken. Revealed {yelled:?} from deck {deck_ids:?}"
    );
    for id in yelled {
        assert!(
            deck_ids.contains(id),
            "every revealed card must come from the deck; {id} was not in it (deck={deck_ids:?})"
        );
    }
    // The performance's own reveal is a separate set from the initial yell; the rule under
    // test is the initial one, but assert the performance did resolve so a future change
    // cannot leave the live silently unresolved.
    assert!(
        !game.state.revealed_cards.is_empty(),
        "the performance must have resolved to a reveal"
    );
}

#[test]
fn wien_real_yell_with_no_blade_heart_grants_heart03_during_the_live() {
    let game = TestGame::new(load_real_database());    let wien = game.id("PL!SP-bp2-021-N");
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
