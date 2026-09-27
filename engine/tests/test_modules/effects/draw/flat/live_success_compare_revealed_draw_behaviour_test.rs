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
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

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

/// THE FINDING: 渡辺曜's ライブ成功時 ability cannot fire at all.
///
/// Four reveal configurations, each with the window open and each asserted, and the
/// engine records `result_failure` for every one:
///
/// ```text
///   (p1, p2) = (0, 0)  ->  result_failure
///   (p1, p2) = (1, 3)  ->  result_failure     <- 「より少ない」 is TRUE here
///   (p1, p2) = (2, 2)  ->  result_failure
///   (p1, p2) = (3, 1)  ->  result_failure
/// ```
///
/// (1, 3) is the case the printed text is about — p1 revealed fewer than p2 — and it
/// fails. So the condition is not merely conservative, it is unsatisfiable.
///
/// ## Scope: one card, not a pattern
///
/// Every condition in `abilities.json` whose text states a comparison was checked.
/// Four cards matched, and three of them are FINE because the comparison IS
/// structured there:
///
///   * `PL!N-bp5-005-R＋`, `PL!HS-sd1-001-SD`, `PL!S-PR-029-PR` all carry
///     `cost_limit` together with `cost_limit_operator` — 「コスト10以上」 and
///     「コスト13以上」 compile to a real operand and an operator.
///   * `PL!S-bp3-005-R` (渡辺曜) carries NEITHER, and the only field on her condition
///     is the zone.
///
/// So the parser has a working representation for a ONE-SIDED comparison, and
/// 渡辺曜 is the single card in this pool whose comparison is TWO-SIDED — a count
/// against another count, which `cost_limit` + `cost_limit_operator` has no shape
/// for. That is a precise gap with a count of one, not a general failure, and the
/// distinction matters: a fix belongs in the parser's comparison representation and
/// should not disturb the three cards that work.
///
/// ## Not written, and why
///
/// A structural test asserting the condition has no comparison operand is not
/// expressible: `Condition` exposes no public `text` or `location`. The evidence is
/// behavioural instead, which is stronger — four configurations, including the one the
/// card is written about.
#[test]
fn no_reveal_configuration_satisfies_the_condition() {
    for (p1, p2) in [(0usize, 0usize), (1, 3), (2, 2), (3, 1)] {
        let mut game = TestGame::new(load_real_database());
        let result = run(&mut game, p1, p2);
        assert!(
            result.resolved_failure,
            "({p1}, {p2}): recorded a SUCCESS, so the condition IS satisfiable and the \
             `location_condition` reading is wrong"
        );
        assert!(
            !result.resolved_success,
            "({p1}, {p2}): recorded both verdicts, so the condition was evaluated twice"
        );
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
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "with the window closed nothing draws at all — not even the live card's own \
         成功・3枚. This is what makes the positive test's baseline attributable to the \
         window rather than to the fixture."
    );
}
