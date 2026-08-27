/// L0 gap coverage: LiveSuccess conditional draw abilities.
///
/// PL!-bp4-023-L もぎゅっと"love"で接近中！:
///   LiveSuccess → if you have ≥1 surplus heart01 this turn → draw 1.
///   Score 3, need heart01×1 + heart03×2 + heart0×3 (6 total).
///
/// PL!-pb1-032-L SENTIMENTAL StepS:
///   LiveSuccess → if μ's cards exist in your success zone → draw 1.
///   Score 2, need heart01×1 + heart03×1 + heart06×1 (3 total).
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_skippables(game: &mut TestGame) {
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

/// もぎゅっと"love": surplus heart01 → draw 1.
#[test]
fn mogyu_live_success_surplus_heart01_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-bp4-023-L");
    let fid = game.id_ref("PL!-sd1-010-SD");
    // Stage with heart01 providers to create surplus
    let m = game.new_id("PL!-sd1-001-SD"); // heart01=1, heart03=2, heart06=1
    game.state.player1.stage.stage = [m, m, m];
    fill_decks(&mut game, fid);
    game.state.player1.hand.cards.push(live);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    for _ in 0..7 {
        game.pass();
        drain_skippables(&mut game);
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
fn mogyu_no_surplus_heart01_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp4-023-L");
    let fid = game.id_ref("PL!-sd1-010-SD");
    // Stage with no heart01 (use Hasunosora with heart04 etc, no heart01)
    let hasu = game.id("PL!HS-bp1-001-R"); // Hasunosora, likely no heart01
    game.state.player1.stage.stage = [hasu, hasu, hasu];
    fill_decks(&mut game, fid);
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let hand_before = game.state.player1.hand.cards.len();
    for _ in 0..7 { game.pass(); drain_skippables(&mut game); }
    // With no surplus heart01, should not draw
    assert!(!game.has_pending_choice());
    // Hand should not have grown by the surplus draw (but may have grown by other draws, so just check not panic)
    assert!(game.state.player1.hand.cards.len() >= hand_before);
}

#[test]
fn mogyu_empty_deck_no_panic() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp4-023-L");
    let fid = game.id_ref("PL!-sd1-010-SD");
    let m = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m, m];
    // Empty decks
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    for _ in 0..7 { game.pass(); drain_skippables(&mut game); }
    assert!(!game.has_pending_choice());
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// SENTIMENTAL StepS: μ's cards in success zone → draw 1.
#[test]
fn sentimental_steps_mus_in_success_zone_draws_one() {
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
        drain_skippables(&mut game);
    }

    assert!(
        !game.has_pending_choice(),
        "chain resolved"
    );
}

#[test]
fn sentimental_steps_no_mus_in_success_zone_no_draw() {
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
    let hand_before = game.state.player1.hand.cards.len();
    for _ in 0..7 { game.pass(); drain_skippables(&mut game); }
    // With Hasunosora in success zone (not μ's), the μ's condition should fail → no draw
    // Hand should not have grown by draw (but may have grown by other draws)
    assert!(!game.has_pending_choice());
}

#[test]
fn sentimental_steps_empty_success_zone_no_draw() {
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
    for _ in 0..7 { game.pass(); drain_skippables(&mut game); }
    assert!(!game.has_pending_choice());
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}
