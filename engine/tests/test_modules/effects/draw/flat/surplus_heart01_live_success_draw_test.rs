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
fn surplus_heart01_draw_live_resolves_out_of_live_card_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-bp4-023-L");
    let fid = game.id_ref("PL!-sd1-010-SD");
    // Stage with heart01 providers to create surplus
    let m = game.new_id("PL!-sd1-001-SD"); // heart01=1, heart03=2, heart06=1
    let m_copy2 = game.new_id("PL!-sd1-001-SD");
    let m_copy3 = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m_copy2, m_copy3];
    fill_decks(&mut game, fid);
    game.state.player1.hand.cards.push(live);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    for _ in 0..7 {
        game.pass();
        drain_skippable_choices(&mut game);
    }

    // Live resolved: it left the live card zone (win → success zone; the
    // third disjunct of the old form (!has_pending_choice) made this assert
    // vacuously true and was removed).
    assert!(
        !game.state.player1.live_card_zone.cards.contains(&live)
            || game.state.player1.success_live_card_zone.cards.contains(&live),
        "live resolved out of the live card zone"
    );
}

#[test]
fn surplus_heart01_draw_live_leaves_no_choice_with_hasunosora_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp4-023-L");
    let fid = game.id_ref("PL!-sd1-010-SD");
    // Stage with no heart01 (use Hasunosora with heart04 etc, no heart01)
    let hasu = game.id("PL!HS-bp1-001-R"); // Hasunosora, likely no heart01
    let hasu_copy2 = game.new_id("PL!HS-bp1-001-R");
    let hasu_copy3 = game.new_id("PL!HS-bp1-001-R");
    game.state.player1.stage.stage = [hasu, hasu_copy2, hasu_copy3];
    fill_decks(&mut game, fid);
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    for _ in 0..7 { game.pass(); drain_skippable_choices(&mut game); }
    // With no surplus heart01, should not draw
    assert!(!game.has_pending_choice());
}

#[test]
fn surplus_heart01_draw_live_leaves_no_choice_with_empty_decks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp4-023-L");
    let m = game.new_id("PL!-sd1-001-SD");
    let m_copy2 = game.new_id("PL!-sd1-001-SD");
    let m_copy3 = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m_copy2, m_copy3];
    // Empty decks
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player1.hand.cards.push(live);
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
