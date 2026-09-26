//! A mandatory look followed by an OPTIONAL banking — the rule-5.7 case, on the
//! smallest card that has it.
//!
//! 日野下花帆 `PL!HS-cl1-001-CL`:
//!
//!   「自分のデッキの上からカードを1枚**見る**。そのカードを控え室に置いても**よい**。」
//!
//! The look carries no 「てもよい」 and the banking does, so they are two separate
//! decisions, and that asymmetry is what makes this card worth a test: the printed
//! verb is 見る, which under rule 5.7 **informs but does not acquire**. Declining
//! the banking must therefore put the card back where it came from — not consume
//! it, and not leave it stranded in the looked-at pool.
//!
//! Those are three distinguishable wrong answers, and a test that only ever accepted
//! the banking would pass while the decline path was entirely untested. That was
//! true of the suite before this file: the card's single existing coverage took the
//! 「置いてよい」 branch.
//!
//! ## Also in this family, still uncovered here
//!
//! 宮下愛 `PL!N-sd2-005-SD2` and 中須かすみ `PL!N-bp3-002-R` both open with
//! 「好きなハートの色を1つ指定する」, a six-way pick feeding a counted `gain_resource`
//! (2 for 愛, 1 to ANOTHER 『虹ヶ咲』 member for かすみ). Those are the strongest
//! available CHOICE-IDENTITY tests — a run that picks two different options and
//! asserts two different colours granted cannot be satisfied by an implementation
//! that always applies the first. Drafts of them are not included because
//! `specify_heart_color` did not present its prompt under `fire_trigger` with the
//! card staged directly, and shipping a test whose failure is a fixture artefact
//! would be worse than leaving the gap visible.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const HANA: &str = "PL!HS-cl1-001-CL"; // 日野下花帆 — mandatory look, optional banking
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

// ====================================================================
// 日野下花帆 — a mandatory look and an optional banking
// ====================================================================

/// 「自分のデッキの上からカードを1枚**見る**。そのカードを控え室に置いても**よい**」.
///
/// The look is unmarked and the banking carries もよい, so the two are separate
/// decisions — and the printed 「見る」 is the rule-5.7 case: seeing a card informs
/// but does not acquire it, so DECLINING must put it back where it came from. A
/// decline that consumed the card, or left it stranded in the looked-at pool, is
/// a different reading and both fail this.
#[test]
fn hanao_declining_the_banking_returns_the_looked_card_to_the_deck() {
    let mut game = TestGame::new(load_real_database());
    let hanao = game.id(HANA);
    game.assert_card_identity(hanao, HANA);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [hanao, -1, -1];
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    fire_trigger(&mut game, hanao, AbilityTrigger::LiveStart, "ライブ開始時");
    // Decline the banking.
    let mut guard = 0;
    let mut declined = 0;
    while game.has_pending_choice() && guard < 6 {
        guard += 1;
        game.select_indices(&[]);
        declined += 1;
    }
    assert!(
        declined >= 1,
        "precondition: the optional banking must raise a choice, or this test never \
         exercised 「置いてよい」"
    );

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "「見る」 only informs: declining the banking must RETURN the card to the \
         deck, so the deck is back where it started. Consuming it would leave {} \
         here.",
        game.state.player1.main_deck.cards.len()
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "and nothing was banked, which is what declining means"
    );
    assert_eq!(
        game.state.looked_at_cards.len(),
        0,
        "nor left stranded in the looked-at pool — that would be a third reading, \
         neither taken nor returned"
    );
}

/// ACCEPTING the banking is the other branch: the card goes to the waitroom, and the
/// deck is down by exactly one.
///
/// The control for the test above — same card, same look, opposite decision.
#[test]
fn hanao_accepting_the_banking_sends_the_card_to_the_waitroom() {
    let mut game = TestGame::new(load_real_database());
    let hanao = game.id(HANA);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [hanao, -1, -1];
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    fire_trigger(&mut game, hanao, AbilityTrigger::LiveStart, "ライブ開始時");
    let mut guard = 0;
    let mut answered = 0;
    while game.has_pending_choice() && guard < 6 {
        guard += 1;
        game.select_indices(&[0]);
        answered += 1;
    }
    assert!(
        answered >= 1,
        "precondition: the optional banking must raise a choice"
    );

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "『デッキの上からカードを1枚見る』 draws it out of the deck for the look, and \
         accepting the banking keeps it out"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 1,
        "『そのカードを控え室に置く』 — exactly one card banked"
    );
}
