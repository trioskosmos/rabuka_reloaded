use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Kanan (PL!S-pb1-003-R) has base hearts: heart02:1, heart04:4, heart05:1 = 6 total.
/// Her LiveStart ability (ab#0): pay 2E → until live end, all her hearts become heart04.
/// Verifies the heart_color_multiplier mechanism through the actual card ability.
#[test]
fn kanan_livestart_converts_all_hearts_to_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let kanan = game.id("PL!S-pb1-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Kanan on stage.
    game.state.player1.stage.stage = [-1, kanan, -1];
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live_card);
    game.give_energy(2);

    advance_to_live(&mut game);
    game.set_live_card(live_card);

    // Pass to advance from LiveCardSet -> FirstAttackerPerformance -> LiveStart
    game.pass();
    // P2 turn draw → LiveStart triggers
    game.pass();

    // Kanan's LiveStart ability fires. Its cost is optional 2E — pay by selecting option 1.
    if game.has_pending_choice() {
        game.select_option(1);
    }

    // Kanan's hearts should now be converted to heart04
    let after = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
    );
    assert_eq!(
        after.hearts.get(&HeartColor::Heart02),
        None,
        "heart02 converted away"
    );
    assert_eq!(
        after.hearts.get(&HeartColor::Heart05),
        None,
        "heart05 converted away"
    );
    assert_eq!(
        after.hearts.get(&HeartColor::Heart04),
        Some(&6),
        "ALL 6 hearts become heart04"
    );
    assert_eq!(after.hearts.len(), 1, "Only heart04 remains");
    assert_eq!(after.hearts.values().sum::<u32>(), 6, "Total unchanged");
}
