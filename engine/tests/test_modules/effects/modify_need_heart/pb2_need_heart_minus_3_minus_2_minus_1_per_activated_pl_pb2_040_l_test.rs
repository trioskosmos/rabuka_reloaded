//! 僕たちはひとつの光 `PL!-pb2-040-L` (ライブ開始時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ開始時 — 自分のステージに、**このターン中に自分の『Printemps』のカードの効果
//!   によってウェイト状態からアクティブにしていたメンバーが1人以上いる場合**、このカードの
//!   必要ハートをheart00を3減らす。2人以上いる場合、さらにheart00を2減らす。
//!   3人以上いる場合、さらにheart00を1減らす。
//!
//! In English: −3 required heart00 for the first member one of my Printemps cards'
//! effects switched out of wait this turn, −2 more for the second, −1 more for the
//! third — and nothing beyond three.
//!
//! # What is built, and what is not
//!
//! BUILT (shared with `PL!-pb2-010-P＋`, whose 7 tests pass):
//!   * `GameState::cards_activated_this_turn` — members switched out of wait this
//!     turn, recorded where the orientation actually changes
//!     (`ability/effects/change_state.rs`), cleared with the other per-turn trackers.
//!   * `EffectFilter::activated_this_turn` and `ConditionCommon::activated_this_turn`.
//!   * `evaluate_card_count_condition` consults the history (condition/card.rs:2364).
//!
//! # What is built, and what is not
//!
//! BUILT (shared with `PL!-pb2-010-P＋`, whose 7 tests pass):
//!   * `GameState::cards_activated_this_turn` — members switched out of wait this turn,
//!     recorded where the orientation actually changes
//!     (`ability/effects/change_state.rs`), cleared with the other per-turn trackers.
//!   * `EffectFilter::activated_this_turn` and `ConditionCommon::activated_this_turn`.
//!   * `evaluate_card_count_condition` consults the history (condition/card.rs:2364).
//!
//! The PARSE is also right: three `modify_required_hearts` steps worth 3, 2 and 1,
//! each behind its own `card_count_condition` on `count >= 1 / 2 / 3`.
//!
//! Three earlier readings of this card were wrong, and they are recorded because
//! each looked conclusive:
//!   * "a flat reduction of 10 regardless of the count" — the helper returned
//!     `printed − modifier`, mixing a printed value with a delta.
//!   * "the reduction applies unconditionally" — the activated members were never
//!     placed ON THE STAGE, and the conditions are `location: stage` counts.
//!   * "the ability does nothing" — the ability was never TRIGGERED. It is
//!     ライブ開始時; the helper staged members and called `recalculate_constants()`,
//!     which does not run a live-start ability at all.
//!
//! The helper now performs the live and fires the trigger, so what it measures is the
//! ability. See docs/PBLL02_ABILITY_REVIEW.md 8.3 for the current result.

use crate::helpers::*;

const HIKARI: &str = "PL!-pb2-040-L";
const P_1: &str = "PL!-bp6-017-N"; // 小泉花陽
const P_2: &str = "PL!-bp5-012-N"; // 南 ことり
const P_3: &str = "PL!-pb1-026-N";
const FILLER: &str = "PL!-sd1-010-SD";

