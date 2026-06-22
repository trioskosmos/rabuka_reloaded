use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor};
use rabuka_engine::game_state::Phase;
use rabuka_engine::zones::MemberArea;

fn setup_both_in_live_zone(game: &mut TestGame) -> (i16, i16, i16) {
    let member = game.id("PL!N-PR-003-PR");
    let phoenix = game.id("PL!N-pb1-038-L");
    let stellar = game.id("PL!N-pb1-039-L");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state
        .player1
        .stage
        .set_area(MemberArea::Center, member);
    game.state.player1.live_card_zone.cards.push(phoenix);
    game.state.player1.live_card_zone.cards.push(stellar);
    let mut hearts = std::collections::HashMap::new();
    hearts.insert(HeartColor::Heart01, 7);
    hearts.insert(HeartColor::Heart02, 2);
    hearts.insert(HeartColor::Heart06, 6);
    hearts.insert(HeartColor::Heart00, 10);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    (member, phoenix, stellar)
}

fn process_abilities(game: &mut TestGame) {
    let player_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    while game.state.has_pending_choice() {
        game.select_indices(&[0]);
        game.state.process_pending_auto_abilities(&player_id);
    }
}

#[test]
fn both_in_live_zone_both_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (member, phoenix, _stellar) = setup_both_in_live_zone(&mut game);
    process_abilities(&mut game);
    let phoenix_score = game.state.mods.get_score_modifier(phoenix);
    assert_eq!(
        phoenix_score, 1,
        "PHOENIX: +1 score from Stellar Stream's heart01=4"
    );
    let member_heart06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(member_heart06, 4, "Stellar Stream: +4 heart06 to member");
}

/// Condition checks SUCCESS zone: subject in LIVE, target in SUCCESS → should fire.
#[test]
fn subject_in_live_target_in_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!N-PR-003-PR");
    let phoenix = game.id("PL!N-pb1-038-L");
    let stellar = game.id("PL!N-pb1-039-L");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state
        .player1
        .stage
        .set_area(MemberArea::Center, member);
    // Subject (PHOENIX) in LIVE zone → ability active (9.3.4.3)
    game.state.player1.live_card_zone.cards.push(phoenix);
    // Target (Stellar Stream, heart01=4) in SUCCESS zone → condition should find it
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(stellar);
    let mut hearts = std::collections::HashMap::new();
    hearts.insert(HeartColor::Heart01, 7);
    hearts.insert(HeartColor::Heart02, 2);
    hearts.insert(HeartColor::Heart06, 6);
    hearts.insert(HeartColor::Heart00, 10);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    process_abilities(&mut game);
    // PHOENIX condition: find heart01>=4 Niji card in success/live zone
    // → finds Stellar Stream (heart01=4) in SUCCESS zone → +1 score
    let phoenix_score = game.state.mods.get_score_modifier(phoenix);
    assert_eq!(
        phoenix_score, 1,
        "PHOENIX in live zone: +1 score from Stellar Stream in success zone"
    );
    // Stellar Stream is in SUCCESS zone → ability NOT active → no heart06
    let member_heart06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(
        member_heart06, 0,
        "Stellar Stream in success zone: ability inactive, no heart06"
    );
}

/// Card in SUCCESS zone → live_start ability does NOT fire (9.3.4.3).
#[test]
fn subject_in_success_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!N-PR-003-PR");
    let phoenix = game.id("PL!N-pb1-038-L");
    let stellar = game.id("PL!N-pb1-039-L");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state
        .player1
        .stage
        .set_area(MemberArea::Center, member);
    // Subject (PHOENIX) in SUCCESS zone → ability NOT active
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(phoenix);
    // Target (Stellar Stream) in LIVE zone → would match condition if ability fired
    game.state.player1.live_card_zone.cards.push(stellar);
    let mut hearts = std::collections::HashMap::new();
    hearts.insert(HeartColor::Heart01, 7);
    hearts.insert(HeartColor::Heart02, 2);
    hearts.insert(HeartColor::Heart06, 6);
    hearts.insert(HeartColor::Heart00, 10);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    process_abilities(&mut game);
    // PHOENIX in success zone → ability doesn't fire → no score
    let phoenix_score = game.state.mods.get_score_modifier(phoenix);
    assert_eq!(
        phoenix_score, 0,
        "PHOENIX in success zone: ability not active, no score"
    );
    // Stellar in LIVE zone → ability fires, condition finds PHOENIX in success (heart01=3≥3)
    let member_heart06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(
        member_heart06, 4,
        "Stellar Stream in live zone: +4 heart06 from PHOENIX in success zone"
    );
}

