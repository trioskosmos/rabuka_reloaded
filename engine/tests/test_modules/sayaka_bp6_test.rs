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

#[test]
fn sayaka_bp6_live_start_modify_cost_only_applies_to_dollchestra() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka = game.id("PL!HS-bp6-010-R"); // 村野さやか, activating card, unit=DOLLCHESTRA
    let doll = game.id("PL!HS-PR-008-PR"); // 徒町小鈴, unit=DOLLCHESTRA
    let non_doll = game.id("PL!HS-bp1-012-PR"); // 乙宗梢, unit=スリーズブーケ (not DOLLCHESTRA)
    let live = game.id("PL!-sd1-020-SD"); // live card
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [doll, sayaka, non_doll];
    // Hand: 1 DOLLCHESTRA card for cost + live card
    let cost_card = game.new_id("PL!HS-bp1-013-N"); // DOLLCHESTRA card
    game.add_to_hand(cost_card);
    game.add_to_hand(live);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Pay optional cost: discard 1 DOLLCHESTRA card
    assert!(game.has_pending_choice(), "Should prompt for optional cost");
    game.select_indices(&[0]);

    // Draw happens automatically. No more choices expected.
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let doll_mod = game.state.mods.get_cost_modifier(doll);
    let sayaka_mod = game.state.mods.get_cost_modifier(sayaka);
    let non_doll_mod = game.state.mods.get_cost_modifier(non_doll);

    eprintln!(
        "cost mods: doll={} sayaka={} non_doll={}",
        doll_mod, sayaka_mod, non_doll_mod
    );

    assert_eq!(doll_mod, 5, "DOLLCHESTRA member should get +5 cost");
    assert_eq!(
        sayaka_mod, 5,
        "Activating card (DOLLCHESTRA) should get +5 cost"
    );
    assert_eq!(non_doll_mod, 0, "Non-DOLLCHESTRA member should get 0 cost");
}

#[test]
fn sayaka_bp6_skip_cost_no_draw_no_cost_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp6-010-R");
    let doll = game.id("PL!HS-PR-008-PR");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [doll, sayaka, -1];
    let cost_card = game.new_id("PL!HS-bp1-013-N");
    game.add_to_hand(cost_card);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..2 { game.pass(); }
    assert!(game.has_pending_choice(), "optional cost prompt expected");
    // Skip the cost (empty selection)
    game.select_indices(&[]);
    while game.has_pending_choice() { game.select_indices(&[]); }
    // No draw, no cost mod when cost skipped
    assert_eq!(game.state.mods.get_cost_modifier(doll), 0, "skipped cost should give 0");
    assert_eq!(game.state.mods.get_cost_modifier(sayaka), 0, "skipped cost should give 0");
}
