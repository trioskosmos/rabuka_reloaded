//! 桂城 泉's 登場 — four clauses in one sentence, and the card that made three of
//! them negative without any help.
//!
//! 「自分と相手のステージにいる**元々持つ**ブレードの数が**3つ以下**の**すべて**のメンバーを
//! ウェイトにする。」
//!
//! The parsed effect is `target: both, original_value: true, blade_limit: 3,
//! blade_limit_operator: "<=", all: true` — so four separate claims:
//!
//!   * 自分**と相手のステージ** — the opponent's members are in scope;
//!   * 元々持つ — the PRINTED blade count, not a running total (`original_value`);
//!   * 3つ**以下** — an inclusive upper bound, so exactly 3 qualifies;
//!   * **すべて** — every qualifying member, not one of them.
//!
//! And 泉 herself prints 4 blades, so her OWN card is excluded by her own filter —
//! the negative case the printed text creates for free, and the one an
//! implementation that dropped the `blade_limit` would immediately break.
//!
//! Both tests assert their premises by reading the card, so a card-pool edit that
//! changed a blade count names itself instead of quietly inverting a conclusion.
//!
//! ## 高海千歌 `PL!S-bp3-010-N` — also here, and NOT the shape I first assumed
//!
//! 「自分のステージにいるメンバーを1人までアクティブにする」 parses to
//! `count: 1, max: true`, and a first draft read that as "demote the rest so at
//! most one is active". It is not: `change_state` implements `max` as a BOUNDED
//! PROMPT — "Select up to 1 member(s) to change state", with `allow_skip` — so the
//! effect makes members active up to a limit and never demotes anything. Testing the
//! demotion reading failed on an engine that was correct, and the drafts were dropped
//! rather than committed. What remains untested there is the prompt's own bound.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const IZUMI: &str = "PL!HS-pb1-008-R"; // 桂城 泉 — both stages, printed blades <= 3, all
/// Printed blade counts 2 / 3 / 4, verified in each test's premises.
const BLADES_2: &str = "PL!-bp3-012-PR"; // 南ことり, 2
const BLADES_3: &str = "PL!-sd1-001-SD"; // 高坂穂乃果, 3
const BLADES_4: &str = "PL!-PR-003-PR"; // 南ことり, 4
const FILLER: &str = "PL!-sd1-010-SD";

fn orientation(game: &TestGame, card: i16) -> Option<&str> {
    game.state.mods.get_orientation_modifier(card)
}

/// The printed blade count, read from the card so a test can assert its own
/// premise instead of trusting a comment.
fn printed_blades(game: &TestGame, card: i16) -> u8 {
    game.state.card_database.get_card(card).unwrap().blade
}

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
// 桂城 泉 — both stages, PRINTED blades <= 3, all of them
// ====================================================================

/// The four clauses, exercised together: BOTH stages, the printed blade count, the
/// `<= 3` bound, and ALL qualifying members.
///
/// 泉 herself prints 4 blades, so her own card is the negative case the printed
/// text creates without any extra work — and the test asserts that, because a
/// filter that ignored the blade bound would put the card that triggered the whole
/// thing to wait.
#[test]
fn izumi_waits_every_member_on_both_stages_whose_printed_blades_are_three_or_fewer() {
    let mut game = TestGame::new(load_real_database());
    let izumi = game.id(IZUMI);
    let two = game.id(BLADES_2);
    let three = game.id(BLADES_3);
    let four = game.id(BLADES_4);
    let foe_two = game.id(BLADES_2);
    let foe_three = game.id(BLADES_3);
    let filler = game.id(FILLER);
    game.assert_card_identity(izumi, IZUMI);
    fill_decks(&mut game);

    // Premises: the exact printed counts the text keys on, including 泉's own.
    for (id, want) in [
        (two, 2u8),
        (three, 3),
        (four, 4),
        (foe_two, 2),
        (foe_three, 3),
        (izumi, 4),
    ] {
        assert_eq!(
            printed_blades(&game, id),
            want,
            "precondition: this member must print exactly {want} blade(s) — the \
             condition is on 元々持つブレードの数, so the fixture is the test"
        );
    }

    game.state.player1.stage.stage = [two, three, -1];
    game.state.player2.stage.stage = [foe_two, foe_three, -1];
    game.add_to_hand(izumi);
    game.give_energy(20);
    // Play her into a free p1 slot; the effect then covers BOTH stages.
    game.state.player1.stage.stage[2] = -1;
    game.play_to_stage(izumi, MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // `自分と相手のステージ` — the opponent's members are included.
    assert_eq!(
        orientation(&game, foe_two),
        Some("wait"),
        "自分**と相手のステージ** — an opponent member printing 2 blades is inside the \
         bound and must be waited too. A p1-only reading leaves it active."
    );
    assert_eq!(
        orientation(&game, foe_three),
        Some("wait"),
        "and the opponent's 3-blade member: 『3つ以下』 is INCLUSIVE, so exactly 3 \
         qualifies"
    );

    // p1's own qualifying members.
    assert_eq!(
        orientation(&game, two),
        Some("wait"),
        "own member with 2 printed blades: inside the bound"
    );
    assert_eq!(
        orientation(&game, three),
        Some("wait"),
        "own member with 3 printed blades: 『3つ以下』 is inclusive, so the boundary \
         value qualifies"
    );

    // The upper bound, and 泉 herself.
    assert_eq!(
        orientation(&game, four),
        None,
        "a member printing 4 blades is OUTSIDE 『3つ以下』 and must be untouched — the \
         ability records nothing for a member it does not act on, so 'no entry' is \
         the reading for 'still active as staged'"
    );
    assert_eq!(
        orientation(&game, izumi),
        None,
        "泉 herself prints 4 blades, so her own card is excluded by her own filter. \
         A reading that ignored the bound would put the card that triggered the whole \
         effect to wait, which is the failure this assertion exists for"
    );
    let _ = filler;
}

/// The bound is on the PRINTED count, so a member whose blades were changed by an
/// earlier effect is still judged on what it printed.
///
/// This is the `original_value: true` clause. It is separately breakable — judging
/// the CURRENT blade total instead would exclude a 2-blade member carrying a +3
/// modifier — and nothing else in the suite distinguishes the two.
#[test]
fn izumi_judges_the_printed_blade_count_not_a_modified_one() {
    let mut game = TestGame::new(load_real_database());
    let izumi = game.id(IZUMI);
    let two = game.id(BLADES_2);
    game.assert_card_identity(izumi, IZUMI);
    fill_decks(&mut game);
    assert_eq!(
        printed_blades(&game, two),
        2,
        "precondition: this member prints 2 blades, well inside the bound"
    );

    // Its CURRENT blades now stand at 6 — outside the bound. The printed count is
    // what the text names, so the member must still be waited.
    game.state
        .mods
        .add_blade_modifier(two, 4);
    assert!(
        printed_blades(&game, two) + 4 > 3,
        "precondition: the CURRENT total must be outside the bound, or this test \
         cannot tell the two readings apart"
    );

    game.state.player1.stage.stage = [two, -1, -1];
    game.add_to_hand(izumi);
    game.give_energy(20);
    game.play_to_stage(izumi, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        orientation(&game, two),
        Some("wait"),
        "『元々持つブレードの数が3つ以下』 reads the PRINTED count: 2 blades qualifies \
         even while an earlier effect has pushed its total to 6. Judging the current \
         total instead would leave it active, and that is the only assertion here that \
         distinguishes the two readings."
    );
}
