use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set(game: &mut TestGame) {
    // From Phase::Main, TurnPhase::FirstAttackerNormal:
    //   5 passes cycles through Active→Energy→Draw→Main→LiveCardSetFirstAttacker
    for _ in 0..5 {
        game.pass();
    }
}

fn run_live_full(game: &mut TestGame, live_id: i16) {
    advance_to_live_card_set(game);
    game.set_live_card(live_id);
    game.pass(); // LiveCardSetFirstAttacker → LiveCardSetSecondAttacker
    game.pass(); // LiveCardSetSecondAttacker → FirstAttackerPerformance (fires LiveStart)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (executes performance)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass(); // SecondAttackerPerformance → LiveVictoryDetermination
    game.pass(); // LiveVictoryDetermination → Active (finalizes snapshot)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

fn setup_game() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    game
}

fn snapshot_adjustments<'a>(game: &'a TestGame) -> Vec<(&'a str, i32, usize)> {
    let mut items = Vec::new();
    for snap in &game.state.performance_snapshots {
        for live in &snap.lives {
            for adj in &live.adjustments {
                items.push((adj.source.as_str(), adj.value, adj.color));
            }
        }
    }
    items
}

fn snapshot_score_bonuses<'a>(game: &'a TestGame) -> Vec<(&'a str, u32)> {
    let mut items = Vec::new();
    for snap in &game.state.performance_snapshots {
        for sline in &snap.breakdown.scores {
            items.push((sline.source.as_str(), sline.value));
        }
    }
    items
}

fn score_bonus(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_score_modifier(cid)
}

fn heart_modifier(game: &TestGame, cid: i16, color: HeartColor) -> i32 {
    game.state.mods.get_need_heart_modifier(cid, color)
}

/// 0 EMOTION in success zone → +0 score, no adjustments
#[test]
fn emotion_zero_in_success_zone() {
    let mut game = setup_game();
    let emo = game.id("PL!N-bp4-027-L");
    game.state.player1.hand.cards.push(emo);
    run_live_full(&mut game, emo);

    assert_eq!(
        score_bonus(&game, emo),
        0,
        "No EMOTION in success zone → +0 score"
    );
    assert_eq!(
        heart_modifier(&game, emo, HeartColor::Heart00),
        0,
        "No EMOTION → +0 heart00"
    );
    assert!(
        snapshot_adjustments(&game).is_empty(),
        "No adjustments expected"
    );
}

/// 1 EMOTION in success zone → +2 score, +3 heart00, snapshot has adjustment
#[test]
fn emotion_one_in_success_zone() {
    let mut game = setup_game();
    let emo = game.id("PL!N-bp4-027-L");
    let emo_in_success = game.id("PL!N-bp4-027-L");
    game.state.player1.hand.cards.push(emo);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(emo_in_success);
    run_live_full(&mut game, emo);

    assert_eq!(
        score_bonus(&game, emo),
        2,
        "1 EMOTION in success → +2 score mod"
    );
    let h00 = heart_modifier(&game, emo, HeartColor::Heart00);
    assert_eq!(h00, 3, "1 EMOTION in success → +3 heart00 mod");

    let adjustments = snapshot_adjustments(&game);
    eprintln!("[TEST ADJ] {:?}", adjustments);
    let heart_adj = adjustments.iter().find(|(_, _, c)| *c == 0);
    assert!(
        heart_adj.is_some(),
        "Heart00 adjustment pill should exist in snapshot"
    );
    let (_, val, _) = heart_adj.unwrap();
    assert_eq!(*val, 3, "Heart00 adjustment should be +3");
}

/// 2 EMOTION cards in success zone → +4 score, +6 heart00
#[test]
fn emotion_two_in_success_zone() {
    let mut game = setup_game();
    let emo = game.id("PL!N-bp4-027-L");
    let emo1 = game.id("PL!N-bp4-027-L");
    let emo2 = game.id("PL!N-bp4-027-L");
    game.state.player1.hand.cards.push(emo);
    game.state.player1.success_live_card_zone.cards.push(emo1);
    game.state.player1.success_live_card_zone.cards.push(emo2);
    run_live_full(&mut game, emo);

    assert_eq!(
        score_bonus(&game, emo),
        4,
        "2 EMOTION in success → +4 score mod"
    );
    let h00 = heart_modifier(&game, emo, HeartColor::Heart00);
    assert_eq!(h00, 6, "2 EMOTION in success → +6 heart00 mod");

    let adjustments = snapshot_adjustments(&game);
    let heart_adj = adjustments.iter().find(|(_, _, c)| *c == 0);
    assert!(heart_adj.is_some(), "Heart00 adjustment pill should exist");
    let (_, val, _) = heart_adj.unwrap();
    assert_eq!(*val, 6, "Heart00 adjustment should be +6");
}

/// The score breakdown should have exactly ONE entry per bonus source
/// (not duplicate entries from success-zone card activations).
#[test]
fn emotion_score_breakdown_no_duplicates() {
    let mut game = setup_game();
    let emo = game.id("PL!N-bp4-027-L");
    let emo_in_success = game.id("PL!N-bp4-027-L");
    game.state.player1.hand.cards.push(emo);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(emo_in_success);
    run_live_full(&mut game, emo);

    let scores = snapshot_score_bonuses(&game);
    let total_bonus: u32 = scores.iter().map(|(_, v)| v).sum();
    assert_eq!(
        scores.len(),
        1,
        "Expected exactly 1 ScoreLine (only live_zone card's activation). Got {}: {:?}",
        scores.len(),
        scores
    );
    assert_eq!(total_bonus, 2, "Total score bonus should be +2");
}
