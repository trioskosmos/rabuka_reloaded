use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::zones::MemberArea;

#[test]
fn live_phase_group_member_wait_optional_discard_removes_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shioriko = game.id("PL!N-bp7-022-N"); // 虹ヶ咲 member
    let waited = game.id(NIJI_MEMBER); // 上原歩夢, 虹ヶ咲 member
    game.state.player1.hand.cards.push(game.id(FILLER)); // optional discard
    game.state.player1.stage.set_area(MemberArea::Center, waited);
    game.state.player1.stage.set_area(MemberArea::LeftSide, shioriko);

    // Enter the live performance phase and record the wait state change.
    use rabuka_engine::core::types::Phase;
    game.state.current_phase = Phase::FirstAttackerPerformance;
    game.state
        .recently_state_changed
        .push((waited, "active".to_string(), "wait".to_string(), "".to_string()));
    game.state.mods.orientation_modifiers.insert(waited, CardOrientation::Wait);

    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    // The auto offers the optional discard → accept → the waited member activates.
    resolve_auto_choices_accepting_optionals(&mut game);

    assert_ne!(
        game.state.mods.orientation_modifiers.get(&waited),
        Some(&CardOrientation::Wait),
        "三船栞子 ab#0 should activate a waited 虹ヶ咲 member during the live phase"
    );
}
