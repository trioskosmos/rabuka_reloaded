/// BP07 CLEAN-G1: "自分のデッキの下から…控え室に置く" / "デッキの一番下のカード"
/// must move cards FROM THE BOTTOM of the deck (deck_bottom), not from hand.
///
/// The parser bug set `source:"hand"` for these, so the engine would have
/// looked at the hand instead of the deck. Fixed in parser_utils SOURCE_PATTERNS.
///
/// Covered abilities (all must move the BOTTOM cards to discard):
///   - PL!S-bp7-006-R 津島善子 ab#0: ライブ開始時, デッキの下から3枚→控え室,
///     全部Aqoursメンバーなら heart04 を得る
///   - PL!S-bp7-015-N 津島善子 ab#0: 起動, デッキの下から1枚→控え室
///   - PL!S-bp7-008-R 小原鞠莉 ab#1: ライブ開始時, デッキの一番下のカード→控え室(任意)
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Seed P1's deck: [top_marker, fillers..., b1, b2, b3] so the LAST three cards
/// are the distinct bottom markers. Returns (top_marker_id, bottoms) where
/// `bottoms` is in POP order from the deck end: [b3, b2, b1] — index 0 is the
/// true bottom-most card (first one removed).
fn seed_deck_with_bottom(game: &mut TestGame) -> (i16, Vec<i16>) {
    let filler = game.id("PL!-sd1-010-SD");
    let a = game.id("PL!S-bp7-006-R"); // 津島善子 (top marker)
    let b1 = game.id("PL!SP-sd1-001-SD"); // 澁谷かのん
    let b2 = game.id("PL!SP-sd1-003-SD"); // 嵐千砂都
    let b3 = game.id("PL!SP-sd1-004-SD"); // 平安名すみれ
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(a); // index 0 = top
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.push(b1);
    game.state.player1.main_deck.cards.push(b2);
    game.state.player1.main_deck.cards.push(b3); // last = bottom
    (a, vec![b3, b2, b1])
}

// ====================================================================
// 津島善子 PL!S-bp7-006-R ab#0 — live start, deck bottom 3 → discard
// ====================================================================

/// The 3 cards moved are the BOTTOM cards, not the top card and not the hand.
#[test]
fn live_start_bottom_three_mills_deck_bottom_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let yoshiko = game.id("PL!S-bp7-006-R");
    game.state.player1.stage.stage = [-1, yoshiko, -1];

    game.give_energy(3);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    // Seed AFTER the live-card-set draws (which pull from the top) so the
    // bottom markers stay at the deck end for the live-start trigger.
    let (top_marker, bottoms) = seed_deck_with_bottom(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // The bottom 3 (b1,b2,b3) moved to discard; the top marker (a) stayed in deck.
    let p1_waitroom: Vec<i16> = game.state.player1.waitroom.cards.iter().copied().collect();
    assert!(
        !p1_waitroom.contains(&top_marker),
        "the TOP deck card must NOT be discarded (source must be deck_bottom)"
    );
    for b in &bottoms {
        assert!(
            p1_waitroom.contains(b),
            "bottom card {:?} should be in discard, discard={:?}",
            b,
            p1_waitroom
        );
    }
    assert_eq!(
        p1_waitroom.len(),
        3,
        "exactly 3 cards discarded, got {}",
        p1_waitroom.len()
    );
}

// ====================================================================
// Condition side: 津島善子 ab#0 grants heart04 when all 3 are Aqours members.
// ====================================================================

/// The follow-up "それらがすべて『Aqours』のメンバーカードの場合、heart04を得る"
/// still fires — the 3 Aqours bottom cards give this member heart04.
#[test]
fn live_start_bottom_three_all_aqours_gives_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let yoshiko = game.id("PL!S-bp7-006-R");
    game.state.player1.stage.stage = [-1, yoshiko, -1];

    game.give_energy(3);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    // Seed AFTER the live-card-set draws so the 3 Aqours cards sit at the bottom.
    let filler = game.id("PL!-sd1-010-SD");
    let aqours1 = game.id("PL!S-bp7-011-N"); // 桜内梨子 (Aqours)
    let aqours2 = game.id("PL!S-bp7-015-N"); // 津島善子 (Aqours)
    let aqours3 = game.id("PL!S-bp7-017-N"); // 小原鞠莉 (Aqours)
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler); // top
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.push(aqours1); // bottom-most
    game.state.player1.main_deck.cards.push(aqours2);
    game.state.player1.main_deck.cards.push(aqours3); // bottom

    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // heart04 should be granted (all 3 discarded are Aqours members).
    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(yoshiko, HeartColor::Heart04);
    assert!(
        heart_mod > 0,
        "heart04 should be granted when all 3 bottom cards are Aqours, got {}",
        heart_mod
    );
}
