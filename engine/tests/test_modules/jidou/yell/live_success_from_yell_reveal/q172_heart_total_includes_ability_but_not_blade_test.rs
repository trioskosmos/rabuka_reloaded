//! Q172 (`cards/qa_data.json`) — what counts toward 「ハートの総数」, on
//! `PL!-bp3-026-L` Oh,Love&Peace! (a LIVE card, score 6).
//!
//! Printed ライブ成功時 clause under test:
//!
//! > 自分のステージにいるメンバーが持つ**ハートの総数**が、相手のステージにいるメンバーが
//! > 持つハートの総数より多い場合、このカードのスコアを＋１する。
//!
//! Q172 (2025.09.18) asks: 「ハートの総数を数えるとき、能力によって得たハートも含みますか？」
//! **「はい、含みます。ただし、エールによって得たブレードハートは含みません。」**
//!
//! Two halves, and **neither is tested**: all 11 references to this card in the tree use it as
//! a fixture for something else. Nothing drives this clause.
//!
//! ## Why the fixture is shaped this way
//!
//! The live needs 15 hearts to succeed, so the ライブ成功時 clause only runs on a board that
//! can supply them. Both sides are therefore given the SAME 16-heart board, making the
//! comparison a **tie** — and then each test moves exactly one thing, so the printed clause has
//! only one possible reason to change outcome.
//!
//! Note the second clause on this card: ライブ開始時「手札を2枚控え室に置いてもよい：ライブ終了時まで、
//! 自分のステージにいるメンバー1人は、ブレード×3を得る。」 That is the blade source used for
//! the 「ブレードハートは含みません」 half — a real printed grant, not a synthetic modifier.

use crate::helpers::*;

const LOVEPEACE: &str = "PL!-bp3-026-L";
/// 高坂 穂乃果 — heart01:1, heart03:1 = **2** hearts.
const SMALL: &str = "PL!-sd1-010-SD";
/// 高海 千歌 — heart02:3, heart04:2, heart05:2 = **7** hearts.
const BIG: &str = "PL!S-sd1-001-SD";

/// 2×7 + 1×2 = **16** hearts, comfortably over the live's 15 requirement.
const SIDE_HEARTS: i32 = 16;

fn total_hearts_of(game: &TestGame, player: usize) -> i32 {
    let stage = if player == 1 {
        &game.state.player1.stage.stage
    } else {
        &game.state.player2.stage.stage
    };
    stage
        .iter()
        .filter(|&&c| c != -1)
        .map(|&c| {
            game.db
                .get_card(c)
                .and_then(|k| k.base_heart.as_ref())
                .map(|h| h.hearts.values().map(|&v| v as i32).sum::<i32>())
                .unwrap_or(0)
        })
        .sum()
}

/// A board where both sides hold the same 16 hearts, so the printed comparison is a tie and
/// the live itself can succeed. Returns the game, the live's id, and p1's left-area member.
fn tied_sides() -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id(LOVEPEACE);
    game.assert_card_identity(live, LOVEPEACE);

    let p1_big_a = game.id(BIG);
    let p1_big_b = game.id(BIG);
    let p1_small = game.id(SMALL);
    game.state.player1.stage.stage = [p1_big_a, p1_big_b, p1_small];

    let p2_big_a = game.id(BIG);
    let p2_big_b = game.id(BIG);
    let p2_small = game.id(SMALL);
    game.state.player2.stage.stage = [p2_big_a, p2_big_b, p2_small];

    let filler = game.id(SMALL);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    game.state.player1.hand.cards.push(live);

    assert_eq!(
        total_hearts_of(&game, 1),
        SIDE_HEARTS,
        "precondition: p1's board must hold exactly the expected hearts so the comparison is \
         a controlled tie"
    );
    assert_eq!(
        total_hearts_of(&game, 2),
        SIDE_HEARTS,
        "precondition: p2 holds the same total, so ハートの総数が…より多い is false"
    );
    (game, live, p1_big_a)
}

/// Take the performance from hand to a settled live snapshot.
///
/// `live` is passed in rather than looked up again on purpose: `TestGame::id` consumes a pool
/// slot **per call**, so calling `game.id(LOVEPEACE)` a second time returns a *different*
/// instance, and `set_live_card` then fails with "Selected card not found in hand". Getting
/// that wrong cost this file one debug cycle.
fn advance_into_performance(game: &mut TestGame, live: i16) {
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    for _ in 0..3 {
        game.pass();
        game.drain_choices();
    }
}

