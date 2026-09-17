use crate::helpers::*;
use rabuka_engine::game::game_setup;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Reproduce: play a kidou card to stage, check actions are not softlocked.
/// The card PL!-sd1-005-SD (星空 凛) has a kidou ability:
///   Move this member from stage to discard: add 1 live card from discard to hand.
#[test]
fn kidou_card_play_to_stage_not_softlocked() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kidou_card = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(kidou_card);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);
    game.give_energy(25);

    // Play the kidou card to stage
    game.play_to_stage(kidou_card, MemberArea::Center);

    // Resolve any pending choices from debut/auto triggers
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Now check: generate actions. We should NOT be softlocked.
    let actions = game_setup::generate_possible_actions(&game.state);
    let has_pass = actions
        .iter()
        .any(|a| a.action_type == game_setup::ActionType::Pass);
    let has_use_ability = actions
        .iter()
        .any(|a| a.action_type == game_setup::ActionType::UseAbility);
    let has_play_member = actions
        .iter()
        .any(|a| a.action_type == game_setup::ActionType::PlayMemberToStage);

    eprintln!("Actions after playing kidou card to stage:");
    for (i, a) in actions.iter().enumerate() {
        eprintln!(
            "  [{}] {:?} {}",
            i,
            a.action_type,
            a.description.lines().next().unwrap_or("")
        );
    }

    assert!(has_pass, "Should have PASS action");
    assert!(
        has_use_ability || has_play_member,
        "Should have UseAbility or PlayMemberToStage actions, not just PASS. Actions: {}",
        actions.len()
    );

    // The game should NOT be stuck in a pending choice
    assert!(
        !game.has_pending_choice(),
        "Should not have pending_choice after settling"
    );
}
