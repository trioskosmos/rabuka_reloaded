/// Q148: ミはμ'sicのミ (PL!-bp3-023-L) Live Start:
/// If total blade of stage members >= 10, required hearts decrease by heart00×2.
/// Ruling: Waited members' blade IS included in the total.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Two blade=7 members active → total 14 ≥ 10 → ability fires.
#[test]
fn q148_blade_total_active_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-bp3-023-L");
    let m_a = game.id("PL!N-pb1-007-R"); // 優木せつ菜, blade=7
    let m_b = game.id("PL!-bp6-003-R＋"); // 南ことり, blade=7

    game.state.player1.stage.stage = [m_a, m_b, -1];
    game.state.player1.hand.cards.push(live);

    for _ in 0..20 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }
    for _ in 0..20 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.try_select_indices(&[0]).unwrap_or_default();
    }

    let need_mod = game
        .state
        .mods
        .get_need_heart_modifier(live, HeartColor::Heart00);
    assert_eq!(need_mod, -2, "Q148: blade 7+7=14 ≥ 10 → need_heart00 -2");
}

/// One blade=7 waited + one blade=7 active. Active only = 7 < 10.
/// Q148: waited blade counts → total 14 ≥ 10 → fires.
#[test]
fn q148_blade_total_includes_waited_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-bp3-023-L");
    let m_a = game.id("PL!N-pb1-007-R"); // blade=7
    let m_b = game.id("PL!-bp6-003-R＋"); // blade=7

    game.state.player1.stage.stage = [m_a, m_b, -1];
    game.state.mods.add_orientation_modifier(m_a, "wait");

    game.state.player1.hand.cards.push(live);

    for _ in 0..20 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }
    for _ in 0..20 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.try_select_indices(&[0]).unwrap_or_default();
    }

    let need_mod = game
        .state
        .mods
        .get_need_heart_modifier(live, HeartColor::Heart00);
    assert_eq!(
        need_mod, -2,
        "Q148: waited blade counts → total 14 ≥ 10 → need_heart00 -2"
    );
}

/// Active blade total = 7 < 10 → ability should NOT fire.
#[test]
fn q148_blade_total_below_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-bp3-023-L");
    let m_a = game.id("PL!N-pb1-007-R"); // blade=7

    game.state.player1.stage.stage = [m_a, -1, -1];
    game.state.player1.hand.cards.push(live);

    for _ in 0..20 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }
    for _ in 0..20 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.try_select_indices(&[0]).unwrap_or_default();
    }

    let need_mod = game
        .state
        .mods
        .get_need_heart_modifier(live, HeartColor::Heart00);
    assert_eq!(need_mod, 0, "Q148: blade 7 < 10 → no heart modifier");
}
