use crate::helpers::*;
use rabuka_engine::core::types::Phase;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn run_live_through_victory(game: &mut TestGame, live_id: i16) {
    advance_to_live_card_set(game);
    game.set_live_card(live_id);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

fn setup_stage_with_3_aqours(game: &mut TestGame) -> (i16, Vec<i16>) {
    // Use abilityless N-rare cards for the 2 supporting members
    // so their abilities don't interfere with the test.
    let mari = game.id("PL!S-bp2-008-R\u{ff0b}");
    let riko = game.id("PL!S-bp2-011-N"); // 桜内梨子, blade=3, heart02:1, heart04:2, heart05:2
    let dia = game.id("PL!S-bp2-013-N"); // 黒澤ダイヤ, blade=3, heart02:1, heart04:1, heart05:2
    game.add_to_stage(MemberArea::LeftSide, mari);
    game.add_to_stage(MemberArea::Center, riko);
    game.add_to_stage(MemberArea::RightSide, dia);
    (mari, vec![mari, riko, dia])
}

/// All 3 areas filled with Aqours (including Mari), distinct names →
/// condition met, ability gained, delayed effect stored.
#[test]
fn all_areas_aqours_diff_names_gains_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    let (mari, _stage_ids) = setup_stage_with_3_aqours(&mut game);

    game.state.player1.hand.cards.push(filler);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.recalculate_constants();

    let gained = game.state.gained_abilities.get(&mari);
    assert!(gained.is_some(), "Mari should have gained an ability");
    if let Some(texts) = gained {
        let has_live_success = texts
            .iter()
            .any(|t| t.contains("ライブ成功時") || t.contains("エール"));
        assert!(
            has_live_success,
            "Gained ability should reference live_success/yell"
        );
    }
    assert!(
        !game.state.delayed_gained_effects.is_empty(),
        "delayed_gained_effects should be populated"
    );
}

/// Empty area → condition fails → no gain.
#[test]
fn empty_area_fails_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp2-008-R\u{ff0b}");
    let chika = game.id("PL!S-sd1-001-SD");

    game.add_to_stage(MemberArea::LeftSide, mari);
    game.add_to_stage(MemberArea::Center, chika);
    game.state.recalculate_constants();

    let gained = game.state.gained_abilities.get(&mari);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Mari should NOT gain ability with empty area"
    );
}

/// Duplicate names → distinct condition fails → no gain.
#[test]
fn duplicate_names_fails_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp2-008-R\u{ff0b}");
    let mari2 = game.new_id("PL!S-bp2-008-R\u{ff0b}");
    let chika = game.id("PL!S-sd1-001-SD");

    game.add_to_stage(MemberArea::LeftSide, mari);
    game.add_to_stage(MemberArea::Center, mari2);
    game.add_to_stage(MemberArea::RightSide, chika);
    game.state.recalculate_constants();

    let gained = game.state.gained_abilities.get(&mari);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Mari should NOT gain ability with duplicate names"
    );
}

/// Non-Aqours member on one area → condition fails → no gain.
#[test]
fn non_aqours_member_fails_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp2-008-R\u{ff0b}");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, mari);
    game.add_to_stage(MemberArea::Center, chika);
    game.add_to_stage(MemberArea::RightSide, filler);
    game.state.recalculate_constants();

    let gained = game.state.gained_abilities.get(&mari);
    assert!(
        gained.is_none() || gained.unwrap().is_empty(),
        "Mari should NOT gain ability with non-Aqours member"
    );
}

/// Full live with condition met + yell revealing live cards →
/// the delayed gained effect is evaluated and the score bonus is applied.
/// Uses "WE WILL!!" live card (needs heart02+heart06+heart0), with heart
/// modifiers added to ensure the live succeeds.
#[test]
fn score_bonus_applied_with_revealed_live_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    use rabuka_engine::card::HeartColor;
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!SP-sd1-023-SD"); // WE WILL!! — score=1, need heart02+heart06+heart0
    let yell_live1 = game.id("PL!SP-sd1-023-SD");
    let yell_live2 = game.id("PL!SP-sd1-023-SD");

    let (_mari, stage_ids) = setup_stage_with_3_aqours(&mut game);

    // WE WILL!! needs heart02×1 + heart06×1 + heart0×1 per card (3 × 3 = 9).
    // Stage provides: heart02=2+1=3 (Mari:2 + Riko:1), heart05=2+2=4 (Mari:2 + Riko:2 + Dia:2), heart04=2+1=3.
    // heart06=0, heart0=0.  We need heart06×3.  Add it to the 2nd stage member.
    game.state
        .mods
        .add_heart_modifier(stage_ids[1], HeartColor::Heart06, 3);

    // Place the live card in hand
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    // Stack live cards at the top of the deck (yell will reveal them)
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(yell_live2);
    game.state.player1.main_deck.cards.push(yell_live1);
    game.state.player1.main_deck.cards.push(live_card);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.recalculate_constants();

    run_live_through_victory(&mut game, live_card);

    // After victory determination, the delayed gained effect evaluated the
    // conditional_alternative.  With >= 3 live cards among revealed yell cards,
    // the >= 3 alternative_condition triggered the +2 bonus.
    // The modify_score was executed — verify through the ability_applications
    // recorded during delayed effects processing or via the score breakdown.
    let mod_score = game.state.mods.get_score_modifier(live_card);
    assert!(
        mod_score >= 2,
        "Score modifier on live card should be >= +2 (Mari bonus). Got {}",
        mod_score
    );
}
