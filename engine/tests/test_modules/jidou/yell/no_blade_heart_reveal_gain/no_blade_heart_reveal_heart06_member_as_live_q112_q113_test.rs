use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// A live card with a REAL score requirement, so performing it actually yells.
/// A 0-score filler live card produces no yell at all, which is the defect this
/// file previously hid behind.
const LIVE_CARD: &str = "PL!S-bp3-020-L";

/// Advance to the yell window and read the modifier DURING the live.
///
/// The ability is ライブ終了時まで (until live end), so the heart modifier is only
/// observable *before* the live ends. Advancing further and reading 0 is correct
/// expiry, not a failure — which is exactly what these three tests used to
/// observe and misreport as an engine bug.
fn advance_into_live(game: &mut TestGame, live_card: &str) {
    for _ in 0..5 {
        game.pass();
    }
    let live = game.id(live_card);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    for _ in 0..3 {
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }
}

fn build(game: &mut TestGame, stage_left: &str, deck_card: &str) -> i16 {
    let sumire = game.id("PL!SP-bp2-015-N");
    game.assert_card_identity(sumire, "PL!SP-bp2-015-N");
    game.state.player1.stage.stage = [game.new_id(stage_left), sumire, -1];
    let d = game.id(deck_card);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(d);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    sumire
}

/// Q112 positive: a REAL yell revealing no blade heart → heart06.
///
/// This test previously used a 0-score live card, so NO yell occurred and the
/// ability correctly stayed silent; it asserted 0 and blamed an "auto-trigger
/// bug". It also read the modifier *after* the live, where a ライブ終了時まで
/// grant has legitimately expired. Both defects are corrected here: a real live
/// card that yells, and a read during the live.
#[test]
fn yell_q112_real_yell_no_blade_heart_grants_heart06() {
    let mut game = TestGame::new(load_real_database());
    let sumire = build(&mut game, "PL!S-sd1-003-SD", "LL-E-001-SD"); // energy: no blade heart
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        0,
        "precondition: no heart before the live"
    );

    advance_into_live(&mut game, LIVE_CARD);

    assert!(
        !game.state.initial_yell_revealed_cards.is_empty(),
        "a real yell revealed cards (premise the old fixture never produced)"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        1,
        "a real yell revealing no blade heart grants heart06 DURING the live"
    );
}

/// Q112 negative: a real yell that DOES reveal a blade heart → condition fails.
///
/// Previously this also used a 0-score live card, so it never tested the
/// condition at all — it read 0 for the same reason the positive did.
#[test]
fn yell_q112_real_yell_with_blade_heart_does_not_grant_heart06() {
    let mut game = TestGame::new(load_real_database());
    let sumire = build(&mut game, "PL!S-sd1-003-SD", "PL!S-sd1-003-SD"); // has a blade heart

    advance_into_live(&mut game, LIVE_CARD);

    assert!(
        !game.state.initial_yell_revealed_cards.is_empty(),
        "precondition: a real yell still happened; only the revealed set differs"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        0,
        "a yelLED blade heart makes the negation false → no heart06. With a real \
         yell this actually tests the condition, which the old 0-score fixture \
         could not."
    );
}

/// Q113: a 0-score live card produces no yell at all → no heart06.
///
/// This negative is CORRECT as written; it is retained (with its premise stated)
/// because it is the honest counterpart to the positives above, and it is what
/// the old file was accidentally asserting for all three cases.
#[test]
fn yell_q113_zero_score_live_yields_no_yell_and_no_heart06() {
    let mut game = TestGame::new(load_real_database());
    let sumire = build(&mut game, "PL!S-sd1-003-SD", "PL!-sd1-010-SD");

    advance_into_live(&mut game, "PL!-sd1-010-SD"); // 0-score live card

    assert!(
        !game.state.yell_occurred,
        "precondition: a 0-score live card produces no yell at all"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06),
        0,
        "no yell → the yell-triggered 自動 must not fire"
    );
}
