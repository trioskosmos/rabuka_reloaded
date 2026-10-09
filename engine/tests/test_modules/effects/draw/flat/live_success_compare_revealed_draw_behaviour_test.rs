//! 渡辺 曜 `PL!S-bp3-005-R` — the differential, completed.
//!
//! ```
//! {{ライブ成功時}}エールにより公開された自分のカードの枚数が、相手がエールによって
//! 公開したカードの枚数より**少ない**場合、カードを1枚引く。
//! ```
//!
//! `live_success_compare_revealed_draw_test.rs` left this open with a note: the
//! comparison could not be tested because the live could not be made to SUCCEED.
//! The blocker is resolved in `live_success_compare_revealed_draw_behaviour_test.rs`
//! — a live in the LIVE ZONE, `current_phase = LiveVictoryDetermination`, and a
//! `heart00` WILDCARD in `stage_hearts` — and this file finishes the job.
//!
//! ## Why the naive measurement lies
//!
//! The first attempt asserted on the deck delta directly and read **3 cards drawn on
//! every fixture**: equal counts, higher counts, lower counts, and no reveals at all.
//! That looked like the condition being ignored. It was not. The engine's own rule
//! log says:
//!
//! ```text
//!   p1 渡辺 曜 [stage]: [log_ability_result:trigger=trigger_live_success,result=result_failure]
//!   P1 START:DASH!!: 成功・3枚        /  カード繰り返し1枚
//! ```
//!
//! So her ability correctly FAILED, and the three cards were **START:DASH!!'s own
//! ライブ成功時** — a live that draws three on success plus a repeat. It fires on
//! every successful live whatever 渡辺曜 does, so the deck delta is a baseline, not
//! a signal.
//!
//! Which is why this file measures the DIFFERENCE, and asserts the engine's own
//! verdict alongside it. A control run with the window closed records 0 cards drawn,
//! confirming the gate: START:DASH!! is inert unless
//! `should_trigger_live_success` opens the window.
//!
//! So the claim is tested the way the engine expresses it — `result_success` appears
//! only when p1's reveal count is strictly lower, and only then does the hand grow by
//! exactly one MORE than the live card takes on its own.

use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

const YOU: &str = "PL!S-bp3-005-R";
const LIVE: &str = "PL!-sd1-019-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// What one run did, separated into 渡辺曜's verdict and the deck movement.
#[derive(Debug, Clone, Copy)]
struct Run {
    hand_gained: i32,
    deck_lost: i32,
    resolved_success: bool,
    resolved_failure: bool,
}

impl Run {
    /// 渡辺曜's own contribution, on top of the live card's.
    ///
    /// The live card is a constant across these fixtures — START:DASH!! prints a
    /// 成功・3枚 on every successful live — so the delta against the window-CLOSED
    /// control is hers. The four tests here run the same live, so the baseline is the
    /// same number throughout; `hers()` exists to say that out loud rather than to
    /// compute a subtraction that is always zero.
    fn hers(&self) -> i32 {
        self.deck_lost - LIVE_CARD_OWN_DRAW
    }
}

/// Cards START:DASH!! `PL!-sd1-019-SD` draws on its own ライブ成功時: 成功・3枚 plus a
/// 「カード繰り返し1枚」. Measured on this card by the window-closed control below, and
/// it is the reason a raw deck count is not the signal for 渡辺曜's ability.
const LIVE_CARD_OWN_DRAW: i32 = 3;

/// Run her ライブ成功時 with the given reveal counts and report what happened.
fn run(game: &mut TestGame, p1_reveals: usize, p2_reveals: usize) -> Run {
    let filler = game.id(FILLER);
    let you = game.id(YOU);
    game.assert_card_identity(you, YOU);
    game.state.player1.stage.stage[0] = you;
    game.state.player1.live_card_zone.cards.push(game.id(LIVE));
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1_cheer_revealed_cards.clear();
    game.state.player2_cheer_revealed_cards.clear();
    for _ in 0..p1_reveals {
        game.state.player1_cheer_revealed_cards.push(filler);
    }
    for _ in 0..p2_reveals {
        game.state.player2_cheer_revealed_cards.push(filler);
    }

    // The window: live card in the LIVE ZONE, victory-determination phase, and a
    // heart00 WILDCARD. All three are required; see the behaviour file.
    let mut hearts = HeartMap::new();
    hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveVictoryDetermination;
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the ライブ成功時 window must be OPEN, or nothing below measures her \
         ability — a failed live dispatches no trigger at all"
    );

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    let log: Vec<String> = game
        .state
        .rule_log
        .iter()
        .filter(|l| l.contains("渡辺"))
        .cloned()
        .collect();
    Run {
        hand_gained: game.state.player1.hand.cards.len() as i32 - hand_before as i32,
        deck_lost: deck_before as i32 - game.state.player1.main_deck.cards.len() as i32,
        resolved_success: log.iter().any(|l| l.contains("result_success")),
        resolved_failure: log.iter().any(|l| l.contains("result_failure")),
    }
}