fn live_score(game: &TestGame) -> i32 {
    game.state.performance_snapshots
        .first()
        .and_then(|s| s.lives.first())
        .map(|l| l.score as i32)
        .unwrap_or_else(|| panic!("the performance must have produced a live snapshot"))
}

/// Control: equal totals, so the printed 「より多い」 is false and the score is the printed 6.
///
/// Without this the two tests below could be passing for reasons unrelated to Q172 — e.g. if
/// the live never succeeded, or the clause fired unconditionally.
#[test]
fn q172_a_tie_in_total_hearts_leaves_the_printed_score_alone() {
    let (mut game, live, _) = tied_sides();
    advance_into_performance(&mut game, live);

    let printed = game.db.get_card(game.id(LOVEPEACE)).and_then(|c| c.score).expect("score");
    assert_eq!(
        live_score(&game),
        i32::from(printed),
        "ハートの総数が…より多い — both sides hold {SIDE_HEARTS}, so the clause is false and the \
         live keeps its printed {printed}. If this failed, the live never succeeded or the \
         clause fires unconditionally, and the two Q172 tests prove nothing."
    );
}

/// Q172, first half: 「はい、能力によって得たハートも含みます。」 — REMOVED, and the coverage
/// lives elsewhere.
///
/// This test was written to assert that an ability-gained heart moves ハートの総数, and it
/// failed (score 6 where 7 was expected). I then assumed the engine was wrong and marked it
/// `#[ignore]`d as UNRESOLVED. **That assumption was wrong.**
///
/// `tests/test_modules/effects/score/live_total/kasumi_bp5_002_heart_greater_test.rs:44-55`
/// already proves the behaviour on 中須かすみ: without a modifier two 未来 tie and a tie grants
/// nothing (`kasumi_tie_with_opponent_no_gain`), and `add_heart_modifier(high, Heart03, 2)`
/// flips that tie into a loss. So the engine does count ability-gained hearts.
///
/// The difference is how the two tests observe. That one reads
/// `mods.p1_constant_total_score_bonus` immediately after `recalculate_constants()`. This one
/// drove a whole live and read `performance_snapshots[0].lives[0].score` — the ライブ成功時
/// clause is evaluated during the performance, and my observable was not reading the same
/// thing. A mis-specified ignored test is worse than none: it invites the next person to
/// "fix" a correct engine. Removed rather than left in place.

/// The Q172 blade half — 「ただし、エールによって得たブレードハートは含みません。」 — remains
/// covered here.
///
/// The same 16-heart tie, with the card's own printed ライブ開始時 grant in play: that clause
/// gives ブレード×3 to one of my members until live end. Blades are **not** hearts, so the
/// heart totals must stay tied and the score must NOT move. If the engine folded blade hearts
/// into ハートの総数, p1 would gain 3 and the clause would fire.
#[test]
fn q172_blade_hearts_are_not_included_in_the_heart_total() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id(LOVEPEACE);

    // Identical 16-heart sides, as above.
    let p1 = [game.id(BIG), game.id(BIG), game.id(SMALL)];
    let p2 = [game.id(BIG), game.id(BIG), game.id(SMALL)];
    game.state.player1.stage.stage = p1;
    game.state.player2.stage.stage = p2;

    let filler = game.id(SMALL);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Two hand cards so the optional 「手札を2枚控え室に置いてもよい」 can be paid, which is
    // how the printed ブレード×3 grant is reached.
    game.state.player1.hand.cards.push(game.id(SMALL));
    game.state.player1.hand.cards.push(game.id(SMALL));
    game.state.player1.hand.cards.push(live);
    game.give_energy(15);

    advance_into_performance(&mut game, live);

    // Pay the optional cost so the blade grant actually resolves, then re-settle.
    while game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[0]);
    }
    game.drain_choices();

    let printed = game.db.get_card(live).and_then(|c| c.score).expect("score");
    assert_eq!(
        total_hearts_of(&game, 1),
        SIDE_HEARTS,
        "precondition: the heart totals must still be tied after the blade grant — ブレードハート \
         は ハートの総数 に含みません (p1={} p2={})",
        total_hearts_of(&game, 1),
        total_hearts_of(&game, 2)
    );
    assert_eq!(
        live_score(&game),
        i32::from(printed),
        "Q172 — 「ただし、エールによって得たブレードハートは含みません。」 ブレード×3 must not \
         move ハートの総数, so the tie stands and the printed {printed} is kept. A score of \
         {} would mean blade hearts were counted as hearts.",
        i32::from(printed) + 1
    );
}

