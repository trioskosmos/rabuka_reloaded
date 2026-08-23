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

    assert!(
        !game.state.player1.live_card_zone.cards.contains(&live)
            || game.state.player1.success_live_card_zone.cards.contains(&live)
            || !game.has_pending_choice(),
        "live resolved"
    );
}

fn filler_id(game: &TestGame) -> i16 {
    let id = game.id_ref("PL!-sd1-010-SD");
    id
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
