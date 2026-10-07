//! 東條 希 `PL!-pb2-016-P＋` — the 「1枚につき」 REPEAT that the existing file does not cover.
//!
//! Text:
//!
//!   ライブ開始時 自分の成功ライブカード置き場にある『lily white』のカード1枚につき、
//!   以下から1つを選ぶ。**同じ選択肢を2回以上選んでもよい。**
//!     ・自分のセンターエリアにいるメンバーは、ライブ終了時まで、ブレードを得る。
//!     ・自分のステージにいるメンバー1人をアクティブにする。
//!     ・カードを1枚引き、手札を1枚控え室に置く。
//!
//! # Why this is a separate file
//!
//! `pb2_repeatable_three_option_choice_per_lily_white_live_pl_pb2_016_p_plus_test.rs`
//! pins that the choice is PRESENTED and that picking an option runs it — but every test
//! in it stands exactly ONE `lily white` card in the success zone (`&[MUS_LIVE]`), and
//! with one card 「1枚につき」 and 「1回だけ」 are indistinguishable. That file therefore
//! passes against an engine that offers the choice exactly once, always.
//!
//! # The printed mechanic
//!
//! 「『lily white』のカード1枚につき、以下から1つを選ぶ」 scales the NUMBER OF CHOICES with
//! the number of qualifying cards, and 「同じ選択肢を2回以上選んでもよい」 only makes sense
//! if the player is picking more than once — otherwise a repeat could never occur. So two
//! `lily white` cards in the success zone means two separate choices.
//!
//! # The engine half of this
//!
//! `execute_choice` documents the per-card reading explicitly (`choice.rs:98-106`:
//! "there they scope HOW MANY TIMES the choice is offered, not what each option may
//! touch") and then only uses the clause to decide NOT to propagate `group_names` into
//! the options. Nothing counts the qualifying cards or re-offers the choice.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const RESONANCE: &str = "PL!-pb2-016-P＋";
const FILLER: &str = "PL!-sd1-010-SD";
/// A `lily white` live for the success live zone — the choice is taken per card.
const MUS_LIVE: &str = "PL!-pb1-029-SRL";
/// A second, DISTINCT instance of the same print so the success zone can hold 2 cards.
const MUS_LIVE_2: &str = "PL!-pb2-025-N";

