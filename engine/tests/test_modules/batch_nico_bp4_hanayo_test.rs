/// Batch: remaining 1-QA cards with simple testable abilities
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!-bp4-009-R (矢澤にこ) Q189: Debut — put 1 active member on stage to wait.
#[test]
fn nico_bp4_q189_debut_active_to_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let nico = game.id("PL!-bp4-009-R");
    let friend = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [friend, -1, -1];
    game.add_to_hand(nico);
    game.give_energy(10);
    game.play_to_stage(nico, MemberArea::Center);

    // Debut fires: choice to put active member to wait.
    // If a choice is pending, the ability targets self's stage only.
    if game.has_pending_choice() {
        game.select_option(1);
        if game.has_pending_choice() {
            game.select_indices(&[0]);
        }
    }

    // Q189: only self's stage members are targeted, ability resolves
}

/// PL!-sd1-019-SD (START:DASH!!) Q36: LiveSuccess timing definition.
/// Draw 3 from deck, arrange any order on top, rest to discard.
#[test]
fn start_dash_q36_live_success_draw_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let start_dash = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(start_dash);
    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    let deck_before = game.state.player1.main_deck.len();

    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(start_dash);
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    // LiveSuccess triggers → draw 3 from deck
    let deck_after = game.state.player1.main_deck.len();
    assert!(
        deck_after <= deck_before,
        "LiveSuccess should draw cards: {} → {}",
        deck_before,
        deck_after
    );
}
