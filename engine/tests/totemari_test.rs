/// Tests for 遠手鞠 (PL!HS-bp2-014-N) — Debut: draw 1, cannot live this turn.
///
/// Q68: "cannot live" means the player CAN set live cards but the live
/// performance automatically fails (no live card succeeds on resolution).
mod helpers;
use helpers::*;

/// Deploy ToteMari, check draw + restriction set.
#[test]
fn tote_mari_q68_debut_draws_and_sets_cannot_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let totemari = game.id("PL!HS-bp2-014-N");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("LL-bp5-001-L");

    // Put cards in deck so draw_card works
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }

    game.state.player1.hand.cards.push(totemari);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);

    game.give_energy(4);
    game.play_to_stage(totemari, rabuka_engine::zones::MemberArea::LeftSide);

    // Debut ability auto-triggers: draw 1, set cannot_live
    while game.has_pending_choice() { game.select_indices(&[]); }

    // Hand: 3 initial - 1 played + 1 drawn = 3
    assert_eq!(game.state.player1.hand.cards.len(), 3,
        "Debut drew 1 card");

    // cannot_live restriction should be set
    assert!(game.state.is_action_prohibited("cannot_live"),
        "cannot_live restriction active after debut");
}

/// During live phase, cannot_live does NOT block setting live cards (Q68).
/// The player can still put a live card face-down. The failure happens at performance.
#[test]
fn tote_mari_q68_can_still_set_live_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let totemari = game.id("PL!HS-bp2-014-N");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("LL-bp5-001-L");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }

    game.state.player1.hand.cards.push(totemari);
    game.state.player1.hand.cards.push(live);

    game.give_energy(4);
    game.play_to_stage(totemari, rabuka_engine::zones::MemberArea::LeftSide);

    while game.has_pending_choice() { game.select_indices(&[]); }

    // Now advance to live phase
    for _ in 0..5 { game.pass(); }

    // Live card set phase: try to set a live card
    // According to Q68, this should be allowed despite cannot_live
    game.set_live_card(live);

    // Setting succeeded — that's the expected behavior per Q68
    assert!(game.state.player1.live_card_zone.cards.contains(&live),
        "Live card can be set despite cannot_live");
}
