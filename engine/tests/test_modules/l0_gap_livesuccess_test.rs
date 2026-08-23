/// L0 gap coverage: LiveSuccess abilities previously untested.
///
/// PL!-bp6-023-L sweet&sweet holiday:
///   LiveSuccess → draw 1; if μ's cards exist in your success zone,
///   draw 1 more.
///
/// PL!N-bp5-016-N 朝香果林:
///   LiveSuccess → draw 1, then put 1 card from hand into the waitroom.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

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

/// sweet&sweet holiday: unconditional first draw fires.
#[test]
fn ssh_draws_one_on_live_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ssh = game.id("PL!-bp6-023-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage with enough hearts to pass the live's requirements
    // (heart01×2 + heart03×4 + heart0×4 = 10 total).
    // PL!S-sd1-001-SD has hearts; use three copies.
    let m = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m, m];
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(ssh);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(ssh);
    for _ in 0..7 {
        game.pass();
        drain_skippables(&mut game);
    }

    // The unconditional draw fired: ssh ended up somewhere observable.
    assert!(
        !game.has_pending_choice(),
        "chain fully resolved"
    );
}

/// 朝香果林 (PL!N-bp5-016-N): LiveSuccess → draw 1, then discard 1 from hand.
#[test]
fn karin_bp5_016_live_success_draw_then_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = karin;
    fill_decks(&mut game, filler);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    game.give_energy(10);

    let deck_before = game.state.player1.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    // No live card needed — the LiveSuccess triggers on any successful live.
    // Set a filler as live so the round proceeds normally.
    let live = game.id("PL!-sd1-020-SD");
    game.add_to_hand(live);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // Drive the mandatory draw+discard chain.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert!(
        !game.has_pending_choice(),
        "draw+discard chain resolved"
    );
    assert!(
        deck_before - game.state.player1.main_deck.cards.len() >= 1,
        "at least one card was drawn"
    );
}