/// Both in SUCCESS zone → neither ability fires (9.3.4.3).
#[test]
fn both_in_success_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!N-PR-003-PR");
    let phoenix = game.id("PL!N-pb1-038-L");
    let stellar = game.id("PL!N-pb1-039-L");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state
        .player1
        .stage
        .set_area(MemberArea::Center, member);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(phoenix);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(stellar);
    let mut hearts = std::collections::HashMap::new();
    hearts.insert(HeartColor::Heart01, 7);
    hearts.insert(HeartColor::Heart02, 2);
    hearts.insert(HeartColor::Heart06, 6);
    hearts.insert(HeartColor::Heart00, 10);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    process_abilities(&mut game);
    assert_eq!(
        game.state.mods.get_score_modifier(phoenix),
        0,
        "PHOENIX in success: no fire"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(member, HeartColor::Heart06),
        0,
        "Stellar in success: no fire"
    );
}

#[test]
fn both_in_live_zone_check_success_zone_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (member, phoenix, _stellar) = {
        let member = game.id("PL!N-PR-003-PR");
        let phoenix = game.id("PL!N-pb1-038-L");
        let stellar = game.id("PL!N-pb1-039-L");
        game.state.player1.stage.stage = [-1, -1, -1];
        game.state
            .player1
            .stage
            .set_area(MemberArea::Center, member);
        // Both in live zone (abilities active per 9.3.4.3).
        // Their conditions check success+live zones for matching cards.
        game.state.player1.live_card_zone.cards.push(phoenix);
        game.state.player1.live_card_zone.cards.push(stellar);
        let mut hearts = std::collections::HashMap::new();
        hearts.insert(HeartColor::Heart01, 7);
        hearts.insert(HeartColor::Heart02, 2);
        hearts.insert(HeartColor::Heart06, 6);
        hearts.insert(HeartColor::Heart00, 10);
        game.state.player1.stage_hearts = Some(BaseHeart { hearts });
        game.state.current_phase = Phase::LiveCardSetFirstAttacker;
        (member, phoenix, stellar)
    };
    process_abilities(&mut game);
    // PHOENIX: heart01>=4 needed → finds Stellar Stream (heart01=4) in live zone → +1 score
    let phoenix_score = game.state.mods.get_score_modifier(phoenix);
    assert_eq!(
        phoenix_score, 1,
        "PHOENIX: +1 from Stellar Stream in live zone"
    );
    // Stellar Stream: heart01>=3 needed → finds PHOENIX (heart01=3) in live zone → +4 heart06
    let member_heart06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(
        member_heart06, 4,
        "Stellar Stream: +4 heart06 from PHOENIX in live zone"
    );
}

#[test]
fn non_niji_live_card_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!N-PR-003-PR");
    let phoenix = game.id("PL!N-pb1-038-L");
    // Use a non-Nijigasaki live card instead of Stellar Stream
    let filler_live = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state
        .player1
        .stage
        .set_area(MemberArea::Center, member);
    game.state.player1.live_card_zone.cards.push(phoenix);
    game.state.player1.live_card_zone.cards.push(filler_live);
    let mut hearts = std::collections::HashMap::new();
    hearts.insert(HeartColor::Heart01, 7);
    hearts.insert(HeartColor::Heart02, 2);
    hearts.insert(HeartColor::Heart06, 6);
    hearts.insert(HeartColor::Heart00, 10);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    process_abilities(&mut game);
    // Neither condition should trigger: filler is not Nijigasaki
    let phoenix_score = game.state.mods.get_score_modifier(phoenix);
    assert_eq!(
        phoenix_score, 0,
        "PHOENIX should NOT get score: no Niji card with heart01=4"
    );
    let member_heart06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(
        member_heart06, 0,
        "Stellar Stream should NOT trigger: no Niji card with heart01=3"
    );
}

#[test]
fn stellar_stream_chooses_one_of_multiple_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member_a = game.id("PL!N-PR-003-PR");
    let member_b = game.id("PL!N-PR-005-PR"); // Also has heart06
    let phoenix = game.id("PL!N-pb1-038-L");
    let stellar = game.id("PL!N-pb1-039-L");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state
        .player1
        .stage
        .set_area(MemberArea::Center, member_a);
    game.state
        .player1
        .stage
        .set_area(MemberArea::LeftSide, member_b);
    game.state.player1.live_card_zone.cards.push(phoenix);
    game.state.player1.live_card_zone.cards.push(stellar);
    let mut hearts = std::collections::HashMap::new();
    hearts.insert(HeartColor::Heart01, 7);
    hearts.insert(HeartColor::Heart02, 2);
    hearts.insert(HeartColor::Heart06, 6);
    hearts.insert(HeartColor::Heart00, 10);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    process_abilities(&mut game);
    // Stellar Stream's effect has target_count=1, so only ONE member gets +4 heart06
    let member_a_heart06 = game
        .state
        .mods
        .get_heart_modifier(member_a, HeartColor::Heart06);
    let member_b_heart06 = game
        .state
        .mods
        .get_heart_modifier(member_b, HeartColor::Heart06);
    let total_buff = member_a_heart06 + member_b_heart06;
    assert_eq!(
        total_buff, 4,
        "Exactly 4 heart06 distributed total (only one member got buff)"
    );
    // At least one of them should have the buff
    assert!(
        member_a_heart06 == 4 || member_b_heart06 == 4,
        "One member should have +4 heart06"
    );
}
