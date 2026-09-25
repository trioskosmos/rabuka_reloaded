use crate::helpers::*;

#[test]
fn live_start_other_group_blade_leaves_lone_source_unboosted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!N-sd1-001-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    let fid2 = game.id_ref("PL!-sd1-010-SD");
    fill_decks(&mut game, fid2);
    game.give_energy(15);

    for _ in 0..7 {
        game.pass();
        drain_skippable_choices(&mut game);
    }

    let blade = game.state.mods.get_blade_modifier(member);
    assert_eq!(
        blade, 0,
        "no other 虹ヶ咲 on stage → no blade boost"
    );
}

fn drain_skippable_choices(game: &mut TestGame) {
    use rabuka_engine::ability::types::Choice;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}
