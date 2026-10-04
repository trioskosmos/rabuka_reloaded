//! Q213 — ハナムスビ `PL!HS-bp5-019-L`.
//!
//! **Card ability** (`cards/cards.json`, `PL!HS-bp5-019-L`):
//! 「{{live_start.png|ライブ開始時}}自分のライブカード置き場にあるこのカード以外の『蓮ノ空』の
//! カード1枚につき、このカードの必要ハートをheart04×2減らす。」
//!
//! **Q213 ruling** (FAQ, 2026.02.28), same card:
//! 「いいえ、{{live_start.png|ライブ開始時}}能力が発動する前にメンバーカードは控え室に移動します。
//! そのため、メンバーカードでは減りません。」
//!
//! # The ruling in one sentence
//!
//! A 蓮ノ空 card counts toward the reduction **only if it is still a live card when
//! the ライブ開始時 window opens**. A 蓮ノ空 *member* forced into the live slot is in
//! the 控え室 by then, so it contributes **0**.
//!
//! # Why the previous version of this file asserted nothing
//!
//! It ended on `assert!(after_waitroom >= before_waitroom)`, which the zero case
//! satisfies — it passes identically whether the member was evicted or never put in
//! the slot — and it never read `get_need_heart_modifier` at all. The ruling it was
//! named for was therefore untested. The sibling file
//! `effects/cost_mod/required_hearts/hanamusubi_required_hearts_exclude_self_test.rs`
//! has nine substantive tests, but every one of them uses another 蓮ノ空 **live**
//! card as the "other card". Not one staged a member, so between them the two files
//! covered both halves of Q213 and observed neither.
//!
//! # Which chain these tests pin
//!
//! * `turn/phases.rs:240` — `check_timing` runs with `face_up == false`.
//! * `turn/phases.rs:245-246` — `trigger_live_start_abilities`, the Q213 window.
//! * `turn/phases.rs:343` — `face_up = true`, first statement of
//!   `execute_performance_phase`.
//! * `turn/phases.rs:349` — `check_timing` again, now evicting non-live cards.
//! * `turn/actions/mod.rs:1732-1736` — `check_invalid_live_cards` returns early
//!   unless that seat's `face_up` is set.
//! * `ability/condition/card.rs:4518-4519` — the `LiveCardZone` arm of the group
//!   count, which has no `is_live()` filter.
//!
//! `execute_performance_phase` runs at `phases.rs:258`, i.e. **after** the scan at
//! `:245-246`. So at the moment the ability resolves, the member is still in
//! `live_card_zone.cards`, and `check_invalid_live_cards` has not run for this seat.
//!
//! These tests therefore assert the CARD TEXT, and they fail today. That is the
//! point: Q213 is the ruling that exposed the largest cross-engine defect in this
//! project, and before this file nothing could detect a regression. The fix is an
//! implementation change (`face_up` must be set before `:245`, or the counting arm
//! must filter on `is_live()`); it is deliberately NOT made here.

use crate::helpers::*;
use rabuka_engine::card::{CardType, HeartColor};

/// ハナムスビ. Base need is heart04 = 9.
const HANAMUSUBI: &str = "PL!HS-bp5-019-L";
/// 蓮ノ空 live card — 「このカード以外」の減算対象になる.
const RENOSORA_LIVE: &str = "PL!HS-bp5-017-L";
/// 蓮ノ空 member (日野下花帆). Matches the 『蓮ノ空』group filter, but is not a live
/// card, so Q213 says it must not reduce anything.
const RENOSORA_MEMBER: &str = "PL!HS-sd1-001-SD";

/// Stage filler, both decks, and the cards under test into P1's hand.
fn fixture() -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let f2 = game.new_id("PL!-sd1-010-SD");
    let f3 = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, f2, f3];
    game.give_energy(10);

    let hanamusubi = game.id(HANAMUSUBI);
    game.state.player1.hand.cards.push(hanamusubi);

    // Fixture preconditions, asserted rather than assumed — a renamed or retyped
    // card would otherwise turn the Q213 assertion into a test of nothing.
    assert_eq!(
        game.db.get_card(hanamusubi).map(|c| c.card_type.clone()),
        Some(CardType::Live),
        "fixture: {HANAMUSUBI} must be a ライブカード"
    );
    assert_eq!(
        game.db.get_card(game.id(RENOSORA_LIVE)).map(|c| c.card_type.clone()),
        Some(CardType::Live),
        "fixture: {RENOSORA_LIVE} must be a ライブカード"
    );
    assert_eq!(
        game.db
            .get_card(game.id(RENOSORA_MEMBER))
            .map(|c| c.card_type.clone()),
        Some(CardType::Member),
        "fixture: {RENOSORA_MEMBER} must be a メンバー — it is the whole point"
    );

    (game, hanamusubi)
}

