//! Daydream Mermaid `PL!N-bp4-030-L` (ライブ成功時) — 「代わりに1つ以上を選ぶ」 may NOT repeat.
//!
//! Text:
//!
//!   ライブ成功時 以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードが
//!   ある場合、**代わりに1つ以上を選ぶ。**
//!     ・自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
//!     ・自分の控え室からメンバーカードを1枚手札に加える。
//!
//! # The official ruling, which is what makes this asymmetric with 東條 希
//!
//! `cards/cards.json` carries this FAQ verbatim for `PL!N-bp4-030-L`:
//!
//! > Q: ライブ成功時効果が発動した際、同じ効果を２回選ぶことができますか？
//! > A: **いいえ。できません。**
//!
//! So when 「1つ以上を選ぶ」 is the operative clause, each option may be taken **once**.
//! That is why the engine's `any_number` re-prompt removes the chosen option from the list.
//!
//! `PL!-pb2-016-P＋` 東條 希 is the exact opposite — it prints
//! 「同じ選択肢を2回以上選んでもよい」 and its per-card repeat restores the full option list.
//! `per_unit_choice_repeats_per_lily_white_card_pl_pb2_016_p_plus_test.rs` pins that half.
//! This file pins this half, so neither shape can be "simplified" into the other.
//!
//! # What is actually being observed
//!
//! Not the effect's board result, but the *menu*: after taking an option under 「1つ以上」,
//! the next prompt must offer only the options that are still untaken. Picking the same
//! option twice would let one ライブ成功時 pay out twice, which the FAQ forbids.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const MERMAID: &str = "PL!N-bp4-030-L";
const FILLER: &str = "PL!-sd1-010-SD";
/// A `虹ヶ咲` live. `虹ヶ咲` is a SERIES, not a unit — no card has `unit == "虹ヶ咲"`, so the
/// qualifying check is on `series`. This is what makes 「代わりに1つ以上を選ぶ」 operative.
const NIRAGASAKE_LIVE: &str = "PL!N-bp1-025-L";

/// Option 0's distinguishing text. Option 1 is 「自分の控え室からメンバーカードを1枚手札に
/// 加える」, which shares no marker with it.
const OPTION_0_MARKER: &str = "エネルギーデッキ";

/// Fire ライブ成功時 with a `虹ヶ咲` card in the success live zone, so the alternative
/// 「1つ以上を選ぶ」 applies.
fn fired() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let lg = game.id(NIRAGASAKE_LIVE);
    let c = game.db.get_card(lg).unwrap();
    assert!(
        matches!(c.card_type, rabuka_engine::card::CardType::Live),
        "precondition: the qualifying card must be a ライブ, got {:?}",
        c.card_type
    );
    assert!(
        c.series.contains("虹ヶ咲"),
        "precondition: 「自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合」 is what \
         switches the clause to 「1つ以上」. `虹ヶ咲` lives are identified by SERIES — no card \
         in the database has `unit == \"虹ヶ咲\"`."
    );
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(lg);

    let m = game.id(MERMAID);
    game.assert_card_identity(m, MERMAID);
    game.state.player1.live_card_zone.cards.push(m);

    let ability_id = {
        let card = game.db.get_card(m).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
            .unwrap_or_else(|| panic!("{MERMAID} lacks a ライブ成功時 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(m).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(m);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card_no),
        Some(m),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    game
}

fn is_option_choice(game: &TestGame) -> bool {
    if !game.has_pending_choice() {
        return false;
    }
    matches!(
        game.get_pending_choice(),
        rabuka_engine::ability::types::Choice::SelectTarget { target, .. } if target == "choice"
    )
}

/// Take option 0, drain any card selection it spawns, and return the next prompt's text.
fn after_taking_option_0(game: &mut TestGame) -> String {
    game.select_option(0);
    let mut guard = 0;
    while guard < 10 && !is_option_choice(game) && game.has_pending_choice() {
        game.select_indices(&[0]);
        guard += 1;
    }
    match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectTarget { description, .. } => {
            description.clone()
        }
        other => panic!("expected a SelectTarget option prompt, got {other:?}"),
    }
}

/// THE claim: under 「1つ以上を選ぶ」 the option just taken is gone from the next prompt.
///
/// Bandai's FAQ for this card answers 「いいえ。できません。」 to exactly this question.
#[test]
fn an_option_taken_under_any_number_cannot_be_taken_again() {
    let mut game = fired();
    assert!(
        is_option_choice(&game),
        "precondition: the choice is offered"
    );

    let next = after_taking_option_0(&mut game);
    assert!(
        !next.contains(OPTION_0_MARKER),
        "「代わりに1つ以上を選ぶ」 — the FAQ for {MERMAID} answers 「同じ効果を２回選ぶことが \
         できますか？」 with 「いいえ。できません。」, so an option already taken must NOT \
         reappear. The next prompt still offers it, which would let one ライブ成功時 pay out \
         twice. Next prompt was: {next}"
    );
}

/// The control: the option NOT yet taken is still on offer, so the removal is a per-option
/// exclusion rather than the choice ending after one pick.
#[test]
fn options_not_yet_taken_remain_available() {
    let mut game = fired();
    assert!(is_option_choice(&game), "precondition: the choice is offered");

    let next = after_taking_option_0(&mut game);
    assert!(
        next.contains("控え室"),
        "「1つ以上を選ぶ」 must still allow a further pick of the untaken option \
         (自分の控え室からメンバーカードを1枚手札に加える). Next prompt was: {next}"
    );
}

/// Taking the remaining option ends the choice — 「1つ以上」 is "one or more of the listed
/// options", each once, not an open-ended repeat.
#[test]
fn the_choice_ends_after_every_option_has_been_taken_once() {
    let mut game = fired();
    assert!(is_option_choice(&game), "precondition: the choice is offered");

    after_taking_option_0(&mut game);
    assert!(
        is_option_choice(&game),
        "precondition: one option remains"
    );
    game.select_option(0);
    let mut guard = 0;
    while guard < 10 && !is_option_choice(&game) && game.has_pending_choice() {
        game.select_indices(&[0]);
        guard += 1;
    }
    assert!(
        !is_option_choice(&game),
        "both printed options have been taken once each, so 「1つ以上」 is satisfied and the \
         choice must be over. A further prompt would let the same option be paid out again, \
         which the FAQ forbids"
    );
}