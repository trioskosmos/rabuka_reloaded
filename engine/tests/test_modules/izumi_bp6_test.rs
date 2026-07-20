/// Tests for PL!HS-bp6-008 桂城 泉 (Izumi Keijou) — Debut ability:
///
/// Q257: このメンバーが登場したとき、該当のライブカードが控え室にありませんでした。
///       このとき、このメンバーをウェイトにする必要はありますか？
///       → はい、必ずウェイトになります。
///
/// Ability text:
///   登場：このメンバーをウェイトにする。その後、自分の控え室からスコア4以下の
///   『蓮ノ空』のライブカードを1枚手札に加える。
///
/// Step 1 (mandatory): this member → wait (orientation modifier)
/// Step 2 (conditional): from waitroom, score ≤ 4 蓮ノ空 live_card → hand (1 card)
///   − Only runs if there is at least one matching card in waitroom.
///   − If multiple, player chooses.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn izumi_energy_needed() -> usize {
    12
}

/// Case A: Waitroom empty → card becomes wait, no second effect, no crash.
#[test]
fn izumi_bp6_q257_waitroom_empty_becomes_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-bp6-008-R");

    game.add_to_hand(izumi);
    game.state.player1.waitroom.cards.clear();
    game.give_energy(izumi_energy_needed());

    game.play_to_stage(izumi, MemberArea::Center);

    assert_eq!(
        game.state.mods.get_orientation_modifier(izumi),
        Some("wait"),
        "Q257 Case A: Izumi must become wait even when waitroom is empty"
    );

    assert!(
        !game.has_pending_choice(),
        "Q257 Case A: no choice should appear when waitroom has no matching live card"
    );

    assert!(
        game.state.player1.stage.stage.contains(&izumi),
        "Q257 Case A: Izumi should remain on stage in wait state"
    );
}

/// Case B: Waitroom has exactly 1 matching card → Izumi becomes wait,
/// and that live card moves to hand automatically.
#[test]
fn izumi_bp6_q257_single_match_moves_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-bp6-008-R");
    // Use a known 蓮ノ空 live from BP6.
    let matching_live = game.id("PL!HS-bp6-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(izumi);
    game.add_to_discard(matching_live);
    game.add_to_discard(filler);
    game.give_energy(izumi_energy_needed());

    game.play_to_stage(izumi, MemberArea::Center);

    assert_eq!(
        game.state.mods.get_orientation_modifier(izumi),
        Some("wait"),
        "Q257 Case B: Izumi must become wait before step 2"
    );

    assert!(
        game.state.player1.hand.cards.contains(&matching_live),
        "Q257 Case B: matching 蓮ノ空 live (score≤4) should be in hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&filler),
        "Q257 Case B: non-matching discard card should remain in waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&matching_live),
        "Q257 Case B: matching live should no longer be in waitroom"
    );
}

/// Case C: Waitroom has only non-matching cards → Izumi becomes wait,
/// no second effect runs, no crash.
#[test]
fn izumi_bp6_q257_non_matching_only_no_second_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-bp6-008-R");

    game.add_to_hand(izumi);
    game.add_to_discard(game.id("PL!S-PR-022-PR"));
    game.give_energy(izumi_energy_needed());

    game.play_to_stage(izumi, MemberArea::Center);

    assert_eq!(
        game.state.mods.get_orientation_modifier(izumi),
        Some("wait"),
        "Q257 Case C: Izumi must still become wait"
    );

    assert!(
        !game.has_pending_choice(),
        "Q257 Case C: no choice should appear when no matching cards exist"
    );
}

/// Case D: Waitroom has 2 matching cards → player is prompted to choose 1.
#[test]
fn izumi_bp6_q257_multiple_matches_prompts_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-bp6-008-R");
    let match1 = game.id("PL!HS-bp6-025-L");
    let match2 = game.id("PL!HS-bp6-026-L");

    game.add_to_hand(izumi);
    game.add_to_discard(match1);
    game.add_to_discard(match2);
    game.give_energy(izumi_energy_needed());

    game.play_to_stage(izumi, MemberArea::Center);

    assert_eq!(
        game.state.mods.get_orientation_modifier(izumi),
        Some("wait"),
        "Q257 Case D: Izumi must become wait before step 2"
    );

    assert!(
        game.has_pending_choice(),
        "Q257 Case D: expect choice when 2 matching cards are available"
    );

    game.select_indices(&[0]);
    assert!(
        game.state.player1.hand.cards.contains(&match1)
            || game.state.player1.hand.cards.contains(&match2),
        "Q257 Case D: exactly one of the two matching lives should be in hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&match1)
            || game.state.player1.waitroom.cards.contains(&match2),
        "Q257 Case D: the unchosen matching live should remain in waitroom"
    );
}
