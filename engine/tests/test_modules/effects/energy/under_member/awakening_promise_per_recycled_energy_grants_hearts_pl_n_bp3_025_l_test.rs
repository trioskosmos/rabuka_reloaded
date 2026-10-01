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
//! Two clauses, the second conditional on the first paying out:
//!
//!   * **step** — recycle any number of energy cards from under one member to the energy
//!     deck (optional, 「好きな枚数」).
//!   * **reward** — that member gains heart02 ×3 for EACH card the step just recycled.
//!
//! So N cards recycled → heart02 × 3N, and 0 recycled → nothing.
//!
//! # What the multiplier counts
//!
//! 「これによって置いた」 is the cards **this ability moved**, not the contents of the
//! エネルギーデッキ and not the cards left under the member. The tests below discriminate
//! between those readings by holding the starting board fixed and changing only the
//! 1-card difference between "recycled" and "left behind":
//!
//! | under | recycled | under left | heart02 |
//! |---|---|---|---|
//! | 1 | 1 | 0 | 3 |
//! | 2 | 1 | 1 | 3 |
//! | 2 | 2 | 0 | 6 |
//! | 3 | 3 | 0 | 9 |
//!
//! A zone-counting engine would score rows 2 and 4 as 1 and 9; one that counts the
//! under-member set would score them 1 and 0.
//!
//! # The 「好きな枚数」 re-prompt is load-bearing
//!
//! 「好きな枚数」 re-prompts after every pick, offering whatever is still under the member.
//! A player who takes fewer than all of them then SKIPS the rest, and both halves are
//! required: the skip closes the step, and only then does 「そうした場合、…を得る」 resolve.
//! `recycle` does both.
//!
//! Answering only the first prompt leaves the ability mid-step, the reward never resolves,
//! and the assertion sees 0 — which reads as an engine bug but is a half-played turn. That
//! is exactly what happened while this file was being written: an intermediate version
//! left the re-prompt pending and produced a convincing but entirely fake "the engine only
//! pays when every card is recycled" table.
//!
//! # Why the recipient is not the counted set
//!
//! The reward carries `card_type: "member_card"` even though the counted cards are energy
//! cards. That is deliberate (`parser.py`, the 「そのメンバー」 override): `card_type` is the
//! RECIPIENT of 「…を得る」, while `per_unit_type` is the counted set. Reading
//! `card_type` as the counted set looks like a parse bug and is not one.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const PROMISE: &str = "PL!N-bp3-025-L";
const FILLER: &str = "PL!-sd1-010-SD";
/// The member hosting the energy, and the recipient of the reward.
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
    game.state.activating_card = Some(live);
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
    (game, member)
}

fn is_under_member_repick(game: &TestGame) -> bool {
    game.has_pending_choice()
        && matches!(
            game.get_pending_choice(),
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "under_member"
        )
}

/// Recycle exactly `n` energy cards from under the member.
fn recycle(game: &mut TestGame, n: usize) {
    // 「…置いてもよい」 may raise a pay/skip gate before the selection.
    let mut guard = 0;
    while guard < 4 && !is_under_member_repick(game) {
        if !game.has_pending_choice() {
            return;
        }
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target.contains("pay_optional_cost") =>
            {
                game.select_choice_option(1); // pay
            }
            _ => {
                game.decline_all_remaining();
            }
        }
        guard += 1;
    }

    game.select_energy_from_zone(n);

    // 「好きな枚数」 re-prompts with what is left. STOP picking by skipping.
    let mut guard = 0;
    while guard < 4 && is_under_member_repick(game) {
        game.select_indices(&[]);
        guard += 1;
    }
}

/// Decline the optional step entirely.
fn decline_step(game: &mut TestGame) {
    let mut guard = 0;
    while guard < 4 && game.has_pending_choice() {
        if is_under_member_repick(game) {
            game.select_indices(&[]);
        } else {
            game.decline_all_remaining();
        }
        guard += 1;
    }
}

fn heart02(game: &TestGame, member: i16) -> i16 {
    game.state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02)
}

/// Drive a recycle and return the heart02 granted to the member.
fn granted_after_recycling(under: usize, n: usize) -> i16 {
    let (mut game, member) = fired_with_energy_under(under);
    let before = heart02(&game, member);
    recycle(&mut game, n);
    heart02(&game, member) - before
}

/// 「これによって置いたエネルギーカード1枚につき、［赤ハート］×3」 — 2 cards → heart02 ×6.
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
        "2 cards recycled × 3 hearts each must be heart02 ×6"
    );
}

/// 「好きな枚数」 really is any number: 1 card out of 2 pays 1 card's worth.
///
/// The discriminator row. Same starting board as the 2-card case, one card still left
/// under the member, and the reward must be half.
#[test]
fn recycling_one_energy_grants_heart02_three() {
    let granted = granted_after_recycling(2, 1);
    assert_eq!(
        granted,
        3,
        "1枚につき［赤ハート］×3 — one recycled card grants heart02 ×3, half the 2-card case. \
         Got {granted}, which would mean the multiplier is reading the under-member set (1) or \
         the energy deck rather than the cards this ability moved."
    );
}

/// The top of the range: 3 cards → heart02 ×9.
#[test]
fn recycling_three_energy_grants_heart02_nine() {
    assert_eq!(
        granted_after_recycling(3, 3),
        9,
        "3 cards recycled × 3 hearts each must be heart02 ×9"
    );
}

/// The multiplier scales with the count rather than granting a flat amount.
#[test]
fn the_reward_scales_with_the_number_recycled() {
    assert_eq!(
        granted_after_recycling(1, 1),
        3,
        "1 card under, 1 recycled → heart02 ×3"
    );
    assert_eq!(
        granted_after_recycling(2, 2),
        6,
        "2 cards under, 2 recycled → heart02 ×6"
    );
    assert_eq!(
        granted_after_recycling(3, 3),
        9,
        "3 under, 3 recycled → heart02 ×9 — linear in the count, so the effect is per-card"
    );
}

/// 「これによって置いた」 is the multiplier's scope.
///
/// The energy deck already holds a card BEFORE the ability resolves. Only the cards THIS
/// ability recycled count.
#[test]
fn a_preexisting_energy_deck_card_is_not_counted() {
    let (mut game, member) = fired_with_energy_under(2);
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
         A larger grant would mean the multiplier read the whole エネルギーデッキ, which also \
         held a card before the ability resolved."
    );
}

/// Declining the optional step grants nothing — 「そうした場合」 gates the reward.
#[test]
fn declining_the_step_grants_no_hearts() {
    let (mut game, member) = fired_with_energy_under(2);
    let before = heart02(&game, member);

    decline_step(&mut game);

    assert_eq!(
        heart02(&game, member) - before,
        0,
        "そうした場合、…を得る — with 0 energy cards recycled the reward must not fire, even \
         though 2 energy cards sit under the member"
    );
}