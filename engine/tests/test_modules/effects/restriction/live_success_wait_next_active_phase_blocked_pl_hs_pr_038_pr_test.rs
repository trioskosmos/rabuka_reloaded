//! `PL!HS-PR-038-PR` ab#1 — ライブ成功時, a sequential effect whose second step is
//! a DELAYED restriction:
//!
//! ```text
//! {{ライブ成功時}}相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
//! このメンバーは次のターンのアクティブフェイズにアクティブにならない。
//! ```
//!
//! The first step is ordinary `change_state` (wait). The second is the parser's
//! 「次のターン」 marker: `restriction_type: cannot_activate` + `delayed: true`,
//! which `effects/restriction.rs` routes into `add_delayed_cannot_active(cid, 1)`
//! keyed on the members the effect just waited (`changed_state_members`). The
//! flag is consulted by `turn/phases.rs` when the member's OWN controller
//! reaches their Active phase (Q280: the tick is owner-scoped, so an
//! intervening opponent active phase does not consume it) and is a one-shot —
//! the member wakes again at the active phase after the one it missed.
//!
//! The block runs on the pass OUT of `Phase::Active`: entering Active only
//! arrives at the window, `pass()` executes the wake block (build `to_activate`,
//! drop flagged members, tick the flag, move to Energy). So each stage below
//! asserts BEFORE the pass to see the flag, and AFTER it to see the effect.

use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::{Phase, TurnPhase};
use rabuka_engine::turn::TurnEngine;

const SUBJECT: &str = "PL!HS-PR-038-PR";
/// Cost 4 — inside 「コスト4以下」, and abilityless so nothing else acts.
const OPPONENT: &str = "PL!-sd1-010-SD";
/// Abilityless live, only here to open the ライブ成功時 window.
const LIVE: &str = "PL!-sd1-019-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// Open the ライブ成功時 window for p1 and dispatch its stage members' triggers,
/// then answer whatever the effect raises. Fixture mirrors
/// `live_success_compare_revealed_draw_behaviour_test`: live in the live zone,
/// `LiveVictoryDetermination`, and a `heart00` wildcard stage score.
///
/// Returns `(subject, opponent_member)`.
fn fire_live_success(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let subject = game.id(SUBJECT);
    let opponent_member = game.id(OPPONENT);
    game.state.player1.stage.stage = [subject, -1, -1];
    game.state.player2.stage.stage = [-1, opponent_member, -1];
    game.state.player1.live_card_zone.cards.push(game.id(LIVE));
    let mut hearts = HeartMap::new();
    hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveVictoryDetermination;
    // Both halves of the invariant: LVD is a live phase, so the turn phase must
    // say Live — pass() debug-asserts phase_invariant() before advancing.
    game.state.current_turn_phase = TurnPhase::Live;
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the ライブ成功時 window must be OPEN, or nothing below measures \
         this ability"
    );
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    game.drain_choices();
    // The live card was only a window prop; drop it so a later real live in the
    // step-through below cannot re-fire this very trigger and reset the flag.
    game.state.player1.live_card_zone.cards.clear();
    (subject, opponent_member)
}

/// Step until `pid` is the active player IN the Active phase — their own
/// 「アクティブフェイズ」 window, before `pass()` runs the wake block (see the
/// module docs). Drains prompts raised by arriving, then hands control back.
fn step_to_active_for(game: &mut TestGame, pid: &rabuka_engine::player::PlayerId) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(
            guard < 40,
            "never reached an Active phase for {} (stuck at {:?}, active {:?})",
            pid,
            game.state.current_phase,
            game.state.active_player().id
        );
        if game.state.current_phase == Phase::Active
            && game.state.active_player().id == *pid
        {
            game.drain_choices();
            return;
        }
        game.pass();
        game.drain_choices();
    }
}

#[test]
fn live_success_waits_one_opp_member_and_flags_it_for_next_active_phase() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (subject, opponent_member) = fire_live_success(&mut game);

    assert_eq!(
        game.state.mods.get_orientation_modifier(opponent_member),
        Some("wait"),
        "step 1: 相手のステージにいるコスト4以下のメンバー1人をウェイトにする"
    );
    assert!(
        game.state.mods.is_delayed_cannot_active(opponent_member),
        "step 2: the member the effect just waited is the one flagged — \
         add_delayed_cannot_active keys on changed_state_members"
    );
    assert!(
        !game.state.mods.is_delayed_cannot_active(subject),
        "the ability's own member is not restricted by its own effect"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(subject),
        None,
        "the wait targets the opponent's stage only — the owner never writes an \
         orientation modifier"
    );
}

#[test]
fn waited_opponent_skips_its_next_active_phase_then_wakes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_subject, opponent_member) = fire_live_success(&mut game);
    assert!(game.state.mods.is_delayed_cannot_active(opponent_member));

    let p2 = game.state.player2.id.clone();

    // The member's FIRST active phase after the flag: blocked.
    step_to_active_for(&mut game, &p2);
    assert!(
        game.state.mods.is_delayed_cannot_active(opponent_member),
        "the flag survives every intervening phase — Q280 ticks it only for its \
         own owner's active phase"
    );
    game.pass();
    game.drain_choices();
    assert_eq!(
        game.state.mods.get_orientation_modifier(opponent_member),
        Some("wait"),
        "次のターンのアクティブフェイズにアクティブにならない — the member must NOT \
         wake at the active phase the flag names"
    );
    assert!(
        !game.state.mods.is_delayed_cannot_active(opponent_member),
        "the one-shot flag is consumed by the phase it blocked"
    );

    // The active phase AFTER the missed one: restriction over, member wakes.
    step_to_active_for(&mut game, &p2);
    game.pass();
    game.drain_choices();
    assert_eq!(
        game.state.mods.get_orientation_modifier(opponent_member),
        Some("active"),
        "the restriction is one-shot — the member wakes at the NEXT active phase"
    );
    assert!(!game.state.mods.is_delayed_cannot_active(opponent_member));
}
