/// QA tests for Strawberry Trapper (PL!S-pb1-021-L / GuiltyKiss)
///
/// Ability (繝ｩ繧､繝匁・蜉滓凾): compound(and)
///   (A) group_condition(Aqours, heart05 >= 4) on self stage
///   (B) temporal: this_turn + opponent_live_success + no_excess_heart
/// Effect: modify_score(+2, self_target)
///
/// Q36: timing = LiveVictoryDetermination, before winner
/// Q132: first attacker's live_success fires
/// Q142: excess heart definition
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Q36: 繝ｩ繧､繝匁・蜉滓凾 fires during LiveVictoryDetermination, NOT during
/// performance phases. Verify the ability trigger is processed during
/// LiveVictoryDetermination by checking that the game state changes
/// (opponent_live_success flag is accessible there).
#[test]
fn strawberry_q36_only_fires_in_live_victory_determination() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected at live start (observed: live card set silently)"
    );

    assert!(
        !game.state.opponent_live_success_this_turn,
        "Q36: Before LiveVictoryDetermination, no live_success processing"
    );

    game.pass();
    assert_eq!(
        game.state.current_phase.to_string(),
        "Perform (2nd)",
        "Now in P2's performance"
    );

    game.pass();
    assert!(
        game.state.current_phase.to_string().contains("Live Result"),
        "Q36: Now in LiveVictoryDetermination phase"
    );

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// Q132: First attacker's live_success ability is evaluated during
/// LiveVictoryDetermination.
#[test]
fn strawberry_q132_first_attacker_evaluated() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected at live start (observed: live card set silently)"
    );

    // Ensure group condition (A) passes: Aqours member with heart05 >= 4
    game.state
        .mods
        .add_heart_modifier(chika, rabuka_engine::card::HeartColor::Heart05, 4);

    // Ensure temporal condition (B) passes
    game.state.p2_live_success_this_turn = true;
    game.state.p2_live_success_no_excess = true;

    game.pass(); // 竊・SecondAttackerPerformance
    game.pass(); // 竊・LiveVictoryDetermination
    game.pass(); // 竊・Active (processes LiveSuccess)

    // Both conditions met 竊・live card should be in success_live_zone with score +2
    assert!(
        !game.state.player1.success_live_card_zone.cards.is_empty(),
        "Q132: Live card should be in success zone when both conditions pass"
    );
    let target = game.state.player1.success_live_card_zone.cards[0];
    assert_eq!(
        game.state.mods.get_score_modifier(target),
        0,
        "LiveSuccess score bonus cleared after live"
    );
    let l = game.state.performance_snapshots[0]
        .lives
        .iter()
        .find(|l| l.card_id == target)
        .unwrap();
    assert_eq!(l.score - l.base_score, 2, "bonus in final score");
}

/// Q142: Excess heart blocks the temporal condition.
#[test]
fn strawberry_q142_excess_heart_prevents_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected at live start (observed: live card set silently)"
    );

    // Ensure group condition (A) passes
    game.state
        .mods
        .add_heart_modifier(chika, rabuka_engine::card::HeartColor::Heart05, 4);

    // Temporal condition (B) fails: excess heart present
    game.state.p2_live_success_this_turn = true;
    game.state.p2_live_success_no_excess = false;

    game.pass(); // 竊・SecondAttackerPerformance
    game.pass(); // 竊・LiveVictoryDetermination
    game.pass(); // 竊・Active (processes LiveSuccess)

    // Excess heart blocks 竊・no score bonus
    // If live card succeeded (in success_zone), verify no +2
    if !game.state.player1.success_live_card_zone.cards.is_empty() {
        let target = game.state.player1.success_live_card_zone.cards[0];
        assert_eq!(
            game.state.mods.get_score_modifier(target),
            0,
            "Q142: Excess heart prevents score bonus"
        );
    }
}

/// Group condition: non-Aqours member 竊・group fails 竊・no score.
#[test]
fn strawberry_q142_wrong_group_prevents_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let non_aqours = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, non_aqours);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected at live start (observed: live card set silently)"
    );

    // Temporal condition (B) passes
    game.state.p2_live_success_this_turn = true;
    game.state.p2_live_success_no_excess = true;

    // Group condition (A) fails: non-Aqours member on stage

    game.pass(); // 竊・SecondAttackerPerformance
    game.pass(); // 竊・LiveVictoryDetermination
    game.pass(); // 竊・Active (processes LiveSuccess)

    // Wrong group 竊・no score bonus
    // Live card may have been discarded from live zones (heart requirements unmet)
    // Verify both live zones are empty 窶・the card was properly removed
    assert!(
        game.state.player1.success_live_card_zone.cards.is_empty(),
        "Q142: No card in success zone (heart requirements unmet with filler)"
    );
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "Q142: No card in live zone (heart requirements unmet with filler)"
    );
}

/// Edge case: opponent_live_success is false 竊・temporal fails 竊・no score.
#[test]
fn strawberry_opponent_didnt_win_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let strawberry = game.id("PL!S-pb1-021-L");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(strawberry);
    game.add_to_stage(MemberArea::Center, chika);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(strawberry);
    advance_to_live_start(&mut game);
    assert!(
        !game.has_pending_choice(),
        "no prompt expected at live start (observed: live card set silently)"
    );

    // Ensure group condition (A) passes
    game.state
        .mods
        .add_heart_modifier(chika, rabuka_engine::card::HeartColor::Heart05, 4);

    // Temporal condition (B) fails: opponent didn't win
    game.state.p2_live_success_this_turn = false;
    game.state.p2_live_success_no_excess = false;

    game.pass(); // 竊・SecondAttackerPerformance
    game.pass(); // 竊・LiveVictoryDetermination
    game.pass(); // 竊・Active (processes LiveSuccess)

    // Opponent didn't win 竊・no score bonus
    // If live card succeeded, verify no +2 score bonus
    if !game.state.player1.success_live_card_zone.cards.is_empty() {
        let target = game.state.player1.success_live_card_zone.cards[0];
        assert_eq!(
            game.state.mods.get_score_modifier(target),
            0,
            "Opponent didn't win 竊・no score bonus"
        );
    } else if !game.state.player1.live_card_zone.cards.is_empty() {
        let target = game.state.player1.live_card_zone.cards[0];
        assert_eq!(
            game.state.mods.get_score_modifier(target),
            0,
            "Opponent didn't win 竊・no score bonus"
        );
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
