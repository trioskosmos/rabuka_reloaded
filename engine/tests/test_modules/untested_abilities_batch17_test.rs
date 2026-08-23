/// Untested-abilities batch 17 — depth=none gaps:
/// - PL!N-bp7-024-N (登場): heart01 while 3 R3BIRTH members on own stage
/// - PL!SP-PR-022-PR (常時): heart02+heart03 at 6 combined staged members
///   (second card exercising the aggregate-total cross-player fix)
/// - PL!-bp6-009-R (常時 center): live total +1 while both side areas hold
///   members with original blade 2
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(trig))
            .unwrap_or_else(|| panic!("card {} lacks a '{trig}' ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!N-bp7-024-N (登場):
// 「自分のステージに『R3BIRTH』のメンバーが3人いる場合、ライブ終了時まで、heart01を得る。」
// ====================================================================

#[test]
fn bp7024_debut_heart01_with_three_r3birth() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp7-024-N");
    let r1 = game.id("PL!N-bp1-023-PRproteinbar");
    let r2 = game.id("PL!N-bp1-024-PR");
    let r3 = game.id("PL!N-PR-012-PR");
    game.state.player1.stage.stage = [me, r1, r2];
    game.state.player2.stage.stage[0] = r3;

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    const H01: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart01;
    assert!(
        game.state.mods.get_heart_modifier(me, H01) > 0,
        "3 R3BIRTH members on stage -> heart01 until live end"
    );
}

#[test]
fn bp7024_debut_no_heart01_with_two_r3birth() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp7-024-N"); // itself R3BIRTH
    let r1 = game.id("PL!N-bp1-023-PRproteinbar");
    let other = game.new_id(FILLER); // μ's — not R3BIRTH
    game.state.player1.stage.stage = [me, r1, other];

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    const H01: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart01;
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H01),
        0,
        "only 2 R3BIRTH members -> no heart01"
    );
}

// ====================================================================
// PL!SP-PR-022-PR (常時):
// 「自分と相手のステージにメンバーが合計6人いるかぎり、heart02+heart03を得る。」
// Cross-player member count — same aggregate-total shape as PR-042.
// ====================================================================

#[test]
fn spr022_constant_hearts_at_six_combined_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-PR-022-PR");
    let f1 = game.new_id(FILLER);
    let f2 = game.new_id(FILLER);
    game.state.player1.stage.stage = [me, f1, f2];
    for i in 0..3usize {
        let m = game.new_id(FILLER);
        game.state.player2.stage.stage[i] = m;
    }

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    const H03: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart03;
    assert!(
        game.state.mods.get_heart_modifier(me, H02) > 0,
        "6 combined members -> heart02"
    );
    assert!(
        game.state.mods.get_heart_modifier(me, H03) > 0,
        "6 combined members -> heart03"
    );
}

// ====================================================================
// PL!-bp6-009-R (常時 センター):
// 「右サイドエリアと左サイドエリアに、元々のブレードの数が2つのメンバーが
//   いるかぎり、ライブの合計スコアを＋１する。」
// ====================================================================

#[test]
fn bp6009_center_side_blade2_members_live_total_plus1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!-bp6-009-R");
    game.add_to_stage(MemberArea::Center, me);

    // μ's members whose ORIGINAL blade count is exactly 2.
    let l = game.id("PL!-sd1-006-SD");
    let r = game.new_id("PL!-sd1-006-SD");
    game.add_to_stage(MemberArea::LeftSide, l);
    game.add_to_stage(MemberArea::RightSide, r);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "blade-2 members in both side areas -> live total +1"
    );
}
