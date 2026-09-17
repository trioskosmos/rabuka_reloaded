use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD";

fn setup_pl_pr_004_pr_heart_filtered_recovery(
    game: &mut TestGame,
    lives: &[&str],
) -> (i16, Vec<i16>) {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!-PR-004-PR");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    game.add_to_hand(game.new_id(FILLER));
    let mut live_ids = Vec::new();
    for l in lives {
        let cid = game.new_id(l);
        live_ids.push(cid);
        game.state.player1.waitroom.cards.push(cid);
    }
    (me, live_ids)
}

#[test]
fn pl_pr_004_pr_recovers_live_requiring_at_least_three_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, live_ids) =
        setup_pl_pr_004_pr_heart_filtered_recovery(&mut game, &["PL!N-sd1-028-SD"]);
    let live = live_ids[0];

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "live with heart01>=3 requirement retrieved"
    );
}

#[test]
fn pl_pr_004_pr_does_not_recover_live_below_three_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, live_ids) =
        setup_pl_pr_004_pr_heart_filtered_recovery(&mut game, &["PL!HS-bp2-020-L"]);
    let live = live_ids[0];

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "live with heart01<3 must NOT be retrievable"
    );
}
