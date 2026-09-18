use crate::helpers::*;

fn heart_mods(game: &TestGame, card: i16) -> [i32; 6] {
    use rabuka_engine::card::HeartColor;
    [
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart01),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart02),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart03),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart04),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart05),
        game.state
            .mods
            .get_heart_modifier(card, HeartColor::Heart06),
    ]
}

#[test]
fn live_start_optional_energy_gains_chosen_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-bp1-003-R＋");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage = [-1, shizuku, -1];
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(20);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    game.set_live_card(live_card);

    let before = heart_mods(&game, shizuku);
    eprintln!(
        "[BEFORE] total: 01={} 02={} 03={} 04={} 05={} 06={}",
        before[0],
        before[1],
        before[2],
        1 + before[3],
        3 + before[4],
        before[5]
    );

    game.pass();
    game.pass();

    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected before heart selection"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget (pay_optional_cost:skip)"
    );
    game.select_option(1);
    assert!(
        game.has_pending_choice(),
        "heart color selection should be pending"
    );
    // SelectHeartColor options are [heart01..heart06], 0-indexed.
    // heart04 is at index 3.
    game.select_option(3);

    let after = heart_mods(&game, shizuku);
    eprintln!(
        "[AFTER]  total: 01={} 02={} 03={} 04={} 05={} 06={}",
        after[0],
        after[1],
        after[2],
        1 + after[3],
        3 + after[4],
        after[5]
    );

    assert_eq!(after[3], 1, "heart04 should have +1 modifier");
    assert_eq!(after[4], 0, "heart05 should have 0 modifier");
}
