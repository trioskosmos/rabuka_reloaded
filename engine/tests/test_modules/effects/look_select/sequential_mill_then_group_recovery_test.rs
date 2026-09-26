//! A `sequential` whose SECOND step reads the RESULT of its first — the shape where
//! a wrong ORDER is invisible to a test that checks only the end state.
//!
//! 高坂穂乃果 `PL!-bp5-010-N` — 「手札を1枚控え室を置いてもよい：自分のデッキの上から
//! カードを3枚控え室に置く。**その後**、自分の控え室から『A-RISE』のメンバーカードを
//! 1枚手札に加える。」
//!
//! The parsed steps are `move_cards deck_top → discard, count 3` then
//! `move_cards source: "discard" → hand, count 1, card_type: member_card,
//! group: A-RISE`. 「その後」 is load-bearing in both directions:
//!
//!   * step 2 can only find anything because step 1 put it there, so a sequential
//!     that ran step 2 first — or against a zone snapshot from before step 1 — has
//!     nothing to recover and silently does half its work;
//!   * step 2 filters by group, so what returns is ONE member out of the three just
//!     milled, not the mill itself.
//!
//! ## Three harness facts this file had to be written around
//!
//! **The real live walk draws before the trigger fires.** Seven `pass()` calls from
//! the start of the turn reach ライブ開始時, and the Draw phase in that window takes
//! a card off the deck first. So "seed the deck top" is undone before the ability
//! resolves — three earlier drafts put their cards at index 0, watched the Draw eat
//! them, and concluded (wrongly) that step 2 was dead. `MILLED` is therefore pushed
//! after one sacrificial card that the Draw absorbs.
//!
//! **`game.id()` allocates a fresh instance on every call.** A premise that calls
//! `game.id(ARISE)` to assert the group, and a placement that calls
//! `game.new_id(ARISE)` to seed, are two DIFFERENT cards — so the card asserted on
//! is not the card placed, and the test measures nothing. Every id here is bound
//! once and reused for both.
//!
//! **The cost and the gain are the same size.** The printed cost discards one card
//! and the recovery adds one, so a hand count taken across the whole resolution is
//! 0 whether or not the recovery worked. Every assertion is a DELTA from the
//! snapshot taken at ライブ開始時, with the draw already accounted for.
//!
//! `A-RISE` is a UNIT-level name matched exactly against the card's unit field;
//! `Printemps` and `BiBi` members do not satisfy it. That is asserted, not assumed,
//! so a card-pool edit cannot quietly make the negative arm vacuous.

use crate::helpers::*;

const HONOKA: &str = "PL!-bp5-010-N"; // 高坂穂乃果 — mill 3, then recover an 『A-RISE』 member
/// An 『A-RISE』 member, which the second step can recover.
const ARISE: &str = "PL!-bp5-222-R"; // 優木あんじゅ
/// A member that is not 『A-RISE』 — the negative for the group filter. It IS a
/// member, so the type filter is satisfied and only the group can exclude it.
const NOT_ARISE: &str = "PL!-sd1-001-SD"; // 高坂穂乃果, unit Printemps
const FILLER: &str = "PL!-sd1-010-SD";
/// A vehicle live, only so there is a live to perform.
const LIVE: &str = "PL!-sd1-019-SD";

/// The pools the assertions read.
#[derive(Debug, Clone, Copy)]
struct Pools {
    deck: usize,
    hand: usize,
    waitroom: usize,
}

fn pools(game: &TestGame) -> Pools {
    Pools {
        deck: game.state.player1.main_deck.cards.len(),
        hand: game.state.player1.hand.cards.len(),
        waitroom: game.state.player1.waitroom.cards.len(),
    }
}