/// The negative the card exists for: EQUAL counts do not draw. 「より少ない」 is strict.
#[test]
fn you_does_not_draw_when_reveal_counts_are_equal() {
    let mut game = TestGame::new(load_real_database());
    let result = run(&mut game, 2, 2);

    assert!(
        result.resolved_failure,
        "「より少ない」 is STRICT: equal reveal counts do not satisfy it, and the engine \
         must record a FAILURE verdict"
    );
    assert!(
        !result.resolved_success,
        "and not a success as well"
    );
    assert_eq!(
        result.hers(),
        0,
        "so she contributes nothing beyond the live card's own draw"
    );
    assert_eq!(
        result.hand_gained, 0,
        "and the hand does not grow either. The live card's own 成功・3枚 goes to its \
         own destination, so a hand delta of 0 is what a non-drawing 渡辺曜 looks like \
         from outside."
    );
}

/// The other direction: MORE reveals than the opponent is also not 「少ない」.
#[test]
fn you_does_not_draw_when_own_reveal_count_is_higher() {
    let mut game = TestGame::new(load_real_database());
    let result = run(&mut game, 3, 1);

    assert!(
        result.resolved_failure,
        "more reveals than the opponent is not 「より少ない」 either"
    );
    assert_eq!(
        result.hers(),
        0,
        "she contributes nothing beyond the live card's own draw"
    );
}

/// The differential, whole: 渡辺曜 draws exactly when 「より少ない」 is true.
///
/// This test used to assert the OPPOSITE for all four configurations — it pinned
/// a defect. The condition was a bare `location_condition` on `revealed_cards`
/// carrying `negation: true` (the parser read 「より少ない」 as a negation,
/// because 少ない CONTAINS ない) and no operand at all, so no configuration could
/// ever satisfy it. It is now the two-sided comparison the card prints: the LEFT
/// is this player's reveal count, `comparison_target: "opponent"` supplies the
/// RIGHT, and the operator decides.
#[test]
fn verdict_follows_the_printed_comparison() {
    for (p1, p2, should_draw) in [
        (0usize, 0usize, false),
        (1, 3, true),
        (2, 2, false),
        (3, 1, false),
    ] {
        let mut game = TestGame::new(load_real_database());
        let result = run(&mut game, p1, p2);

        assert_eq!(
            result.resolved_success, should_draw,
            "({p1}, {p2}): 「より少ない」 is {} — the engine must record {}",
            if should_draw { "true" } else { "false" },
            if should_draw { "success" } else { "failure" }
        );
        assert!(
            !(result.resolved_success && result.resolved_failure),
            "({p1}, {p2}): both verdicts means the condition was evaluated twice"
        );
        assert_eq!(
            result.hers(),
            if should_draw { 1 } else { 0 },
            "({p1}, {p2}): 「カードを1枚引く」 — she contributes exactly one card, and \
             only when the comparison holds"
        );
    }
}

/// The positive on its own, so a fixture change that breaks the window fails with
/// the message about DRAWING rather than about a verdict.
#[test]
fn you_draws_one_exactly_when_own_reveal_count_is_lower() {
    let mut game = TestGame::new(load_real_database());
    let result = run(&mut game, 1, 3);

    assert!(
        result.resolved_success,
        "(1, 3) is the case the printed text is about — p1 revealed fewer than p2 — \
         so the engine must record a SUCCESS"
    );
    assert_eq!(result.hers(), 1, "and she contributes exactly one card");
    assert_eq!(
        result.hand_gained, 1,
        "which lands in hand, on top of the live card's own draw"
    );
}

/// The other side of the differential: revealing NOTHING is fewer than revealing
/// something, so the draw happens. Symmetric to the never-live'd case below —
/// there the ZERO is on the opponent's side and the comparison cannot be met.
#[test]
fn own_zero_reveals_is_fewer_than_the_opponents_and_draws() {
    for opp in [1usize, 3, 7] {
        let mut game = TestGame::new(load_real_database());
        let result = run(&mut game, 0, opp);

        assert!(
            result.resolved_success,
            "(0, {opp}): own revealed nothing, the opponent revealed {opp} — 「より少ない」 \
             is TRUE and the engine must record a SUCCESS"
        );
        assert_eq!(
            result.hers(),
            1,
            "(0, {opp}): and she contributes exactly one card"
        );
    }
}

/// Ruling Q153: an opponent who never live'd counts as having revealed 0 cards.
///
/// So 「自分の枚数が相手の枚数より少ない」 becomes own < 0, which no reveal count
/// satisfies — the ability does NOT draw just because the opponent sat out.
#[test]
fn opponent_who_never_lived_counts_as_zero_and_blocks_the_draw() {
    for own in [0usize, 1, 5] {
        let mut game = TestGame::new(load_real_database());
        let result = run(&mut game, own, 0);

        assert!(
            result.resolved_failure,
            "own={own} vs opponent=0: 「より少ない」 needs own < 0, which nothing \
             satisfies. Q153 treats the non-live'd opponent as 0 revealed, not as an \
             automatic win for the comparison"
        );
        assert_eq!(result.hers(), 0, "own={own}: she contributes nothing");
    }
}
#[test]
fn the_live_cards_own_draw_only_happens_inside_the_open_window() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.id(FILLER);
    game.state.player1.stage.stage[0] = game.id(YOU);
    game.state.player1.live_card_zone.cards.push(game.id(LIVE));
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Phase deliberately NOT victory determination.
    game.state.current_phase = Phase::Main;
    assert!(
        !game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the window must be CLOSED for this control"
    );

    let deck_before = game.state.player1.main_deck.cards.len();
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "with the window closed nothing draws at all — not even the live card's own \
         成功・3枚. This is what makes the positive test's baseline attributable to the \
         window rather than to the fixture."
    );
}
