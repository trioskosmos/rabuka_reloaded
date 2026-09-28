//! PL!HS-pb1-003-R 大沢瑠璃乃 — the ターン2回 second-firing regression guard.
//!
//!   * ab#1 自動 ターン2回 「自分の手札からカードが1枚以上控え室に置かれるたび、
//!     ライブ終了時まで、heart01 と ブレードを得る。」
//!     (condition source: `preceding_moved`, location `discard`)
//!
//! ## The gap this file closes
//!
//! `jidou/discard_watch/on_hand_to_discard_each_time_gain_heart01_and_blade_test.rs`
//! has a test named `rurino_use_limit_blocks_second_same_turn` that sets up two
//! qualifying discard events — and then deliberately asserts NOTHING about the
//! second one:
//!
//! ```ignore
//! // heart01 may be 1 or 2 depending on post-resolve re-enqueue.
//! // The important thing is no crash.
//! ```
//!
//! The card is ターン2回, so exactly 2 grants is the only correct reading, and
//! "may be 1 or 2" hides the regression in plain sight. That is precisely the
//! shape of the re-scan-guard bug (Bug B, `docs/JIDOU_COMBINATION_WORK.md` §6):
//! the guard (`just_completed_ability_key`) leaked into the next scan and, since
//! ブレード+1 is a non-movement effect (`just_completed_moved` empty), vetoed the
//! second qualifying event outright. 瑠璃乃 could never use her second
//! allowance in a real game, and the existing test would not have noticed — it
//! accepts both answers.
//!
//! ## The three link observables
//!
//! 1. **Two separate qualifying events in one turn grant twice** — the
//!    ターン2回 allowance, and the Bug B regression guard.
//! 2. **A batch of N cards is ONE event** (Q241): the allowance is per *event*,
//!    not per card, so a 3-card batch must leave the second allowance intact.
//! 3. **A third event is refused** — the allowance is a ceiling, not a minimum.
//!
//! Note on the trigger mechanism: this jidou reads
//! `source: "preceding_moved"`, i.e. it watches `recently_moved_cards` rather
//! than the hand/waitroom zones directly. The tests drive it with
//! `set_recently_moved_cards` — the same public entry the existing discard-watcher
//! file uses — so the condition, the enqueue, and the ターン2回 accounting are all
//! the real engine's.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const RURINO: &str = "PL!HS-pb1-003-R"; // 大沢瑠璃乃
const DISCARD_FODDER: &str = "PL!-sd1-010-SD";

fn setup() -> (TestGame, i16) {
    let mut game = TestGame::new(load_real_database());
    let rurino = game.id(RURINO);
    game.assert_card_identity(rurino, RURINO);
    game.assert_card_cost(rurino, 15);
    game.state.player1.stage.stage = [-1, rurino, -1];
    (game, rurino)
}

fn blade_mod(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_blade_modifier(cid)
}

fn heart01_mod(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_heart_modifier(cid, HeartColor::Heart01)
}

/// Run the real trigger scan + resolution, then drain any prompts.
fn trigger_auto(game: &mut TestGame) {
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        game.select_indices(&[0]);
    }
}

/// One qualifying discard event: `n` cards move to 控え室 together.
fn discard_event(game: &mut TestGame, n: usize) {
    let ids: Vec<i16> = (0..n).map(|_| game.id(DISCARD_FODDER)).collect();
    game.state.set_recently_moved_cards(ids);
    trigger_auto(game);
}

// ====================================================================
// 1. THE SECOND FIRING — the ターン2回 allowance, and the Bug B guard
// ====================================================================

/// Two SEPARATE qualifying events in one turn each fire ab#1: 2 ブレade + 2 heart01.
///
/// This is the assertion `rurino_use_limit_blocks_second_same_turn` declined to
/// make. The ceiling is 2, so exactly 2 is the printed maximum and the
/// assertion is tight in both directions.
#[test]
fn hs_pb1_003_two_separate_discard_events_in_one_turn_grant_twice() {
    let (mut game, rurino) = setup();
    assert_eq!(
        (heart01_mod(&game, rurino), blade_mod(&game, rurino)),
        (0, 0),
        "precondition: no qualifying event yet, no grant"
    );

    discard_event(&mut game, 1);
    assert_eq!(
        blade_mod(&game, rurino),
        1,
        "event 1: a hand→控え室 placement fires ab#1 -> ブレード+1 (ターン2回, 1 of 2 used)"
    );

    discard_event(&mut game, 1);
    assert_eq!(
        blade_mod(&game, rurino),
        2,
        "event 2: the SECOND qualifying event in the same turn MUST fire again. \
         This is the ターン2回 allowance and the Bug B regression guard: a leaked \
         re-scan key vetoed the second event, so this read 1 before the fix. The \
         existing `rurino_use_limit_blocks_second_same_turn` test accepted '1 or 2' \
         here, so nothing caught it."
    );
    assert_eq!(
        heart01_mod(&game, rurino),
        2,
        "both firings also grant heart01 — the jidou is a paired heart01+ブレード grant"
    );
}

// ====================================================================
// 2. A BATCH IS ONE EVENT (Q241) — per event, not per card
// ====================================================================

/// Three cards in ONE event grant ONCE, leaving the second allowance available.
///
/// If a batch of 3 consumed three allowances, the second *event* would be wrongly
/// refused and the total would read 1 — a different failure that lands on the
/// same visible number as "the jidou never fired", which is why both halves are
/// asserted.
#[test]
fn hs_pb1_003_batch_of_three_is_one_event_and_keeps_the_second_allowance() {
    let (mut game, rurino) = setup();

    discard_event(&mut game, 3);
    assert_eq!(
        blade_mod(&game, rurino),
        1,
        "a batch of 3 fires ab#1 ONCE (Q241). It must not consume one allowance \
         per card — that would exhaust ターン2回 on a single event."
    );

    discard_event(&mut game, 1);
    assert_eq!(
        blade_mod(&game, rurino),
        2,
        "the second event is still allowed after a 3-card batch, proving the batch \
         consumed exactly one allowance"
    );
}

// ====================================================================
// 3. THE CEILING — a third event is refused
// ====================================================================

/// A THIRD qualifying event in one turn is refused: ターン2回 is a ceiling.
///
/// Without this, an engine that simply dropped the per-turn limit would pass
/// tests 1 and 2 (both ≤ 2 grants) while over-firing here — the mirror of Bug B,
/// in the permissive direction.
#[test]
fn hs_pb1_003_turn2_refuses_a_third_event_in_one_turn() {
    let (mut game, rurino) = setup();

    discard_event(&mut game, 1);
    discard_event(&mut game, 1);
    assert_eq!(
        blade_mod(&game, rurino),
        2,
        "precondition: both allowances used"
    );

    discard_event(&mut game, 1);
    assert_eq!(
        blade_mod(&game, rurino),
        2,
        "ターン2回 is a CEILING: a third qualifying event in the same turn must be \
         refused. An engine that ignored the limit would read 3 here while still \
         passing the two-firing assertions above."
    );
    assert_eq!(
        heart01_mod(&game, rurino),
        2,
        "the refusal covers the whole paired grant (heart01 + ブレード), not one half"
    );
}
