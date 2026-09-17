use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_skippable_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

#[test]
fn success_zone_group_draw_live_leaves_no_choice_with_matching_group_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-pb1-032-L");
    let fid2 = game.id_ref("PL!-sd1-010-SD");
    let m = game.new_id("PL!-sd1-001-SD"); // μ's member
    game.state.player1.stage.stage = [m, m, m];
    fill_decks(&mut game, fid2);
    game.state.player1.hand.cards.push(live);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    for _ in 0..7 {
        game.pass();
        drain_skippable_choices(&mut game);
    }

    assert!(
        !game.has_pending_choice(),
        "chain resolved"
    );
}

#[test]
fn success_zone_group_draw_live_leaves_no_choice_with_wrong_group_success_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-pb1-032-L");
    let fid = game.id_ref("PL!-sd1-010-SD");
    // Stage with non-μ's members (Hasunosora) so success zone will have Hasunosora live, not μ's
    let hasu = game.id("PL!HS-bp1-001-R");
    game.state.player1.stage.stage = [hasu, hasu, hasu];
    fill_decks(&mut game, fid);
    game.state.player1.hand.cards.push(live);
    // Put a Hasunosora live in success zone to ensure μ's condition fails
    let hasu_live = game.id("PL!HS-bp1-019-L");
    game.state.player1.success_live_card_zone.cards.push(hasu_live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    for _ in 0..7 { game.pass(); drain_skippable_choices(&mut game); }
    // With Hasunosora in success zone (not μ's), the μ's condition should fail → no draw
    assert!(!game.has_pending_choice());
}

#[test]
fn success_zone_group_draw_live_leaves_no_choice_with_initially_empty_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-pb1-032-L");
    let fid = game.id_ref("PL!-sd1-010-SD");
    let m = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m, m];
    fill_decks(&mut game, fid);
    game.state.player1.hand.cards.push(live);
    // Ensure success zone empty
    game.state.player1.success_live_card_zone.cards.clear();
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    for _ in 0..7 { game.pass(); drain_skippable_choices(&mut game); }
    assert!(!game.has_pending_choice());
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}
