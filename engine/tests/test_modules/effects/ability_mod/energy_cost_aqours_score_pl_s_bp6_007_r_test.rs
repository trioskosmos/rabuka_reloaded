use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_s_bp6_007_r_energy_cost_grants_aqours_constant_score_abilities() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp6-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    let aqours_friend = game.id("PL!S-pb1-010-PR");
    // PL!S-bp6-007-R and PL!S-pb1-007-R are one letter apart AND both print
    // 国木田花丸 — the identity pin added here caught that the old fixture was
    // staging a second copy of 花丸, so "up to TWO 『Aqours』 members" was being
    // read off two copies of one character. 高海千歌 is a genuinely different
    // 『Aqours』 member, which is what the printed claim needs.
    game.assert_card_identity(hanamaru, "PL!S-bp6-007-R");
    game.assert_card_identity(aqours_friend, "PL!S-pb1-010-PR");
    game.assert_distinct_card_names(hanamaru, aqours_friend, "花丸 and her Aqours friend");
    game.add_to_stage(MemberArea::LeftSide, aqours_friend);
    let outsider = game.id(FILLER);
    game.add_to_stage(MemberArea::RightSide, outsider);
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id("PL!-sd1-019-SD"));
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.new_id("PL!-sd1-019-SD"));
    game.give_energy(2);
    let hand_before = game.state.player1.hand.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectTarget") => game.select_option(0),
            Some("SelectCard") => game.select_indices(&[0, 1]),
            _ => game.select_indices(&[]),
        }
    }
    assert!(game.state.player1.energy_zone.active_count() <= 2, "sanity");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "energy payment leaves hand alone"
    );
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 2,
        "up to TWO 『Aqours』 members each gain ライブの合計スコア+1 (the \
         non-『Aqours』 member on the right is not counted)"
    );
}

#[test]
fn hanamaru_q_constant_score_success_threshold_matrix() {
    for success_count in 0..=3 {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let hanamaru = game.id("PL!S-bp6-007-R");
        game.add_to_stage(MemberArea::Center, hanamaru);
        for _ in 0..success_count {
            game.state
                .player2
                .success_live_card_zone
                .add_card(game.new_id("PL!-sd1-019-SD"));
        }
        game.give_energy(2);
        fire_trigger(
            &mut game,
            hanamaru,
            AbilityTrigger::LiveStart,
            "ライブ開始時",
        );
        let mut guard = 0;
        while game.has_pending_choice() && guard < 8 {
            guard += 1;
            game.select_indices(&[0, 1]);
        }
        assert_eq!(
            game.state.mods.p1_constant_total_score_bonus,
            i16::from(u8::from(success_count >= 2)),
            "success_count={success_count}"
        );
    }
}
