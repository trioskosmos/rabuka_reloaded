/// Q146: 園田海未 (PL!-bp3-004-R＋) Debut:
/// 自分のステージにいるメンバー1人につき、カードを1枚引く。その後、手札を1枚控え室に置く。
/// Ruling: 能力を発動メンバーも含めてステージにいるメンバーを数えます。
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// 園田海未 alone on stage → 1 member → draw 1, discard 1 = net 0.
#[test]
fn q146_activating_member_alone_draws_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kumi = game.id("PL!-bp3-004-R＋");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(kumi);
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
    game.give_energy(20);

    let hand_before = game.state.player1.hand.cards.len(); // 2
    game.play_to_stage(kumi, MemberArea::Center);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.try_select_indices(&[0]).unwrap_or_default();
    }

    // Q146: 1 member → draw 1, discard 1. Net: 0.
    // play_to_stage removes kumi from hand (-1), debut net 0.
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Q146: 1 member → draw 1 discard 1 = net 0"
    );
}

/// 園田海未 + 2 fillers on stage → play 園田海未 → 3 members → draw 3, discard 1.
#[test]
fn q146_three_members_draws_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kumi = game.id("PL!-bp3-004-R＋");
    let filler_a = game.id("PL!-sd1-002-SD");
    let filler_b = game.id("PL!-sd1-002-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [filler_a, filler_b, -1];
    game.state.player1.hand.cards.push(kumi);
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
    game.give_energy(20);

    let hand_before = game.state.player1.hand.cards.len(); // 2
    game.play_to_stage(kumi, MemberArea::RightSide);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.try_select_indices(&[0]).unwrap_or_default();
    }

    // Q146: 3 members → draw 3, discard 1. Net: +2. play_to_stage: -1. Total: +1.
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "Q146: 3 members → draw 3 discard 1 = net +1"
    );
}

/// 園田海未 + 1 filler → 2 members → draw 2, discard 1.
#[test]
fn q146_two_members_draws_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kumi = game.id("PL!-bp3-004-R＋");
    let filler = game.id("PL!-sd1-002-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [filler, -1, -1];
    game.state.player1.hand.cards.push(kumi);
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
    game.give_energy(20);

    let hand_before = game.state.player1.hand.cards.len(); // 2
    game.play_to_stage(kumi, MemberArea::Center);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.try_select_indices(&[0]).unwrap_or_default();
    }

    // Q146: 2 members → draw 2, discard 1. Net: +1. play_to_stage: -1. Total: 0.
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after, hand_before,
        "Q146: 2 members → draw 2 discard 1 = net 0"
    );
}

/// 園田海未 already on stage (waited) — stays on stage.
#[test]
fn q146_waited_member_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kumi = game.id("PL!-bp3-004-R＋");
    game.state.player1.stage.stage = [kumi, -1, -1];
    game.state.mods.add_orientation_modifier(kumi, "wait");

    assert!(
        game.state.player1.stage.stage.contains(&kumi),
        "園田海未 should be on stage in waited state"
    );
}

/// Opponent's stage members don't count.
#[test]
fn q146_opponent_members_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kumi = game.id("PL!-bp3-004-R＋");
    let opp_m = game.id("PL!-sd1-002-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(kumi);
    game.state.player1.hand.cards.push(live);
    game.state.player2.stage.stage = [opp_m, opp_m, opp_m];

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
    game.give_energy(20);

    let hand_before = game.state.player1.hand.cards.len(); // 2
    game.play_to_stage(kumi, MemberArea::Center);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.try_select_indices(&[0]).unwrap_or_default();
    }

    // Only 1 own member → draw 1, discard 1. Net: 0. play_to_stage: -1.
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Q146: opponent members not counted → net 0"
    );
}
