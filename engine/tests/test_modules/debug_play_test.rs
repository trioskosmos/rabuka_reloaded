use crate::helpers::*;

#[test]
fn debug_play_to_stage_after_phase_advance() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chisato = game.id("PL!SP-pb1-014-N");
    let filler = game.id("PL!-sd1-010-SD");

    // Fill decks
    for _ in 0..10 {
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player2.main_deck.cards.push(f);
    }

    game.add_to_hand(chisato);
    game.add_to_hand(filler);

    eprintln!("=== BEFORE ADVANCE ===");
    eprintln!("phase = {}", game.state.current_phase);
    eprintln!("turn_phase = {:?}", game.state.current_turn_phase);
    eprintln!("active_player_id = {}", game.state.active_player().id);
    eprintln!("p1_hand = {:?}", game.state.player1.hand.cards);
    eprintln!("p2_hand = {:?}", game.state.player2.hand.cards);

    for i in 0..4 {
        game.pass();
        eprintln!("=== AFTER PASS {} ===", i + 1);
        eprintln!("phase = {}", game.state.current_phase);
        eprintln!("active_player_id = {}", game.state.active_player().id);
        eprintln!("p1_hand = {:?}", game.state.player1.hand.cards);
        eprintln!("p2_hand = {:?}", game.state.player2.hand.cards);
    }

    eprintln!("=== BEFORE play_to_stage ===");
    eprintln!("active_player_id = {}", game.state.active_player().id);
    eprintln!("p1_hand = {:?}", game.state.player1.hand.cards);
    eprintln!("p2_hand = {:?}", game.state.player2.hand.cards);

    let p2_card = game.state.player2.hand.cards[0];
    for _ in 0..4 {
        let energy_card = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(energy_card);
    }
    game.state.player2.energy_zone.active_energy_count += 4;
    game.play_to_stage(p2_card, rabuka_engine::zones::MemberArea::Center);
    eprintln!(
        "play succeeded, p2 stage: {:?}",
        game.state.player2.stage.stage
    );
}
