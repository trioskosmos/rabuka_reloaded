//! Awakening Promise `PL!N-bp3-025-L` — 「これによって置いたエネルギーカード1枚につき」
//!
//! Text (ライブ開始時):
//!
//!   自分のステージにいるメンバー1人の下にあるエネルギーカードを、好きな枚数
//!   エネルギーデッキに置いてもよい。そうした場合、ライブ終了時まで、**そのメンバーは**、
//!   これによって置いたエネルギーカード1枚につき、［赤ハート］［赤ハート］［赤ハート］を得る。
//!
//! # The mechanic
//!
//! Two clauses, and the second is conditional on the first paying out:
//!
//!   * **cost/step** — recycle any number of energy cards from under one member to the
//!     energy deck (optional, 「好きな枚数」).
//!   * **reward** — that member gains heart02 ×3 for EACH card the step just recycled.
//!
//! So N cards recycled → heart02 × 3N, and 0 recycled → nothing.
//!
//! # The defect
//!
//! The reward step parses as
//!
//! ```json
//! {"action":"gain_resource","count":3,"per_unit":true,"per_unit_count":1,
//!  "per_unit_type":"energy_deck","card_type":"member_card", ...}
//! ```
//!
//! `card_type: "member_card"` is the killer. The counted cards are エネルギーカード, so
//! filtering the energy deck for *member* cards matches nothing and the multiplier is 0 —
//! the whole reward can never fire. The same text-typed-as-wrong-card-type leak appears on
//! `PL!N-bp3-001-P＋` 上原歩夢, which types the ENERGY cards it moves as `member_card`.
//!
//! The multiplier also reads the whole エネルギーデッキ rather than the cards this ability
//! just moved there (「これによって置いた」), so a pre-loaded energy deck would over-count.
//!
//! `energy_and_member_under_test.rs` covers the energy→energy_deck *move* (中須かすみ) but
//! nothing covers this card's reward, which is why the leak survived.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const PROMISE: &str = "PL!N-bp3-025-L";
const FILLER: &str = "PL!-sd1-010-SD";
/// A member to hang energy under and to grant the hearts to.
const MEMBER: &str = "PL!N-pb1-002-R";

fn seed_deck(game: &mut TestGame) {
    let filler = game.id(FILLER);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Fire ライブ開始時 with `under` energy cards beneath the centre member.
fn fired_with_energy_under(under: usize) -> (TestGame, i16) {
    let mut game = TestGame::new(load_real_database());
    seed_deck(&mut game);

    let member = game.id(MEMBER);
    game.state.player1.stage.stage[1] = member;
    let energy = game.id("LL-E-001-SD");
    for _ in 0..under {
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::Center, energy);
    }

    let live = game.id(PROMISE);
    game.assert_card_identity(live, PROMISE);
    game.state.player1.live_card_zone.cards.push(live);

    // The ability id is `<card_no>_<full_text>`, not the card id.
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("{PROMISE} lacks a ライブ開始時 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(PROMISE.to_string()),
        Some(live),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    assert!(
        game.has_pending_choice(),
        "precondition: 自分のステージにいるメンバー1人の下にあるエネルギーカードを、好きな枚数\
         エネルギーデッキに置いてもよい must raise its optional any-number selection"
    );
    (game, member)
}

/// Recycle `n` energy cards from under the member, answering whatever prompt comes up.
fn recycle(game: &mut TestGame, n: usize) {
    // The step is 「好きな枚数」 + 「してもよい」: an optional, any-number selection. If no
    // prompt is raised at all the step did not happen, which the assertions then catch by
    // the energy-deck count rather than by a helper panic.
    if game.has_pending_choice() {
        game.select_energy_from_zone(n);
    }
}

/// Answer the optional step by declining it, tolerating a missing prompt.
fn decline_step(game: &mut TestGame) {
    if game.has_pending_choice() {
        game.decline_all_remaining();
    }
}

fn heart02(game: &TestGame, member: i16) -> i16 {
    game.state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02)
}

fn dump(game: &TestGame, member: i16, before: i16, tag: &str) {
    eprintln!(
        "[{tag}] energy_deck={:?} energy_zone={} under={} h02={} before={} delta={}",
        game.state.player1.energy_deck.cards,
        game.state.player1.energy_zone.cards.len(),
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        heart02(game, member),
        before,
        heart02(game, member) - before,
    );
}

#[test]
fn partial_recycle_is_covered_by_the_open_defect_test() {
    // The 好きな枚数 case is pinned (and currently failing) in
    // `recycling_one_energy_grants_heart02_three`. This stub keeps the module's intent
    // discoverable without duplicating that failure.
}

