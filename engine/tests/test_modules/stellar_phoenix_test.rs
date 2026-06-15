use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor};
use rabuka_engine::game_state::Phase;
use rabuka_engine::zones::MemberArea;

fn setup_both_in_live_zone(game: &mut TestGame) -> (i16, i16, i16) {
    let member = game.id("PL!N-PR-003-PR");
    let phoenix = game.id("PL!N-pb1-038-L");
    let stellar = game.id("PL!N-pb1-039-L");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.stage.set_area(MemberArea::Center, member);
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
    assert_eq!(phoenix_score, 1, "PHOENIX: +1 score from Stellar Stream's heart01=4");
    let member_heart06 = game.state.mods.get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(member_heart06, 4, "Stellar Stream: +4 heart06 to member");
}

#[test]
fn phoenix_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (member, phoenix, _stellar) = {
        let member = game.id("PL!N-PR-003-PR");
        let phoenix = game.id("PL!N-pb1-038-L");
        let stellar = game.id("PL!N-pb1-039-L");
        game.state.player1.stage.stage = [-1, -1, -1];
        game.state.player1.stage.set_area(MemberArea::Center, member);
        // PHOENIX in success zone, Stellar Stream in live zone
        game.state.player1.success_live_card_zone.cards.push(phoenix);
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
    // PHOENIX should have +1 score (condition: heart01=4 card in zone → Stellar Stream)
    let phoenix_score = game.state.mods.get_score_modifier(phoenix);
    assert_eq!(phoenix_score, 1, "PHOENIX in success zone: +1 score");
    let member_heart06 = game.state.mods.get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(member_heart06, 4, "Stellar Stream with PHOENIX in success zone: +4 heart06");
}

#[test]
fn stellar_stream_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (member, phoenix, _stellar) = {
        let member = game.id("PL!N-PR-003-PR");
        let phoenix = game.id("PL!N-pb1-038-L");
        let stellar = game.id("PL!N-pb1-039-L");
        game.state.player1.stage.stage = [-1, -1, -1];
        game.state.player1.stage.set_area(MemberArea::Center, member);
        // Stellar Stream in success zone, PHOENIX in live zone
        game.state.player1.success_live_card_zone.cards.push(stellar);
        game.state.player1.live_card_zone.cards.push(phoenix);
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
    // PHOENIX should have +1 score (condition: heart01=4 card in zone → Stellar Stream)
    let phoenix_score = game.state.mods.get_score_modifier(phoenix);
    assert_eq!(phoenix_score, 1, "PHOENIX +1 from Stellar Stream in success zone");
    let member_heart06 = game.state.mods.get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(member_heart06, 4, "Stellar Stream in success zone: +4 heart06");
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
    game.state.player1.stage.set_area(MemberArea::Center, member);
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
    assert_eq!(phoenix_score, 0, "PHOENIX should NOT get score: no Niji card with heart01=4");
    let member_heart06 = game.state.mods.get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(member_heart06, 0, "Stellar Stream should NOT trigger: no Niji card with heart01=3");
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
    game.state.player1.stage.set_area(MemberArea::Center, member_a);
    game.state.player1.stage.set_area(MemberArea::LeftSide, member_b);
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
    let member_a_heart06 = game.state.mods.get_heart_modifier(member_a, HeartColor::Heart06);
    let member_b_heart06 = game.state.mods.get_heart_modifier(member_b, HeartColor::Heart06);
    let total_buff = member_a_heart06 + member_b_heart06;
    assert_eq!(total_buff, 4, "Exactly 4 heart06 distributed total (only one member got buff)");
    // At least one of them should have the buff
    assert!(
        member_a_heart06 == 4 || member_b_heart06 == 4,
        "One member should have +4 heart06"
    );
}
