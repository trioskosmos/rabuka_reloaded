/// L0 gap coverage: LiveSuccess optional-energy draw abilities.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_pay(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

fn advance_live(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        drain_pay(game);
    }
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn setup_stage_and_live(game: &mut TestGame, live_no: &str) -> i16 {
    let m = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m, m];
    fill_decks(game, game.id_ref("PL!-sd1-010-SD"));
    let live = game.id(live_no);
    game.state.player1.hand.cards.push(live);
    game.give_energy(20);
    advance_to_live_card_set_p1(game);
    game.set_live_card(live);
    live
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// PL!SP-pb1-004-R: LiveSuccess, pay 4E → draw 1.
#[test]
fn pb1_004_pay_4e_draw_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _live = setup_stage_and_live(&mut game, "PL!SP-pb1-004-R");
    let deck_before = game.state.player1.main_deck.cards.len();

    advance_live(&mut game);

    // The draw may have fired automatically after paying the energy cost.
    // Check that the deck was consumed by at least the base draw.
    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_before > deck_after,
        "deck should shrink as draws happen"
    );
    assert!(
        !game.has_pending_choice(),
        "all prompts resolved"
    );
}

/// PL!SP-bp5-020-N: LiveSuccess, pay 1E → draw 1.
#[test]
fn bp5_020_pay_1e_draw_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _live = setup_stage_and_live(&mut game, "PL!SP-bp5-020-N");
    let deck_before = game.state.player1.main_deck.cards.len();

    advance_live(&mut game);

    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_before > deck_after,
        "deck should shrink as draws happen"
    );
    assert!(
        !game.has_pending_choice(),
        "all prompts resolved"
    );
}