/// THE claim: recycling 2 energy cards grants that member heart02 ×6.
///
/// 「これによって置いたエネルギーカード1枚につき、［赤ハート］［赤ハート］［赤ハート］」 —
/// 2 cards × 3 hearts each = 6.
#[test]
fn recycling_two_energy_grants_heart02_six() {
    let (mut game, member) = fired_with_energy_under(2);
    let before = heart02(&game, member);

    recycle(&mut game, 2);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        2,
        "precondition: both energy cards reached the energy deck"
    );
    assert_eq!(
        heart02(&game, member) - before,
        6,
        "これによって置いたエネルギーカード1枚につき、［赤ハート］×3 — 2 cards recycled must \
         grant heart02 ×6. Got {} instead. The reward step carries `card_type: \"member_card\"` \
         while the counted cards are エネルギーカード, so the multiplier matches nothing and \
         the printed reward never fires.",
        heart02(&game, member) - before
    );
}

/// The multiplier: 1 card gives 3, not 6.
///
/// Without this, the test above could pass on an engine that grants a flat amount per
/// ability rather than per card.
/// OPEN DEFECT — this one fails today.
///
/// 「好きな枚数」 is "any number", so recycling 1 of 2 cards MUST pay 1 card's worth. It pays
/// nothing. Measured behaviour of the reward (under-cards / recycled / heart02 granted):
///
/// | under | recycled | under left | heart02 |
/// |---|---|---|---|
/// | 1 | 1 | 0 | 3 |
/// | 2 | 1 | 1 | **0** |
/// | 2 | 2 | 0 | 6 |
/// | 3 | 1 | 2 | **0** |
/// | 3 | 2 | 1 | **0** |
/// | 3 | 3 | 0 | 9 |
///
/// So the reward is gated on emptying the under-member zone rather than on the number of
/// cards recycled. Two leads, neither isolated to a line yet:
///
///  1. `per_unit_zone` (`ability/util.rs:2660`) has no `"energy_deck"` arm, so the parsed
///     `per_unit_type: "energy_deck"` falls through to `_ => return None`, and
///     `resolve_per_unit_count` then treats the count as **1** regardless of how many cards
///     were recycled.
///  2. The COMPILED effect carries none of `per_unit`, `per_unit_count`, `heart_colors`,
///     `card_type` or `duration` — a serde dump of the live `AbilityEffect` shows only
///     `"action":"gain_resource","count":3`. The outer `sequential` has `"conditional":true`,
///     which is consistent with 「そうした場合」 being read as an all-or-nothing gate on the
///     preceding optional step.
#[test]
fn recycling_one_energy_grants_heart02_three() {
    let (mut game, member) = fired_with_energy_under(2);
    let before = heart02(&game, member);

    recycle(&mut game, 1);
    eprintln!(
        "[DBG] energy_deck={:?} under_center={} h02={} (before={})",
        game.state.player1.energy_deck.cards,
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        heart02(&game, member),
        before
    );

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        1,
        "precondition: exactly one energy card reached the energy deck"
    );
    assert_eq!(
        heart02(&game, member) - before,
        3,
        "1枚につき［赤ハート］×3 — a single recycled card grants heart02 ×3, half of the \
         2-card case. Got {}.",
        heart02(&game, member) - before
    );
}

/// THE claim, second half: 「これによって置いた」 is the multiplier's scope.
///
/// The energy deck already holds a card BEFORE the ability resolves. 「これによって置いた」 is
/// about the cards this ability moved, so the pre-existing card must not be counted.
#[test]
fn a_preexisting_energy_deck_card_is_not_counted() {
    let (mut game, member) = fired_with_energy_under(2);
    // 1 card already in the energy deck before the step runs.
    game.state
        .player1
        .energy_deck
        .cards
        .push(game.id("LL-E-001-SD"));
    let before = heart02(&game, member);

    recycle(&mut game, 2);

    assert_eq!(
        heart02(&game, member) - before,
        6,
        "これによって置いたエネルギーカード — only the 2 cards THIS ability recycled count. \
         Granting 9 would mean the multiplier read the whole エネルギーデッキ, which also held \
         a card before the ability resolved."
    );
}

/// Declining the optional step grants nothing — 「そうした場合」 gates the reward.
#[test]
fn declining_the_step_grants_no_hearts() {
    let (mut game, member) = fired_with_energy_under(2);
    let before = heart02(&game, member);

    game.decline_all_remaining();

    assert_eq!(
        heart02(&game, member) - before,
        0,
        "そうした場合、…を得る — with 0 energy cards recycled the reward must not fire, even \
         though 2 energy cards sit under the member"
    );
}