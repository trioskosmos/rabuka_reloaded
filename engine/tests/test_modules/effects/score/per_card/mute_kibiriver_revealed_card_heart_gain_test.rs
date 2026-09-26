use crate::helpers::*;

/// 無敵級*ビリーバー (PL!N-bp5-029-L):
/// Live Start: If "中須かすみ" on stage → reveal 4 from deck top,
/// select 1 "中須かすみ" from them, gain that card's heart colors,
/// discard all revealed cards.
#[test]
fn mute_kibiriver_normal_flow() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");
    let kasumi = game.id("PL!N-bp5-002-R");

    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);

    // Deck top: include a Kasumi card for selection (different copy from stage)
    for _ in 0..5 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!N-bp5-002-R"));
    }
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    // Handle pending choice — select a card
    while game.has_pending_choice() {
        let acts = game.generated_actions();
        if !acts.is_empty() {
            game.select_generated(0);
            game.drain_auto_ability_choices();
        } else {
            game.select_indices(&[]);
            game.drain_auto_ability_choices();
        }
    }

    // Debug: check all modifiers
    for color in &[
        rabuka_engine::card::HeartColor::Heart01,
        rabuka_engine::card::HeartColor::Heart03,
        rabuka_engine::card::HeartColor::Heart04,
        rabuka_engine::card::HeartColor::Heart05,
        rabuka_engine::card::HeartColor::Heart06,
    ] {
        let val = game.state.mods.get_heart_modifier(kasumi, *color);
    }

    // PL!N-bp5-002-R has heart03=3, heart04=1, heart05=1, heart06=1 in base_heart
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart01),
        0,
        "heart01 not in selected card → 0"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart03),
        1,
        "heart03 in selected card → +1"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart04),
        1,
        "heart04 in selected card → +1"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart05),
        1,
        "heart05 in selected card → +1"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart06),
        1,
        "heart06 in selected card → +1"
    );

    // All revealed cards discarded
    assert!(
        game.state.player1.waitroom.cards.len() >= 4,
        "Revealed cards should be discarded (got {})",
        game.state.player1.waitroom.cards.len()
    );
}

/// No Kasumi on stage → condition fails → no effect
#[test]
fn mute_kibiriver_no_kasumi_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);

    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(!game.has_pending_choice(), "No Kasumi → no effect");
}

#[test]
fn mute_kibiriver_no_kasumi_in_revealed_no_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");
    let kasumi = game.id("PL!N-bp5-002-R");
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);
    let filler = game.id("PL!-sd1-010-SD");
    // Deck top 4: all filler, no Kasumi
    for _ in 0..4 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    // Reveal 4 filler, no Kasumi to select → should have no SelectCard or auto-skip
    // The ability should still discard the 4 revealed
    while game.has_pending_choice() {
        let acts = game.generated_actions();
        if !acts.is_empty() { game.select_generated(0); } else { game.select_indices(&[]); }
        game.drain_auto_ability_choices();
    }
    assert!(
        !game.has_pending_choice()
    );
    // The ability looks at 4 and discards them; the waitroom is empty before
    // this, so "exactly 4" is checkable and ">= 4" is not worth asserting.
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        4,
        "the 4 revealed filler cards were discarded"
    );
    // No heart should be gained because no Kasumi selected
    assert_eq!(game.state.mods.get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart03), 0);
}

#[test]
fn mute_kibiriver_multiple_kasumi_in_revealed_select_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");
    let kasumi = game.id("PL!N-bp5-002-R");
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);
    // Deck top 4: 2 Kasumi + 2 filler.
    //
    // Built with put_on_deck_top, NOT push. `push` appends to the TOP of the
    // deck, so the original fixture pushed the two Kasumi first and then ten
    // filler on top of them: the engine revealed four filler, never saw a
    // Kasumi, and the "select a Kasumi" prompt this test drives had nothing to
    // select. The test passed because the discarded count was ">=".
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // Two DISTINCT instances: `put_on_deck_top` with one id twice would stage
    // the same card in two slots, and then the revealed set is not four cards.
    let kasumi_a = game.new_id("PL!N-bp5-002-R");
    let kasumi_b = game.new_id("PL!N-bp5-002-R");
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, kasumi_b);
    put_on_deck_top(&mut game, 0, kasumi_a);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert!(game.has_pending_choice(), "should have selection with multiple Kasumi");
    game.select_indices(&[0]);
    while game.has_pending_choice() { game.select_indices(&[]); game.drain_auto_ability_choices(); }
    assert!(!game.has_pending_choice());
    // 「公開したカードをすべて控え室に置く」 — ALL revealed cards, so both
    // Kasumi (selected and not) and both fillers land there. Identity first,
    // then the count.
    let wait = &game.state.player1.waitroom.cards;
    assert!(
        wait.contains(&filler),
        "the revealed filler cards are discarded"
    );
    assert!(
        wait.contains(&kasumi_a) && wait.contains(&kasumi_b),
        "「公開したカードをすべて控え室に置く」 includes the selected card, not \
         just the ones left behind; waitroom holds {} cards",
        wait.len()
    );
    assert_eq!(
        wait.len(),
        4,
        "all four revealed cards are discarded, the selected one included"
    );

    // The ability's actual effect, which no assertion in this file covered:
    // 「これにより選んだカードが持つ色のハートを1つずつ得る」 — the STAGED Kasumi
    // gains one heart of every colour the selected card prints. Derived from the
    // card data rather than hard-coded, so both instances agree on it.
    let selected = game.db.get_card(kasumi_a).expect("card in db");
    let printed = selected
        .base_heart
        .as_ref()
        .expect("a member prints hearts")
        .hearts
        .iter()
        .filter(|(color, count)| *count > 0 && (1..=6).contains(&color.index()))
        .count() as i16;
    let gained: i32 = (1..=6)
        .map(|i| {
            game.state
                .mods
                .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::from_index(i))
        })
        .sum();
    assert_eq!(
        gained, printed as i32,
        "the staged Kasumi gains one heart per colour the SELECTED card prints \
         ({} colours on PL!N-bp5-002-R)",
        printed
    );
}
