use crate::helpers::*;

#[test]
fn shizuku_pl_n_sd2_003_sd2_hand_cost_reduced_with_nijigasaki_live_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-sd2-003-SD2");
    let niji_live = game.id("PL!N-sd1-025-SD");
    game.add_to_hand(shizuku);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(niji_live);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(shizuku),
        -2,
        "虹ヶ咲 live card in success zone -> hand cost −2"
    );
}

#[test]
fn shizuku_pl_n_sd2_003_sd2_hand_cost_full_without_success_zone_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-sd2-003-SD2");
    game.add_to_hand(shizuku);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(shizuku),
        0,
        "empty success zone -> full hand cost"
    );
}

#[test]
fn shizuku_pl_n_sd2_003_sd2_non_nijigasaki_live_does_not_reduce_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-sd2-003-SD2");
    let aqours_live = game.id("PL!-sd1-020-SD");
    game.add_to_hand(shizuku);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(aqours_live);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(shizuku),
        0,
        "non-虹ヶ咲 live card in success zone -> full hand cost"
    );
}
