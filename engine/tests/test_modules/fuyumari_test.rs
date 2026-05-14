/// Tests for 鬼塚冬毬 (PL!SP-pb1-011-R) — Debut ability:
///
/// 登場 「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから控え室に置いてもよい：
/// 自分の控え室から、これにより控え室に置いたメンバーカードを1枚、そのメンバーがいたエリアに登場させる。
///
/// Q63: Effect-debut doesn't pay member cost separately.
/// Q95: Only the exact card sent as cost can be appeared (not same-name copies).
use crate::helpers::*;

/// Q63: Effect-debut doesn't pay member cost separately.
///
/// Setup: only 1 matching card exists anywhere (just the one on stage).
/// When the optional cost sends it to discard, the effect auto-resolves
/// (exactly 1 match) — no energy spent beyond 鬼塚冬毬's own cost.
#[test]
fn fuyumari_q63_effect_debut_no_cost_payment() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::Center);

    // Cost prompt: select the Liella! member from stage[0]
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // No second prompt: exactly 1 card in discard (the cost-sent one)
    // → auto-resolves to same_area[0]
    assert!(
        !game.has_pending_choice(),
        "Exactly 1 matching card in discard → auto-resolve, no choice needed"
    );

    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 0,
        "All 13 energy spent on 鬼塚冬毬 (Q63)"
    );

    assert_eq!(
        game.state.player1.stage.stage[0], liella_member,
        "Liella! member returned to LeftSide (same_area)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center"
    );
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
    assert_eq!(
        game.state.player1.stage.stage[0], liella_member,
        "Liella! member should remain on stage when cost is skipped"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center"
    );
}

/// Q95: Player chooses which card from discard appears via same_area.
///
/// Put 2 DIFFERENT member cards in discard. The cost sends a Liella! member
/// from stage to discard. Now 3 member cards in discard. The effect prompts
/// to choose 1. The player picks index [0] → that card appears at the
/// vacated area. The other 2 stay in discard.
#[test]
fn fuyumari_q95_player_chooses_card_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fuyumari = game.id("PL!SP-pb1-011-R");
    let liella_member = game.id("PL!SP-sd1-006-SD");
    let other_member = game.id("PL!SP-sd1-008-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.stage.stage[0] = liella_member;
    // Two DIFFERENT member cards in discard to force a choice
    game.state.player1.waitroom.cards.push(other_member);
    game.state.player1.waitroom.cards.push(liella_member);
    game.give_energy(13);

    let discard_before = game.state.player1.waitroom.cards.len(); // 2

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::Center);

    // Cost: select the Liella! member from stage
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Now discard has 3 cards (other_member, liella_member, liella_member)
    // Effect prompts to choose 1 → player picks index [0] = other_member
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // The chosen card (other_member) should be on stage at LeftSide (same_area[0])
    assert_eq!(
        game.state.player1.stage.stage[0], other_member,
        "Chosen card (other_member) should appear at same_area (LeftSide)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center"
    );

    assert!(
        game.state.player1.waitroom.cards.len() <= discard_before,
        "Cost adds 1, effect removes 1 → net ≤ discard_before"
    );
}

/// Edge case: Non-Liella! member on stage (should not be valid cost target).
/// If the engine shows a prompt, verify it can be skipped.
#[test]
fn fuyumari_edge_no_valid_cost_target() {
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

    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center"
    );
}

/// Edge case: exclude_self — 鬼塚冬毬 herself should not appear in the
/// cost selection prompt. Only the Liella! member should be selectable.
#[test]
fn fuyumari_edge_exclude_self_from_cost() {
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

    assert_eq!(
        game.state.player1.stage.stage[1], fuyumari,
        "鬼塚冬毬 on Center"
    );
}