/// Drive 高坂穂乃果's ライブ開始時 with `milled` as the three cards the mill will
/// take, and return the pools at ライブ開始時 and after the ability resolves.
///
/// `milled` is pushed AFTER one sacrificial card, because the walk's Draw phase
/// takes index 0 before the trigger fires. Binding the ids once and reusing them
/// is what makes the placement match the premise.
fn resolve_with_milled(game: &mut TestGame, milled: &[&str]) -> (Pools, Pools) {
    let filler = game.id(FILLER);
    let honoka = game.id(HONOKA);
    let live = game.id(LIVE);
    let sacrificial = game.new_id(FILLER);
    let placed: Vec<i16> = milled.iter().map(|c| game.new_id(c)).collect();

    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    // Index 0 first: the Draw phase in the live walk takes this one.
    game.state.player1.main_deck.cards.push(sacrificial);
    for id in &placed {
        game.state.player1.main_deck.cards.push(*id);
    }
    for _ in 0..40 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    // Cards in hand so the optional cost is payable.
    for _ in 0..3 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.stage.stage = [honoka, -1, -1];
    // Zone the live directly: `set_live_card` needs it in hand and routes the walk
    // differently, and the cost does not prompt on that path.
    game.state.player1.live_card_zone.cards.push(live);
    game.give_energy(5);

    // → Active, Energy, Draw, Main, LiveCardSetP1, LiveCardSetP2, LiveStart
    for _ in 0..7 {
        game.pass();
    }
    let at_live_start = pools(game);

    // The one prompt is the optional cost; the cost, the mill and the recovery all
    // resolve on it.
    assert!(
        game.has_pending_choice(),
        "precondition: the optional cost must raise a prompt on the real \
         ライブ開始時 dispatch"
    );
    game.select_indices(&[0]);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        guard += 1;
        game.select_indices(&[0]);
    }
    (at_live_start, pools(game))
}

/// The mill and the recovery, in that order, from one board.
///
/// The only 『A-RISE』 card among the three is one the mill takes off the deck, and
/// the waitroom starts empty, so the hand can only END UP with a card if step 2 ran
/// after step 1 and read the waitroom step 1 had just filled.
#[test]
fn honoka_recovers_the_arise_member_she_just_milled() {
    let mut game = TestGame::new(load_real_database());
    let arise = game.id(ARISE);
    let not_arise = game.id(NOT_ARISE);
    game.assert_card_identity(arise, ARISE);
    game.assert_card_identity(not_arise, NOT_ARISE);
    // Premises: exactly one card the recovery can take, and two it must refuse.
    assert!(
        rabuka_engine::ability::util::card_matches_group_str(&game.db, arise, Some("A-RISE")),
        "precondition: 優木あんじゅ must be in 『A-RISE』 — the group step 2 filters on"
    );
    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(
            &game.db,
            not_arise,
            Some("A-RISE")
        ),
        "precondition: a 『Printemps』 member must be OUTSIDE 『A-RISE』, or the group \
         filter is never exercised"
    );

    let (start, end) = resolve_with_milled(&mut game, &[ARISE, NOT_ARISE, NOT_ARISE]);

    assert_eq!(
        end.deck,
        start.deck - 3,
        "『デッキの上からカードを3枚控え室に置く』 — exactly three left the deck, so \
         the mill ran"
    );
    assert_eq!(
        end.waitroom,
        start.waitroom + 3,
        "three were banked and ONE came back: the cost card plus the two the group \
         filter refused. A sequential that only milled leaves 4 here, and one that \
         only recovered leaves 1."
    );
    assert_eq!(
        end.hand,
        start.hand,
        "『自分の控え室から『A-RISE』のメンバーカードを1枚手札に加える』 — the cost's -1 \
         and the recovery's +1 cancel, so the hand ends where it started. This is the \
         ONE number that separates the two arms, and it is a net of two opposite \
         moves, so both the cost and the recovery have to be real for it to land here."
    );
}

/// The negative that is not about ORDER: a mill that finds nothing the recovery
/// wants.
///
/// Both steps still run — the cost was paid and three cards were banked — so this
/// isolates the group filter from the sequencing.
#[test]
fn honoka_recovers_nothing_when_no_milled_card_is_arise() {
    let mut game = TestGame::new(load_real_database());
    let not_arise = game.id(NOT_ARISE);
    game.assert_card_identity(not_arise, NOT_ARISE);
    assert!(
        !rabuka_engine::ability::util::card_matches_group_str(
            &game.db,
            not_arise,
            Some("A-RISE")
        ),
        "precondition: every milled card must be outside 『A-RISE』"
    );

    let (start, end) = resolve_with_milled(&mut game, &[NOT_ARISE, NOT_ARISE, NOT_ARISE]);

    assert_eq!(
        end.deck,
        start.deck - 3,
        "the same three left the deck as in the positive arm, so the pair differs \
         only in the filter's verdict"
    );
    assert_eq!(
        end.waitroom,
        start.waitroom + 4,
        "all three stay banked plus the cost card: the mill definitely ran, so this \
         is a filter result and not an ability that did nothing"
    );
    assert_eq!(
        end.hand,
        start.hand - 1,
        "and the hand ends ONE lower — the cost with no recovery to offset it. The \
         positive arm ends unchanged, and that is the whole difference."
    );
}
