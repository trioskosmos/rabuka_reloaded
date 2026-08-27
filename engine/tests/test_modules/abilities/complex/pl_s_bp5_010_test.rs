use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!S-bp5-010-N 璃奈 (Rina, Aqours)
/// 登場 (Debut): If stage members have >= 5 total heart02,
///   opponent's live card need_heart increases by heart00
///   (making it harder for opponent to succeed their live).
///
/// Edge cases:
///   - heart02 < 5 → no effect
///   - heart02 >= 5 → opponent's live card need modified
///   - Affects opponent's turn, not own

fn trigger_debut_for_card(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("debut ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

#[test]
fn rina_bp5n_heart02_ge5_increases_opponent_need_heart00() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!S-bp5-010-N");
    let h02_member = game.id("PL!S-sd1-010-SD"); // heart02=2 each
    let opp_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: [h02(2), Rina(1), h02(2)] = 5 total heart02
    game.state.player1.stage.stage = [h02_member, rina, h02_member];
    game.state.player2.live_card_zone.cards.push(opp_live);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(5);

    // Verify heart02 total
    let stage_hearts = game.state.player1.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    let h02_total = *stage_hearts.hearts.get(&HeartColor::Heart02).unwrap_or(&0);
    assert!(
        h02_total >= 5,
        "heart02 total should be >=5, got {}",
        h02_total
    );

    let need_before = game
        .state
        .mods
        .get_need_heart_modifier(opp_live, HeartColor::Heart00);
    trigger_debut_for_card(&mut game, rina);
    let need_after = game
        .state
        .mods
        .get_need_heart_modifier(opp_live, HeartColor::Heart00);

    assert!(
        need_after > need_before,
        "need_heart00 for opponent must increase ({} -> {})",
        need_before,
        need_after
    );
}

#[test]
fn rina_bp5n_heart02_lt5_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!S-bp5-010-N");
    let opp_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: Rina alone (heart02=1) only → total < 5
    game.state.player1.stage.stage = [-1, rina, -1];
    game.state.player2.live_card_zone.cards.push(opp_live);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(5);

    let stage_hearts = game.state.player1.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    let h02_total = *stage_hearts.hearts.get(&HeartColor::Heart02).unwrap_or(&0);
    assert!(
        h02_total < 5,
        "heart02 total should be <5, got {}",
        h02_total
    );

    let need_before = game
        .state
        .mods
        .get_need_heart_modifier(opp_live, HeartColor::Heart00);
    trigger_debut_for_card(&mut game, rina);
    let need_after = game
        .state
        .mods
        .get_need_heart_modifier(opp_live, HeartColor::Heart00);

    assert_eq!(
        need_after, need_before,
        "no change when heart02 < 5 ({} vs {})",
        need_before, need_after
    );
}
