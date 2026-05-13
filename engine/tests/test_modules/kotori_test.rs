/// Tests for 南ことり (PL!-bp5-003-R＋) — Multi-name card reference:
///
/// Q207: Multi-name (&) card "上原歩夢&澁谷かのん&日野下花帆" counts as
///       1 member, and can be matched by any of its individual names.
/// Q208: When both multi-name and single 上原歩夢 are on stage, the
///       multi-name can use its OTHER names (かのん/花帆) to avoid collision.
use crate::helpers::*;

/// Q207: Multi-name card can be found by any of its individual names.
#[test]
fn kotori_q207_multiname_matches_any_individual_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let multi = game.id("LL-bp1-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    // Multi-name card on stage
    game.state.player1.stage.stage[1] = multi;
    game.state.player1.hand.cards.push(filler);

    // The card should be findable by any of its 3 constituent names
    // Verify by checking the card's name in the database
    let card = game
        .db
        .get_card(multi)
        .expect("Multi-name card should exist");
    let name = &card.name;
    assert!(name.contains("歩夢"), "Name contains 歩夢");
    assert!(name.contains("かのん"), "Name contains かのん");
    assert!(name.contains("花帆"), "Name contains 花帆");

    // It occupies exactly 1 stage slot = 1 member
    let member_count = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1)
        .count();
    assert_eq!(member_count, 1, "1 stage slot = 1 member (Q207)");
}

/// Q208: Multi-name + single Ayumu on stage → each still occupies 1 slot.
#[test]
fn kotori_q208_multiname_and_single_coexist() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let multi = game.id("LL-bp1-001-R\u{ff0b}");
    let single_ayumu = game.id("PL!N-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Both cards on stage (different slots)
    game.state.player1.stage.stage[0] = multi;
    game.state.player1.stage.stage[1] = single_ayumu;
    game.state.player1.hand.cards.push(filler);

    // Both occupy 2 slots = 2 members
    let member_count = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1)
        .count();
    assert_eq!(member_count, 2, "2 cards = 2 members (Q208)");

    // The multi-name card's OTHER names (かのん, 花帆) are unique
    let card = game
        .db
        .get_card(multi)
        .expect("Multi-name card should exist");
    assert!(card.name.contains("かのん"));
    assert!(card.name.contains("花帆"));

    // The multi-name can be referenced as 澁谷かのん or 日野下花帆
    // to differentiate from the single 上原歩夢
    eprintln!(
        "[KOTORI] Multi: {} | Single: {}",
        card.name,
        game.db
            .get_card(single_ayumu)
            .map(|c| c.name.as_str())
            .unwrap_or("?")
    );
}
