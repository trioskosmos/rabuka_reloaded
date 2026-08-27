use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_success(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

#[test]
fn daydream_mermaid_choice_appears_and_selects_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let mermaid = game.id("PL!N-bp4-030-L");
    let filler = game.id("PL!-sd1-010-SD");
    let h05 = game.id("PL!S-sd1-003-SD");
    let h06 = game.id("PL!-PR-015-PR");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [h06, h05, -1];
    game.state.player1.hand.cards.push(mermaid);

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player1.energy_deck.cards.push(energy);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(mermaid);
    advance_to_live_success(&mut game);

    assert!(game.has_pending_choice(), "Choice should appear");

    let energy_before = game.state.player1.energy_zone.cards.len();
    game.select_option(0);
    assert!(
        game.state.player1.energy_zone.cards.len() > energy_before,
        "Energy should be placed from energy deck"
    );
}

#[test]
fn daydream_mermaid_choice_selects_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let mermaid = game.id("PL!N-bp4-030-L");
    let filler = game.id("PL!-sd1-010-SD");
    let h05 = game.id("PL!S-sd1-003-SD");
    let h06 = game.id("PL!-PR-015-PR");

    game.state.player1.stage.stage = [h06, h05, -1];
    game.state.player1.hand.cards.push(mermaid);

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(mermaid);
    advance_to_live_success(&mut game);

    assert!(game.has_pending_choice(), "Choice should appear");

    let hand_before = game.state.player1.hand.cards.len();
    game.select_option(1);

    // move_cards finds 8 member cards in discard but count=1 → prompts card selection
    assert!(game.has_pending_choice(), "Card selection should appear");
    game.select_indices(&[0]);

    assert!(
        game.state.player1.hand.cards.len() > hand_before,
        "Member should be recovered to hand"
    );
}