/// Put `card` into the live slot, then walk into the ライブ開始時 window and read
/// the heart04 modifier ハナムスビ's ability produced.
///
/// Returns `(reduction, second_card_id)`. The id is handed back because `game.id`
/// allocates a FRESH instance per call: the test used to call `game.id(RENOSORA_MEMBER)`
/// again to look for the member, got a different id, and reported the card as missing
/// from the waitroom while it was sitting there under the id this function had placed.
fn reduction_with_second_live_card(
    game: &mut TestGame,
    hanamusubi: i16,
    second: &str,
) -> (i16, i16) {
    let second_id = game.id(second);
    game.state.player1.hand.cards.push(second_id);

    // Step BY NAME to the live-card phase, then set. `advance_to_phase` is the
    // helper the codebase mandates over `for _ in 0..5 { pass() }`: a blind count
    // encodes the phase list in the test, and Q213 *is* a phase-ordering defect,
    // so a phase that gains or loses a step would move this test with no failure.
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker);
    game.set_live_card(hanamusubi);
    // A member is not a legal live card, but `set_live_card` only moves the card
    // out of hand — validation happens later, in `check_invalid_live_cards`. That
    // is exactly the window Q213 is about, so the illegal state must be created
    // here rather than asserted away.
    game.set_live_card(second_id);

    // ライブ開始時 fires on the step into the performance phase. Arriving hands any
    // prompt to this caller rather than draining it, which is what we want.
    game.advance_to_phase(rabuka_engine::game_state::Phase::FirstAttackerPerformance);
    // Sibling abilities on the same cards can raise their own optional prompts;
    // drain those so hanamusubi's cost-free effect has run.
    game.drain_auto_ability_choices();

    (
        game.state
            .mods
            .get_need_heart_modifier(hanamusubi, HeartColor::Heart04),
        second_id,
    )
}

/// **The control.** A 蓮ノ空 *live* card in the live slot reduces heart04 by 2.
/// This is what makes the member case below a measurement rather than an
/// assumption: if this ever reads 0, the fixture is broken and the member
/// assertion below would pass for the wrong reason.
#[test]
fn a_renosora_live_card_in_the_slot_does_reduce_heart04() {
    let (mut game, hanamusubi) = fixture();
    let (reduction, _second_id) =
        reduction_with_second_live_card(&mut game, hanamusubi, RENOSORA_LIVE);
    assert_eq!(
        reduction, -2,
        "control: 『蓮ノ空』のライブカード1枚につき heart04×2 — expected -2, got {reduction}"
    );
}

/// **Q213.** A 蓮ノ空 *member* in the live slot reduces nothing, because it is in
/// the 控え室 before the ライブ開始時 ability resolves.
///
/// Fails today: `turn/actions/mod.rs:1732-1736` gates the eviction on `face_up`,
/// which `turn/phases.rs:343` sets only after `trigger_live_start_abilities` has
/// already run at `:245-246`, and the counting arm at
/// `ability/condition/card.rs:4518-4519` applies no `is_live()` filter.
#[test]
fn q213_a_renosora_member_in_the_slot_reduces_nothing() {
    let (mut game, hanamusubi) = fixture();
    let (reduction, _second_id) =
        reduction_with_second_live_card(&mut game, hanamusubi, RENOSORA_MEMBER);
    assert_eq!(
        reduction, 0,
        "Q213: 「ライブ開始時能力が発動する前にメンバーカードは控え室に移動します。\
         そのため、メンバーカードでは減りません。」 — a 蓮ノ空 member must reduce 0, \
         got {reduction}"
    );
}

/// **Q213, the second half.** The member must actually be *in the waitroom* by the
/// time the ability resolves — not merely absent from the count. An implementation
/// that fixed the reduction by filtering `is_live()` at the counting site while
/// leaving the member sitting in the live slot would pass the test above and fail
/// this one, and would still be wrong: the ruling says the card MOVES.
#[test]
fn q213_the_renosora_member_is_in_the_waitroom_when_live_start_resolves() {
    let (mut game, hanamusubi) = fixture();
    // The id the helper PLACED, not a fresh `game.id` — see the helper's doc comment.
    let (_reduction, member) =
        reduction_with_second_live_card(&mut game, hanamusubi, RENOSORA_MEMBER);

    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "Q213 moves the member to the 控え室 before ライブ開始時 resolves; \
         waitroom={:?} live={:?}",
        game.state.player1.waitroom.cards,
        game.state.player1.live_card_zone.cards
    );
    assert!(
        !game.state.player1.live_card_zone.cards.contains(&member),
        "and it is no longer in the live slot: live={:?}",
        game.state.player1.live_card_zone.cards
    );
    assert!(
        game.state.player1.live_card_zone.cards.contains(&hanamusubi),
        "the live card itself must stay in the slot: live={:?}",
        game.state.player1.live_card_zone.cards
    );
}