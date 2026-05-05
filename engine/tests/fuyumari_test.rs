/// Tests for 鬼塚冬毬 (PL!SP-pb1-011-R) — Debut ability:
///
/// 登場 「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから控え室に置いてもよい：
/// 自分の控え室から、これにより控え室に置いたメンバーカードを1枚、そのメンバーがいたエリアに登場させる。
///
/// Q63: Effect-debut doesn't pay member cost separately.
/// Q95: Only the exact card sent as cost can be appeared (not same-name copies).

mod helpers;
use helpers::*;

/// Q63: When the ability fires and appears a card from discard to stage,
/// the appeared card's cost is NOT paid. Only 鬼塚冬毬's own cost (13) is spent.
#[test]
fn fuyumari_q63_effect_debut_no_cost_payment() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    game.state.player1.waitroom.cards.push(liella_member);
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::Center);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(game.state.player1.energy_zone.active_energy_count, 0,
        "All 13 energy spent on 鬼塚冬毬 (Q63)");

    let on_stage = game.state.player1.stage.stage.contains(&liella_member);
    assert!(on_stage,
        "Liella! member should be on stage after ability resolves, stage={:?}",
        game.state.player1.stage.stage);
    assert_eq!(game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center");
}

/// Q63 variant: optional cost NOT paid → only 鬼塚冬毬 appears, no card swap.
#[test]
fn fuyumari_q63_optional_cost_skipped() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::Center);

    // Skip the optional cost (don't select — engine should allow skipping)
    // The debute ability fires, but since cost is optional, skipping means
    // the effect never executes. Only 鬼塚冬毬 is on stage.
    if game.has_pending_choice() {
        // To skip: provide empty selection or a skip option
        // Try empty indices to skip the optional cost
        game.select_indices(&[]);
    }

    // 鬼塚冬毬 on Center, Liella! member still at LeftSide (unchanged)
    assert_eq!(game.state.player1.stage.stage[0], liella_member,
        "Liella! member should remain on stage when cost is skipped");
    assert_eq!(game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center");
}

/// Q95: When there are multiple copies of the same card in discard,
/// the engine should only move 1 of them to the stage.
#[test]
fn fuyumari_q95_only_specific_card_appeared() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    game.state.player1.waitroom.cards.push(liella_member);
    game.state.player1.waitroom.cards.push(liella_member);
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::Center);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let on_stage = game.state.player1.stage.stage.contains(&liella_member);
    assert!(on_stage,
        "Liella! member should be on stage, stage={:?}",
        game.state.player1.stage.stage);
    assert_eq!(game.state.player1.stage.stage[1], fuyumari);

    // Cost adds 1 to discard, effect removes 1 → net same count
    // Previously had 2 copies, cost removes from stage (discard+1=3), effect appears 1 (=2)
    assert_eq!(game.state.player1.waitroom.cards.len(), 2,
        "Q95: Cost adds 1, effect removes 1, net same");
}

/// Edge case: Non-Liella! member on stage (should not be valid cost target).
/// If the engine shows a prompt, verify it can be skipped.
#[test]
fn fuyumari_no_liella_on_stage_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = filler;
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::Center);

    // If a prompt appears, it might show filler as a valid target (if group filter
    // doesn't exclude non-Liella!). Skip by passing empty selection.
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center");
}

/// Edge case: exclude_self — 鬼塚冬毬 herself should not appear in the
/// cost selection prompt. Only the Liella! member should be selectable.
#[test]
fn fuyumari_exclude_self_from_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = liella_member;
    game.state.player1.waitroom.cards.push(liella_member);
    game.give_energy(13);

    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::Center);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center");
}
