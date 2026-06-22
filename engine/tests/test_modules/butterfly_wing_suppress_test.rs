use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn setup_p1_deck(game: &mut TestGame, live_ids: &[i16]) {
    let filler = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for (i, &id) in live_ids.iter().enumerate() {
        game.state.player1.main_deck.cards.insert(1 + i, id);
    }
}

/// Butterfly Wing suppresses live_start abilities of stage members.
#[test]
fn butterfly_wing_suppresses_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let butterfly = game.id("PL!SP-pb2-046-L");
    let mei = game.id("PL!SP-pb1-007-P＋"); // live_start: activate 2 energy
    let filler = game.id("PL!-sd1-010-SD");

    // Put members on stage (3 positions needed)
    game.add_to_stage(MemberArea::LeftSide, filler);
    game.add_to_stage(MemberArea::Center, mei);
    game.add_to_stage(MemberArea::RightSide, filler);

    // Record energy before live_start
    let energy_before = game.state.player1.energy_zone.active_energy_count;

    // Set up deck, advance to live card set, set butterfly wing
    setup_p1_deck(&mut game, &[]);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(butterfly);
    game.set_live_card(butterfly);

    // Advance to live_start phase
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Mei's live_start should be suppressed — energy should NOT have increased
    let energy_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        energy_after, energy_before,
        "Butterfly Wing should suppress live_start: energy should not increase (was {}, got {})",
        energy_before, energy_after
    );
}
