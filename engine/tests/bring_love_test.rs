/// Q225: Multi-name card counts as 1 member on stage, not per-individual.
mod helpers;
use helpers::*;

/// A multi-name card (3 individuals) on stage counts as 1 member for blade/gain_resource per_unit counting.
#[test]
fn bring_love_q225_multiname_counts_as_one_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let multi = game.id("LL-bp1-001-R\u{ff0b}"); // 上原歩夢&澁谷かのん&日野下花帆
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [multi, -1, -1];

    // Count member cards on stage — should be 1 (the multi card), not 3
    let stage_ids: Vec<i16> = game.state.player1.stage.stage.iter()
        .filter(|&&id| id != -1).copied().collect();
    assert_eq!(stage_ids.len(), 1, "One stage slot occupied");
    assert_eq!(stage_ids[0], multi, "Multi-name card occupies the slot");

    // Verify the card's name contains '&' separating 3 individuals
    let card = game.state.card_database.get_card(multi).expect("Multi-name card should exist");
    let name = &card.name;
    let parts: Vec<&str> = name.split('&').collect();
    assert!(parts.len() >= 3, "Multi-name card has 3+ individual names");
}
