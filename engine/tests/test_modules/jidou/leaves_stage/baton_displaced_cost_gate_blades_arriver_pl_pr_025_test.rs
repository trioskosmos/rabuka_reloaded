use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// 「このメンバーがステージから控え室に置かれたとき、このメンバーがコスト9以上の
// メンバーとバトンタッチしていた場合、ライブ終了時まで、そのバトンタッチで登場した
// メンバーはブレードを2つ得る。」
//
// The subject is the REPLACED member (she is the one who reaches the waitroom),
// so the printed cost gate describes the ARRIVING member, and the blade goes to
// that same arriver. Three prints share this exact text.
const SUBJECT_PR_025: &str = "PL!-PR-025-PR"; // 南 ことり, cost 4
const SUBJECT_S_047: &str = "PL!S-PR-047-PR"; // 桜内梨子, cost 4
const SUBJECT_HS_040: &str = "PL!HS-PR-040-PR"; // 安養寺姫芽, cost 4

const FILLER: &str = "PL!-sd1-010-SD"; // cost 4, abilityless
const HI_ARRIVER: &str = "PL!-sd1-014-SD"; // 星空 凛, cost 9, abilityless
const LO_ARRIVER: &str = "PL!-bp3-016-N"; // 東條 希, cost 7, abilityless

/// Baton-touch `arriver_no` over `subject_no` sitting on center stage, then
/// drain whatever the play leaves pending. Returns the three ids involved.
fn baton_over_subject(
    subject_no: &str,
    arriver_no: &str,
) -> (
    TestGame,
    i16, // subject (replaced, now in the waitroom)
    i16, // arriver (now on stage)
    i16, // left filler (untouched bystander)
) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let left = game.new_id(FILLER);
    let right = game.new_id(FILLER);
    let subject = game.id(subject_no);
    game.state.player1.stage.stage = [left, subject, right];

    let arriver = game.new_id(arriver_no);
    game.state.player1.hand.cards.push(arriver);
    game.give_energy(25);

    game.play_to_stage_baton(arriver, MemberArea::Center);

    // Baton plays can queue the replaced member's watcher together with the
    // arrival itself; `drain_choices` answers each prompt by kind (declines
    // only the skippable ones) instead of blind index-0 selection.
    game.drain_choices();
    assert!(
        !game.has_pending_choice(),
        "baton play must drain clean, still pending: {:?}",
        game.get_pending_choice()
    );

    (game, subject, arriver, left)
}

#[test]
fn pr_025_baton_over_cost_nine_arriver_gains_two_blades() {
    let (game, subject, arriver, left) = baton_over_subject(SUBJECT_PR_025, HI_ARRIVER);

    assert_eq!(
        game.state.player1.stage.stage[1], arriver,
        "arriver occupies center after the baton"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&subject),
        "replaced subject must reach the waitroom - that is the trigger"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(arriver),
        2,
        "「そのバトンタッチで登場したメンバー」: the cost-9 arriver gets +2 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(subject),
        0,
        "the replaced member gains nothing herself"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(left),
        0,
        "bystanders gain nothing"
    );
}

#[test]
fn s_pr_047_baton_over_cost_nine_arriver_gains_two_blades() {
    let (game, subject, arriver, _left) = baton_over_subject(SUBJECT_S_047, HI_ARRIVER);
    assert!(game.state.player1.waitroom.cards.contains(&subject));
    assert_eq!(
        game.state.mods.get_blade_modifier(arriver),
        2,
        "桜内梨子 prints the same ability: +2 blade on the arriver"
    );
}

#[test]
fn hs_pr_040_baton_over_cost_nine_arriver_gains_two_blades() {
    let (game, subject, arriver, _left) = baton_over_subject(SUBJECT_HS_040, HI_ARRIVER);
    assert!(game.state.player1.waitroom.cards.contains(&subject));
    assert_eq!(
        game.state.mods.get_blade_modifier(arriver),
        2,
        "安養寺姫芽 prints the same ability: +2 blade on the arriver"
    );
}

#[test]
fn baton_over_cost_seven_arriver_gains_nothing() {
    let (game, subject, arriver, _left) = baton_over_subject(SUBJECT_PR_025, LO_ARRIVER);
    assert!(
        game.state.player1.waitroom.cards.contains(&subject),
        "the baton still displaces her - only the blade is gated"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(arriver),
        0,
        "「コスト9以上のメンバー」: a cost-7 arriver must gain no blade"
    );
}
