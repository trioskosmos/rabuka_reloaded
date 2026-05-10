use std::path::Path;

mod helpers;
use helpers::*;

fn main() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let member = game.id("PL!-sd1-010-SD");
    
    game.state.player1.stage.stage[1] = honoka;
    game.state.player1.hand.cards.push(member);
    game.state.player1.hand.cards.push(member);

    game.give_energy(13);
    
    // Activate ability
    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::UseAbility,
        Some(honoka), None, None, None,
    ).expect("activate");
    
    // Check what choice is pending
    if let Some(ref choice) = game.state.pending_choice {
        println!("Pending choice: {:?}", choice);
    }
}
