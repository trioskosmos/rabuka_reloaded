use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// PL!N-pb1-002-R 中須かすみ
//   登場: 自分のエネルギー置き場にあるエネルギー2枚をこのメンバーの下に置いてもよい。
//   常時: このメンバーの下にエネルギーカードが2枚以上置かれているかぎり、
//         ライブの合計スコアを＋１する。
//
// 「ライブの合計スコアを＋１する」 is a live-TOTAL bonus: it lands in the
// per-player constant accumulator (p1_constant_total_score_bonus), NOT as a
// per-card score modifier keyed under Kasumi's member id.

#[test]
fn kasumi_constant_score_bonus_applies_when_energy_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id("PL!N-pb1-002-R");

    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(20);

    game.play_to_stage(kasumi, MemberArea::Center);

    // Before paying optional cost, total bonus should be 0
    let before = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(before, 0, "No score bonus without energy under member");

    // Pay the optional cost (place 2 energy under)
    assert!(game.has_pending_choice());
    game.select_energy_from_zone(2);

    assert_eq!(
        game.state.player1.stage.under_cards[1].len(),
        2,
        "Two energy cards under Kasumi"
    );

    // Now 2 energy are under → live-total bonus should apply
    let after = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(
        after, 1,
        "+1 live-total score bonus when 2+ energy under member"
    );
}

#[test]
fn kasumi_constant_score_without_energy_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id("PL!N-pb1-002-R");

    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(20);
    game.play_to_stage(kasumi, MemberArea::Center);

    // Skip the optional cost
    assert!(game.has_pending_choice());
    game.select_indices(&[]);

    assert_eq!(
        game.state.player1.stage.under_cards[1].len(),
        0,
        "No energy under Kasumi"
    );
    let after = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(after, 0, "No score bonus without energy under member");
}