/// The heart00 REQUIREMENT MODIFIER for `count` members activated this turn.
///
/// The modifier is read directly. A previous version of this helper returned
/// `printed − modifier`, which mixes a printed value with a delta and made the numbers
/// meaningless (a printed 8 with a −2 modifier "reduces" by 10) — which is exactly how
/// the "flat 10 regardless" reading got in. Nothing was wrong with the engine.
/// The heart00 requirement modifier on 040 after PERFORMING it as the live, with
/// `count` Printemps members staged and switched out of wait this turn.
///
/// The ability is ライブ開始時, so it has to be TRIGGERED — an earlier version of this
/// helper only staged the members and called `recalculate_constants()`, which never runs
/// a live-start ability at all, and the modifier was trivially 0. That, not the engine,
/// is why it read as "the ability does nothing".
fn modifier_for(count: usize) -> i16 {
    use rabuka_engine::core::types::AbilityTrigger;
    use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let f = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
    }

    // The activated members must be on the stage AND in the activation history: the
    // step conditions are `location: stage` card counts over the history.
    for (i, card_no) in [P_1, P_2, P_3].iter().take(count).enumerate() {
        let id = game.id(card_no);
        // All three stage slots are usable: 040 is a LIVE card and is performed from
        // the live card zone, so it never occupies the stage's centre slot. Alternating
        // between two side slots (as an earlier version did) made a third member
        // overwrite the first, so the 3人以上 step could never be met.
        game.state.player1.stage.stage[i] = id;
        if !game.state.cards_activated_this_turn.contains(&id) {
            game.state.cards_activated_this_turn.push(id);
        }
    }

    // 040 is a LIVE card, not a member: it is performed through the live-card-set
    // phase, not `play_to_stage`. (Using `play_to_stage_baton` here failed outright.)
    let hikari = game.id(HIKARI);
    game.assert_card_identity(hikari, HIKARI);
    game.add_to_hand(hikari);
    game.give_energy(40);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(hikari);
    game.drain_choices();

    let mut h = BaseHeart { hearts: HeartMap::new() };
    h.hearts.insert(HeartColor::Heart00, 30);
    game.state.player1.stage_hearts = Some(h);

    let ability_id = {
        let card = game.db.get_card(hikari).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("{HIKARI} lacks a ライブ開始時 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(hikari).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(hikari);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(hikari),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();

    let m = game
        .state
        .mods
        .get_need_heart_modifier(hikari, HeartColor::Heart00);
    println!(
        "  count={count} printed={} modifier={m}",
        game.db
            .get_card(hikari)
            .and_then(|c| c.need_heart.as_ref())
            .and_then(|n| n.hearts.get(&HeartColor::Heart00))
            .copied()
            .unwrap_or(0)
    );
    m
}

#[test]
fn no_activated_member_reduces_nothing() {
    assert_eq!(
        modifier_for(0),
        0,
        "このターン中に…アクティブにしていたメンバーが1人以上いる場合 — with nothing \
         activated no step may apply"
    );
}

#[test]
fn one_activated_member_reduces_need_by_three() {
    assert_eq!(
        modifier_for(1),
        -3,
        "1人以上いる場合、必要ハートをheart00を3減らす — the step's condition is a \
         card_count_condition (location stage, count >= 1, activated_this_turn) and is \
         satisfiable in this fixture"
    );
}

#[test]
fn two_activated_members_reduce_need_by_five() {
    assert_eq!(modifier_for(2), -5, "3 + 2 — the first two steps");
}

#[test]
fn three_activated_members_reduce_need_by_six() {
    assert_eq!(modifier_for(3), -6, "3 + 2 + 1 — all three steps");
}

/// A member that is in wait AGAIN now still counts, because the clause is retrospective.
///
/// The previous version of this test asserted `0` and passed — but only because it
/// staged the member, sampled the modifier, called `recalculate_constants()` (which
/// never runs a ライブ開始時) and compared against the value from one line earlier.
/// Nothing in between could have changed it, so the assertion was unconditionally
/// true and its premise was never actually tested.
///
/// The premise was also wrong. 「このターン中に…ウェイト状態からアクティブにしていた」
/// is retrospective: the step's condition is a `card_count_condition` over the
/// *activation history* (`activated_this_turn`), so membership of that history is what
/// counts. A member that was activated this turn and has since been waited again still
/// satisfies it. Measured: the modifier is -3, not 0.
#[test]
fn a_member_waited_again_after_being_activated_still_counts() {
    use rabuka_engine::core::types::AbilityTrigger;
    use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let f = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
    }
    let hikari = game.id(HIKARI);
    let waiting = game.id(P_1);
    // A real activation history entry whose member is currently in wait: this is the
    // case 「ウェイト状態からアクティブにしていた」 must not count.
    game.state.player1.stage.stage[0] = waiting;
    game.state.mods.add_orientation_modifier(waiting, "wait");
    if !game.state.cards_activated_this_turn.contains(&waiting) {
        game.state.cards_activated_this_turn.push(waiting);
    }

    game.add_to_hand(hikari);
    game.give_energy(40);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(hikari);
    game.drain_choices();

    let mut h = BaseHeart { hearts: HeartMap::new() };
    h.hearts.insert(HeartColor::Heart00, 30);
    game.state.player1.stage_hearts = Some(h);

    let ability_id = {
        let card = game.db.get_card(hikari).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("{HIKARI} lacks a ライブ開始時 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(hikari).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(hikari);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(hikari),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();

    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(hikari, HeartColor::Heart00),
        -3,
        "ウェイト状態からアクティブにしていた — the step counts MEMBERS OF THE ACTIVATION \
         HISTORY, not members currently active. This one was activated this turn and has \
         since been waited again, which does not undo the activation, so the 1人以上 step \
         still applies."
    );
}

/// The clause parses as a per-turn history, not a state filter.
#[test]
fn the_clause_parses_as_a_history_not_a_state() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(HIKARI);
    let eff = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("ライブ開始時 ability has an effect")
    };
    assert_eq!(
        eff.action,
        rabuka_engine::ability::enums::ActionType::Sequential,
        "three escalating steps, one per threshold"
    );
    // The history flag must be on the nodes that carry the thresholds.
    let json = serde_json::to_value(&eff).unwrap();
    let found = json.to_string().contains("activated_this_turn");
    assert!(
        found,
        "このターン中に…アクティブにしていた — every step's clause must be tagged as \
         an activation history"
    );
    assert!(
        !json.to_string().contains("\"state\":\"wait\""),
        "and no step may still demand the member be in wait"
    );
}
