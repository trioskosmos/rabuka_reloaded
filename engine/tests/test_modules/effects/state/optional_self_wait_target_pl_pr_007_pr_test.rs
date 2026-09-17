use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_pr_007_pr_optional_self_wait_keeps_selected_opponent_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let nozomi = game.id("PL!-PR-007-PR");
    let target1 = game.id("PL!-sd1-010-SD");
    let target2 = game.id("PL!-sd1-011-SD");
    game.state
        .player2
        .stage
        .set_area(MemberArea::Center, target1);
    game.state
        .player2
        .stage
        .set_area(MemberArea::LeftSide, target2);
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(10);
    game.play_to_stage(nozomi, MemberArea::RightSide);
    assert!(
        game.has_pending_choice(),
        "Nozomi should wait for optional cost choice"
    );
    game.select_option(1);
    let nozomi_orientation = game.state.mods.get_orientation_modifier(nozomi);
    assert_eq!(
        nozomi_orientation,
        Some("wait"),
        "Nozomi should be in wait state"
    );
    assert!(
        game.has_pending_choice(),
        "Nozomi should wait for effect target choice"
    );
    game.select_indices(&[1]);
    let p2_center = game.state.player2.stage.get_area(MemberArea::Center);
    assert_eq!(
        p2_center,
        Some(target1),
        "Target member should still be on stage at Center"
    );
    let target1_orientation = game.state.mods.get_orientation_modifier(target1);
    assert_eq!(
        target1_orientation,
        Some("wait"),
        "Target member should be in wait state"
    );
}
