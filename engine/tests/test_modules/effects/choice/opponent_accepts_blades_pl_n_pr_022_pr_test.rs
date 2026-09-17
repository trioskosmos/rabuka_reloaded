use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_n_pr_022_pr_opponent_accepts_choice_gains_blade_only_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let emma = game.id("PL!N-PR-022-PR");
    let filler = game.id("PL!-sd1-010-SD");
    game.state
        .player2
        .stage
        .set_area(MemberArea::Center, filler);
    game.state.player1.hand.cards.push(emma);
    game.give_energy(10);
    game.play_to_stage(emma, MemberArea::LeftSide);
    assert!(
        game.has_pending_choice(),
        "Emma's Debut ability should be waiting for choice"
    );
    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue should have an entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Choice player should be p2 (opponent)"
    );
    game.select_option(0);
    let p2_center_blade = game.state.mods.get_blade_modifier(filler);
    assert_eq!(
        p2_center_blade, 1,
        "Player 2's member should have gained 1 blade"
    );
    let p1_emma_blade = game.state.mods.get_blade_modifier(emma);
    assert_eq!(
        p1_emma_blade, 0,
        "Player 1's Emma should NOT have gained a blade"
    );
}