/// Stand RESONANCE and fire ライブ開始時 with `count` qualifying `lily white` cards in the
/// success live card zone.
fn fired_with_lily_white_count(count: usize) -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Both prints are `lily white` lives, so both qualify under 自分の成功ライブカード置き場
    // にある『lily white』のカード. Distinct instances, never the same id twice.
    let one = game.new_id(MUS_LIVE);
    let two = game.id(MUS_LIVE_2);
    assert_ne!(
        one, two,
        "the two qualifying cards must be distinct instances"
    );
    for (label, id) in [("A", one), ("B", two)] {
        let c = game.db.get_card(id).unwrap();
        assert_eq!(
            c.unit.as_deref(),
            Some("lily white"),
            "precondition: qualifying card {label} ({id}) must be a lily white card, \
             otherwise 「『lily white』のカード1枚につき」 is counting the wrong set"
        );
    }
    for id in [one, two].iter().take(count) {
        game.state
            .player1
            .success_live_card_zone
            .cards
            .push(*id);
    }

    let r = game.id(RESONANCE);
    game.assert_card_identity(r, RESONANCE);
    game.state.player1.live_card_zone.cards.push(r);

    let ability_id = {
        let card = game.db.get_card(r).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("{RESONANCE} lacks a ライブ開始時 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(r).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(r);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(r),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    game
}

/// Take the per-card choice once, answering any CARD selections its option spawns, and
/// report whether the choice came back.
///
/// Option 2 (カードを1枚引き、手札を1枚控え室に置く) finishes by asking the player to pick the
/// hand card to discard, so answering the option is not a single step. This drains only
/// card-selection prompts and stops as soon as a `choice` prompt is on screen again, which
/// is the signal that the per-card repeat happened.
fn is_option_choice(game: &TestGame) -> bool {
    if !game.has_pending_choice() {
        return false;
    }
    matches!(
        game.get_pending_choice(),
        rabuka_engine::ability::types::Choice::SelectTarget { target, .. } if target == "choice"
    )
}

fn take_choice_once(game: &mut TestGame) -> bool {
    game.select_option(2);
    let mut guard = 0;
    while guard < 10 {
        if is_option_choice(game) {
            return true;
        }
        if !game.has_pending_choice() {
            return false;
        }
        game.select_indices(&[0]);
        guard += 1;
    }
    is_option_choice(game)
}

/// THE gameplay claim: two qualifying cards means the player chooses TWICE.
///
/// 「同じ選択肢を2回以上選んでもよい」 is only meaningful if the choice is offered more
/// than once, and 「1枚につき」 sets the multiplier at the number of qualifying cards.
#[test]
fn two_lily_white_cards_offer_the_choice_twice() {
    let mut game = fired_with_lily_white_count(2);
    assert!(
        game.has_pending_choice(),
        "precondition: the first choice is offered"
    );

    assert!(
        take_choice_once(&mut game),
        "『lily white』のカード1枚につき、以下から1つを選ぶ — TWO lily white cards are in the \
         成功ライブカード置き場, so the choice must be offered TWICE. It was offered once and \
         then exhausted, which drops half of the printed effect. \
         (同じ選択肢を2回以上選んでもよい confirms repeats are intended.)"
    );
}

/// The control: ONE qualifying card means exactly one choice, and no more.
///
/// Without this, the test above would also pass on an engine that simply re-offers the
/// choice unconditionally.
#[test]
fn one_lily_white_card_offers_the_choice_exactly_once() {
    let mut game = fired_with_lily_white_count(1);
    assert!(
        game.has_pending_choice(),
        "precondition: the choice is offered for the one qualifying card"
    );

    assert!(
        !take_choice_once(&mut game),
        "ONE lily white card means ONE choice — 1枚につき with a multiplier of 1. A second \
         choice here would mean the repeat is not driven by the card count"
    );
}

/// THE distinction from 「1つ以上を選ぶ」: here the SAME option stays selectable.
///
/// This card prints 「同じ選択肢を2回以上選んでもよい」, so the repeat must restore the
/// FULL option list. The `any_number` path used by `PL!N-bp4-030-L` does the opposite —
/// it removes the chosen option — and that is right THERE, because Bandai's own FAQ for
/// that card answers 「ライブ成功時効果が発動した際、同じ効果を２回選ぶことができますか？」
/// with 「いいえ。できません。」 (`PL!N-bp4-030-L` carries that FAQ in cards/cards.json).
///
/// So the two shapes are deliberately asymmetric, and this pins the half that is easy to
/// break: collapsing the per-card repeat onto the `any_number` path would silently take
/// the repeat clause away from this card.
#[test]
fn the_same_option_stays_selectable_on_the_repeat() {
    let mut game = fired_with_lily_white_count(2);
    assert!(game.has_pending_choice(), "precondition: the choice is offered");

    // Option 0 = 「自分のセンターエリアにいるメンバーは、ライブ終了時まで、ブレードを得る。」
    const OPTION_0: &str = "センターエリア";
    game.select_option(0);
    let mut guard = 0;
    while guard < 10 && !is_option_choice(&game) && game.has_pending_choice() {
        game.select_indices(&[0]);
        guard += 1;
    }

    assert!(
        is_option_choice(&game),
        "precondition: the per-card repeat is offered again"
    );
    let desc = match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectTarget { description, .. } => {
            description.clone()
        }
        other => panic!("expected a SelectTarget option prompt, got {other:?}"),
    };
    assert!(
        desc.contains(OPTION_0),
        "同じ選択肢を2回以上選んでもよい — the option just taken must STILL be offered on \
         the repeat. The `any_number` path (PL!N-bp4-030-L) removes it, but that card's FAQ \
         answers 「いいえ。できません。」 to the same question; THIS card prints the opposite, \
         so the per-card repeat must restore the full option list. Repeat prompt was: {desc}"
    );
}