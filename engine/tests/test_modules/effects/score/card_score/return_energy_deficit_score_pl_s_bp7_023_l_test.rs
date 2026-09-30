use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const LIVE_START: &str = "ライブ開始時";
const FILLER: &str = "PL!-sd1-010-SD";

fn setup_pl_s_bp7_023_l_energy_return_score(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);

    let live = game.id("PL!S-bp7-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);

    let chika_a = game.new_id("PL!S-sd1-001-SD");
    let chika_b = game.new_id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage[0] = chika_a;
    game.state.player1.stage.stage[1] = chika_b;

    let e = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e);
    game.state.player1.energy_zone.add_active(1);
    live
}

fn give_pl_s_bp7_023_l_opponent_active_energy(game: &mut TestGame, n: usize) {
    for _ in 0..n {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(n as u8);
}

#[test]
fn pl_s_bp7_023_l_return_energy_opponent_one_ahead_grants_one_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_pl_s_bp7_023_l_energy_return_score(&mut game);
    give_pl_s_bp7_023_l_opponent_active_energy(&mut game, 1);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "opponent exactly 1 energy ahead after the return -> live +1"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "our energy was returned to the energy deck"
    );
}

#[test]
fn pl_s_bp7_023_l_return_energy_opponent_two_ahead_grants_two_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_pl_s_bp7_023_l_energy_return_score(&mut game);
    give_pl_s_bp7_023_l_opponent_active_energy(&mut game, 2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "opponent 2+ energies ahead -> alternative +2 instead"
    );
}

#[test]
fn pl_s_bp7_023_l_tied_energy_after_return_grants_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_pl_s_bp7_023_l_energy_return_score(&mut game);
    give_pl_s_bp7_023_l_opponent_active_energy(&mut game, 0);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "tied energy counts -> no score bonus"
    );
}

/// 「『Aqours』のメンバーが2人以上いる場合」 in isolation — the gate never binds in the
/// tests above.
///
/// `setup_pl_s_bp7_023_l_energy_return_score` always stages 高海千歌 + 松浦果南, both
/// ラブライブ！サンシャイン!!, so the gate is satisfied in every existing case. An
/// implementation that ignored the group clause entirely — scoring purely off the energy
/// comparison — would pass all three, because none of them ever has fewer than two Aqours
/// members on stage.
///
/// This stages **one** Aqours member plus one ラブライブ！(μ's) member and reproduces the
/// +2 energy situation verbatim. The energy branch is identical to
/// `pl_s_bp7_023_l_return_energy_opponent_two_ahead_grants_two_score`; the only difference is
/// the gate. Expect **0**, so a defect that dropped the gate would give 2 and fail here only.
#[test]
fn pl_s_bp7_023_l_one_aqours_member_fails_the_two_member_gate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!S-bp7-023-L");
    game.assert_card_identity(live, "PL!S-bp7-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);

    // One サンシャイン!! (Aqours) and one ラブライブ！ (not Aqours).
    let aqours = game.new_id("PL!S-sd1-001-SD"); // 高海千歌 — サンシャイン!!
    let outsider = game.new_id("PL!-sd1-010-SD"); // 高坂 穂乃果 — ラブライブ！, NOT Aqours
    game.state.player1.stage.stage[0] = aqours;
    game.state.player1.stage.stage[1] = outsider;

    // Pin the premise from card data: exactly one of the two is in the Aqours group, so the
    // 「2人以上」 gate fails on group membership and not on head count.
    let aq_card = game.db.get_card(aqours).unwrap();
    let out_card = game.db.get_card(outsider).unwrap();
    assert!(
        format!("{:?}", aq_card.series).contains("サンシャイン"),
        "precondition: 高海千歌 must be a サンシャイン!! (Aqours) member"
    );
    assert!(
        !format!("{:?}", out_card.series).contains("サンシャイン"),
        "precondition: 高坂 穂乃果 must NOT be サンシャイン!!, or the gate would hold and this \
         would duplicate the +2 test (series={:?})",
        out_card.series
    );

    let e = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e);
    game.state.player1.energy_zone.add_active(1);

    // Same energy situation as the +2 test: opponent two ahead after our energy returns.
    give_pl_s_bp7_023_l_opponent_active_energy(&mut game, 2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "自分のステージに『Aqours』のメンバーが2人以上いる場合 — only ONE of these two members is \
         『Aqours』, so the whole clause is inert and no score bonus applies even though the \
         energy branch alone would have given +2. A modifier of 2 here means the group gate is \
         not being enforced."
    );
}
