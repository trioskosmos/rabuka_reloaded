/// Tests for PL!SP-bp7-011-R (鬼塚冬毬) ab#0 — 登場: 手札をすべて控え室に置いてもよい：カードを6枚引く。
///
/// The optional cost is "discard ALL cards from hand" — not "discard 1".
/// These tests verify the engine offers a discard-all-or-skip choice.
///
/// Ruling note (colon-gated optional cost): "手札をすべて控え室に置いてもよい：
/// カードを6枚引く" uses the "may [discard X]: [effect Y]" pattern. Per the
/// engine's resolver (cost.rs / resolver.rs), if the optional cost is skipped
/// OR cannot be paid (empty hand), the gated effect does NOT fire. So:
///   - Accept: discard entire hand, then draw 6.
///   - Skip:   nothing discarded, no draw (hand unchanged).
///   - Empty hand: cost auto-skips, no draw.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn hand_of(game: &mut TestGame) -> Vec<i16> {
    game.state.player1.hand.cards.to_vec()
}

/// Play 鬼塚冬毬 to stage and accept the discard-all hand cost, then draw 6.
fn play_tomari_discard_all(game: &mut TestGame, tomari: i16) {
    game.add_to_hand(tomari);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(tomari, MemberArea::LeftSide);
    // The debut cost prompts: discard all hand (or skip).
    assert!(
        game.has_pending_choice(),
        "Should prompt for discard-all-hand optional cost"
    );
    game.select_option(1); // accept: pay_cost_all
}

#[test]
fn tomari_discards_entire_hand_then_draws_6() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tomari = game.id("PL!SP-bp7-011-R");
    let filler = game.id("PL!-sd1-010-SD");

    // 3 cards in hand besides the member
    let extra = [game.id("PL!-sd1-011-SD"), game.id("PL!-sd1-012-SD"), game.id("PL!-sd1-013-SD")];
    for &c in &extra {
        game.add_to_hand(c);
    }
    // Deck has enough for draw 6
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.give_energy(9);

    let hand_before = hand_of(&mut game);
    let expected_discard = hand_before.len();

    play_tomari_discard_all(&mut game, tomari);

    // After accepting: hand should be empty except the 6 drawn.
    // Cost: 1 card (the member) is played to stage, then ALL remaining hand
    // cards are discarded, then 6 drawn.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        6,
        "Should draw 6 cards after discarding entire hand"
    );
    // The discarded cards are in waitroom
    assert!(
        game.state.player1.waitroom.cards.len() >= expected_discard,
        "Discarded cards should be in waitroom"
    );
    for c in &extra {
        assert!(
            !game.state.player1.hand.cards.contains(c),
            "Hand should not contain discarded card"
        );
    }
}

#[test]
fn tomari_skips_discard_effect_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tomari = game.id("PL!SP-bp7-011-R");
    let filler = game.id("PL!-sd1-010-SD");

    let extra = [game.id("PL!-sd1-011-SD"), game.id("PL!-sd1-012-SD")];
    for &c in &extra {
        game.add_to_hand(c);
    }
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(9);

    let hand_before = hand_of(&mut game);

    game.add_to_hand(tomari);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(tomari, MemberArea::LeftSide);
    assert!(
        game.has_pending_choice(),
        "Should prompt for discard-all-hand optional cost"
    );
    game.select_option(0); // skip

    // Skipping the optional discard gates the effect — no draw occurs.
    // The hand keeps its cards (no discard happened).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before.len(),
        "Skipping discard should keep hand unchanged (effect does not fire)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&extra[0])
            && !game.state.player1.waitroom.cards.contains(&extra[1]),
        "No cards should be discarded when skipping"
    );
}

#[test]
fn tomari_empty_hand_auto_skips_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tomari = game.id("PL!SP-bp7-011-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(9);

    // Hand only contains the member being played
    game.add_to_hand(tomari);
    game.state.player1.hand.cards.retain(|c| *c == tomari);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(tomari, MemberArea::LeftSide);

    // With no cards in hand to discard, the optional discard-all cost
    // auto-skips and the gated draw effect does not fire.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Empty hand: discard auto-skips, no draw (effect gated by cost)"
    );
}
