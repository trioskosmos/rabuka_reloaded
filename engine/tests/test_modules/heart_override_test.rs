use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_performance(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
    }
}

/// Kasumi (PL!N-bp3-014-N) has base hearts: heart03:1.
/// Her LiveStart ability (ab#0): select heart01/heart03/heart04 → until live end,
/// this member's original heart becomes the selected heart.
/// Verifies total_hearts in the display reflects each of the 3 possible choices.
#[test]
fn kasumi_heart_transform_reflected_in_header_stats() {
    use rabuka_engine::display::game_state_to_display;

    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let kasumi = game.id("PL!N-bp3-014-N");
    let filler = game.id("PL!-sd1-010-SD");

    // Kasumi on stage center.
    game.state.player1.stage.stage = [-1, kasumi, -1];
    fill_both_decks(&mut game, filler);

    advance_to_performance(&mut game);

    // Kasumi's LiveStart fires, SelectHeartColor choice should be pending.
    assert!(
        game.has_pending_choice(),
        "SelectHeartColor should be pending"
    );

    // Test each of the 3 choices (heart01, heart03, heart04)
    for (option, expected_color, expected_index) in [
        (0, HeartColor::Heart01, 1),
        (1, HeartColor::Heart03, 3),
        (2, HeartColor::Heart04, 4),
    ] {
        let mut game = TestGame::new(db.clone());
        let kasumi = game.id("PL!N-bp3-014-N");
        let filler = game.id("PL!-sd1-010-SD");
        game.state.player1.stage.stage = [-1, kasumi, -1];
        fill_both_decks(&mut game, filler);
        advance_to_performance(&mut game);

        // Select the heart color option
        game.select_option(option);

        // Verify the transform was applied
        assert_eq!(
            game.state.mods.heart_color_multiplier.get(&kasumi),
            Some(&expected_color),
            "Kasumi's heart transform should be {:?}",
            expected_color
        );

        // Verify total_hearts in display reflects the transform
        let display = game_state_to_display(&game.state);
        let total = &display.player1.total_hearts;
        let expected_idx = expected_index as usize;
        assert_eq!(
            total[expected_idx], 1,
            "total_hearts[{}] should be 1 after transform to {:?}",
            expected_idx, expected_color
        );
        // All other heart slots should be 0 (only card on stage has 1 heart, transformed)
        for (i, &val) in total.iter().enumerate() {
            if i != expected_idx && i < 7 {
                assert_eq!(
                    val, 0,
                    "total_hearts[{}] should be 0 after transform to {:?}, got {}",
                    i, expected_color, val
                );
            }
        }
    }
}

fn fill_both_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
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

    // Kanan on stage center.
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

    // Verify heart_color_multiplier only contains Kanan (not filler)
    assert_eq!(
        game.state.mods.heart_color_multiplier.len(),
        1,
        "Only Kanan should have heart_color_multiplier"
    );
    assert!(
        game.state.mods.heart_color_multiplier.contains_key(&kanan),
        "Kanan should be in heart_color_multiplier"
    );

    // Kanan's own hearts should be set to heart04 in the multiplier
    {
        let mult = &game.state.mods.heart_color_multiplier;
        assert_eq!(
            mult.get(&kanan),
            Some(&HeartColor::Heart04),
            "Kanan's multiplier should be heart04"
        );
    }

    // Kanan's stage heart contribution now shows only heart04
    let after = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
        &Default::default(),
        &Default::default(),
    );
    // The filler (PL!-sd1-010-SD) has its own hearts too, so after.hearts.len() > 1
    assert_eq!(
        after.hearts.get(&HeartColor::Heart04),
        Some(&6),
        "Kanan's 6 hearts become heart04"
    );
    assert_eq!(
        after.hearts.values().sum::<u8>(),
        game.state
            .player1
            .calculate_stage_hearts(
                &game.state.card_database,
                &Default::default(),
                &Default::default(),
                &Default::default(),
            )
            .hearts
            .values()
            .sum::<u8>(),
        "Total unchanged (filler + Kanan)"
    );

    // Advance through performance phase and verify that Kanan's member contribution
    // reflects the heart transformation during live.
    game.pass();
    // Kanan's member contribution in the snapshot should have all hearts as heart04
    let kanan_contribution = game.state.performance_snapshots.first().and_then(|snap| {
        snap.member_contributions
            .iter()
            .find(|mc| mc.source_id == kanan)
    });
    if let Some(mc) = kanan_contribution {
        assert_eq!(
            mc.base_hearts[HeartColor::Heart04.index()],
            6,
            "Kanan's 6 hearts should be heart04 in member contribution"
        );
        assert_eq!(
            mc.base_hearts[HeartColor::Heart02.index()],
            0,
            "heart02 should be 0 in member contribution"
        );
        assert_eq!(
            mc.base_hearts[HeartColor::Heart05.index()],
            0,
            "heart05 should be 0 in member contribution"
        );
    }
}
