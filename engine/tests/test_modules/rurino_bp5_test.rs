use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Discard a みらくらぱーく！ card → only みらくらぱーく！ members get heart01.
/// Non-みらくらぱーく！ (μ's Printemps) are excluded.
#[test]
fn rurino_bp5_discard_only_matching_unit_gets_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp5-003-R＋"); // unit=みらくらぱーく！
    let hs = game.id("PL!HS-bp6-011-R"); // unit=みらくらぱーく！
    let muse = game.id("PL!-sd1-010-SD"); // unit=Printemps
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs, rurino, muse];
    let cost_card = game.new_id("PL!HS-bp6-011-R");
    let live = game.id("PL!-sd1-020-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    // Set hand explicitly to avoid draw-phase card index interference
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(cost_card);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Pay optional cost — select card from hand (index 0 = cost_card)
    assert!(game.has_pending_choice());
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0]);

    // Select 1 member on stage to receive heart01.
    // Both hs and rurino match the みらくらぱーく！ group.
    // Pick rurino (stage index 1 = 2nd filtered position).
    while game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        match ct.as_str() {
            "SelectCard" => {
                game.select_indices(&[1]);
            }
            "SelectTarget" => {
                game.select_option(0);
            }
            _ => break,
        }
    }

    // Exactly 1 みらくらぱーく！ member gets +1 heart01 (count=1)
    let hs_h1 = game
        .state
        .mods
        .get_heart_modifier(hs, rabuka_engine::card::HeartColor::Heart01);
    let rurino_h1 = game
        .state
        .mods
        .get_heart_modifier(rurino, rabuka_engine::card::HeartColor::Heart01);
    let total_matching = hs_h1 + rurino_h1;
    assert_eq!(
        total_matching, 1,
        "Exactly one みらくらぱーく！ member should get +1 heart01 (got hs={}, rurino={})",
        hs_h1, rurino_h1
    );
    // μ's member does NOT get heart01
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(muse, rabuka_engine::card::HeartColor::Heart01),
        0,
        "μ's member should NOT get heart01"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(rurino, rabuka_engine::card::HeartColor::Heart01),
        1,
        "rurino (みらくらぱーく！) should get +1 heart01"
    );
    // μ's member does NOT get heart01
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(muse, rabuka_engine::card::HeartColor::Heart01),
        0,
        "μ's member should NOT get heart01"
    );
}
