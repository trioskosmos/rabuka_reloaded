use crate::helpers::*;
use rabuka_engine::ability::condition::ConditionContext;
use rabuka_engine::ability::resolver::AbilityResolver;
use rabuka_engine::card::HeartColor;
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;
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
    assert_eq!(game.state.current_phase, Phase::LiveVictoryDetermination);
    TurnEngine::execute_live_victory_determination(&mut game.state);
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

#[test]
fn repeated_recalculation_registers_delayed_ability_once() {
    let mut game = TestGame::new(load_real_database());
    let mari = setup_stage_with_3_aqours(&mut game).0;

    for _ in 0..5 {
        game.state.recalculate_constants();
        assert_eq!(game.state.delayed_gained_effects.len(), 1);
        assert_eq!(game.state.delayed_gained_effects[0].0, mari);
        assert_eq!(game.state.gained_abilities.get(&mari).unwrap().len(), 1);
        assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
        assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
    }
}

#[test]
fn condition_loss_removes_delayed_registration_and_restoring_stage_registers_once() {
    for changed_area in [MemberArea::LeftSide, MemberArea::RightSide] {
        let mut game = TestGame::new(load_real_database());
        let (mari, _) = setup_stage_with_3_aqours(&mut game);
        game.state.recalculate_constants();
        assert_eq!(game.state.delayed_gained_effects.len(), 1);
        let removed = game.state.player1.stage.get_area(changed_area).unwrap();
        game.state.player1.stage.set_area(changed_area, -1);
        game.state.player1.waitroom.cards.push(removed);
        log::debug!(
            "[MARI_REMOVAL] area={:?} removed={} print={} mari={} stage={:?}",
            changed_area,
            removed,
            game.db.get_card(removed).unwrap().card_no,
            mari,
            game.state.player1.stage.stage
        );

        for step in 0..3 {
            game.state.recalculate_constants();
            log::debug!(
                "[MARI_REMOVAL] step={} source_on_stage={} delayed={} gained={:?}",
                step,
                game.state.player1.stage.stage.contains(&mari),
                game.state.delayed_gained_effects.len(),
                game.state.gained_abilities.get(&mari)
            );
            assert!(game.state.delayed_gained_effects.is_empty());
            assert!(game
                .state
                .gained_abilities
                .get(&mari)
                .is_none_or(|abilities| abilities.is_empty()));
        }

        game.state
            .player1
            .waitroom
            .cards
            .retain(|card| *card != removed);
        game.state.player1.stage.set_area(changed_area, removed);
        for _ in 0..3 {
            game.state.recalculate_constants();
            assert_eq!(game.state.delayed_gained_effects.len(), 1);
            assert_eq!(game.state.delayed_gained_effects[0].0, mari);
        }
    }
}

/// "No ability was gained" for `card`.
///
/// The three negative tests below all assert this, and each used to spell it
/// `gained.is_none() || gained.unwrap().is_empty()` — a double negative that
/// reads as if some third state were allowed. A zero-length list is the claim.
fn assert_nothing_gained(game: &TestGame, card: i16, ctx: &str) {
    match game.state.gained_abilities.get(&card) {
        None => {}
        Some(list) => assert!(
            list.is_empty(),
            "{}: expected no ability gained, got {list:?}",
            ctx
        ),
    }
}

/// Empty area → condition fails → no gain.
#[test]
fn empty_area_fails_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp2-008-R\u{ff0b}");
    let chika = game.id("PL!S-sd1-001-SD");
    game.assert_card_identity(mari, "PL!S-bp2-008-R＋");

    game.add_to_stage(MemberArea::LeftSide, mari);
    game.add_to_stage(MemberArea::Center, chika);
    game.state.recalculate_constants();

    // Setup guard: the right area must really be empty, or this is not the
    // "empty area" case and the assertion below would pass for any reason.
    assert_eq!(
        game.state.player1.stage.stage[2],
        -1,
        "setup guard: the right area must be the empty one"
    );
    assert_nothing_gained(&game, mari, "empty area");
}

/// Duplicate names → distinct condition fails → no gain.
#[test]
fn duplicate_names_fails_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp2-008-R\u{ff0b}");
    let mari2 = game.new_id("PL!S-bp2-008-R\u{ff0b}");
    let chika = game.id("PL!S-sd1-001-SD");
    game.assert_card_identity(mari, "PL!S-bp2-008-R＋");
    // 名前が異なる is the condition under test, so prove the two instances really
    // share one name and are two separate cards.
    game.assert_same_card_name(mari, mari2, "two copies of 国木田花丸");
    assert_ne!(mari, mari2, "two separate card instances");

    game.add_to_stage(MemberArea::LeftSide, mari);
    game.add_to_stage(MemberArea::Center, mari2);
    game.add_to_stage(MemberArea::RightSide, chika);
    game.state.recalculate_constants();

    assert_nothing_gained(&game, mari, "duplicate names");
}

/// Non-Aqours member on one area → condition fails → no gain.
#[test]
fn non_aqours_member_fails_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp2-008-R\u{ff0b}");
    let chika = game.id("PL!S-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(mari, "PL!S-bp2-008-R＋");
    game.assert_card_identity(filler, "PL!-sd1-010-SD");
    // Group matching is SERIES-based (see WRITING_TESTS "Card group matching
    // depends on series"), so pin the contrast rather than a group field: the
    // filler must be a different card from the 『Aqours』 members the positive
    // twin stages, and the positive twin is what proves the series check works.
    game.assert_distinct_card_names(
        filler,
        game.id("PL!S-bp2-011-N"),
        "the non-『Aqours』 fixture vs an 『Aqours』 member",
    );

    game.add_to_stage(MemberArea::LeftSide, mari);
    game.add_to_stage(MemberArea::Center, chika);
    game.add_to_stage(MemberArea::RightSide, filler);
    game.state.recalculate_constants();

    assert_nothing_gained(&game, mari, "non-『Aqours』 member on stage");
}

/// 0 live cards in yell → condition not met → no bonus anywhere.
#[test]
fn zero_live_cards_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = setup_stage_with_3_aqours(&mut game).0;
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!SP-sd1-023-SD");

    game.state.player1.live_card_zone.cards.push(live_card);
    game.state.recalculate_constants();

    game.state.revealed_cards.push(filler);
    assert!(!evaluate_delayed_mari(&mut game, mari), "No condition met");

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "No live-total bonus without live cards among reveals"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "No per-card modifier without live cards among reveals"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(mari),
        0,
        "No per-card modifier on Mari without live cards among reveals"
    );
}

fn fill_unique_decks(game: &mut TestGame) {
    for _ in 0..4 {
        for card_no in [
            "PL!-sd1-010-SD",
            "PL!S-bp2-011-N",
            "PL!S-bp2-012-N",
            "PL!S-bp2-013-N",
        ] {
            let p1_card = game.new_id(card_no);
            let p2_card = game.new_id(card_no);
            assert!(game.db.get_card(p1_card).is_some());
            assert!(game.db.get_card(p2_card).is_some());
            game.state.player1.main_deck.cards.push(p1_card);
            game.state.player2.main_deck.cards.push(p2_card);
        }
    }
    for side in [Side::P1, Side::P2] {
        let energy = game.new_id("LL-E-001-SD");
        match side {
            Side::P1 => game.state.player1.energy_deck.cards.push(energy),
            Side::P2 => game.state.player2.energy_deck.cards.push(energy),
        }
    }
}

#[test]
fn score_bonus_applied_with_revealed_live_cards() {
    let mut game = TestGame::new(load_real_database());
    let (mari, stage_ids) = setup_stage_with_3_aqours(&mut game);
    fill_unique_decks(&mut game);
    let live_card = game.new_id("PL!SP-sd1-023-SD");
    let yell_lives: Vec<_> = (0..3).map(|_| game.new_id("PL!SP-sd1-023-SD")).collect();
    let draw_card = game.state.player1.main_deck.cards.remove(0);
    for &card in yell_lives.iter().rev() {
        game.state.player1.main_deck.cards.insert(0, card);
    }
    game.state.player1.main_deck.cards.insert(0, draw_card);
    game.state.player1.hand.cards.push(live_card);
    game.state
        .mods
        .add_heart_modifier(stage_ids[1], HeartColor::Heart06, 1);
    for _ in 0..3 {
        game.state.recalculate_constants();
    }

    run_live_through_victory(&mut game, live_card);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.current_phase, Phase::LiveVictoryDetermination);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 2);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
    assert_eq!(game.state.mods.get_score_modifier(mari), 0);
    assert_eq!(game.state.mods.get_score_modifier(live_card), 0);
    assert!(game.state.player1.hand.cards.contains(&draw_card));
    assert!(game
        .state
        .player1
        .success_live_card_zone
        .cards
        .contains(&live_card));
    let snapshot = game
        .state
        .performance_snapshots
        .iter()
        .find(|snapshot| snapshot.player_id == "p1")
        .expect("Mari's owner must have a performance snapshot");
    assert!(snapshot.success);
    assert_eq!(snapshot.lives.len(), 1);
    assert_eq!(snapshot.lives[0].card_id, live_card);
    assert_eq!(snapshot.lives[0].base_score, 1);
    assert_eq!(snapshot.lives[0].score, 1);
    assert_eq!(snapshot.total_score, 6);
    assert_eq!(
        snapshot
            .yell_cards
            .iter()
            .take(3)
            .map(|card| card.card_id)
            .collect::<Vec<_>>(),
        yell_lives
    );
    assert!(!snapshot
        .yell_cards
        .iter()
        .any(|card| card.card_id == live_card || card.card_id == draw_card));
}

#[test]
fn successful_owners_use_their_own_zero_one_or_three_live_reveals() {
    for p1_reveals in [0, 1, 2, 3, 4] {
        for p2_reveals in [0, 1, 2, 3, 4] {
            assert_owner_gameplay([p1_reveals, p2_reveals], [true, true], [true, true]);
        }
    }
}

#[test]
fn failed_owner_gets_no_bonus_even_with_three_live_reveals() {
    for owner in 0..2 {
        let mut succeeds = [true, true];
        succeeds[owner] = false;
        assert_owner_gameplay([3, 3], [true, true], succeeds);
    }
}

#[test]
fn owner_without_live_gets_no_bonus_from_opponents_reveals() {
    for owner in 0..2 {
        let mut sets_live = [true, true];
        sets_live[owner] = false;
        assert_owner_gameplay([3, 3], sets_live, [true, true]);
    }
}

fn assert_owner_gameplay(reveal_counts: [usize; 2], sets_live: [bool; 2], succeeds: [bool; 2]) {
    let mut game = TestGame::new(load_real_database());
    let (_, p1_stage) = setup_stage_with_3_aqours(&mut game);
    let p2_stage = [
        game.new_id("PL!S-bp2-008-R\u{ff0b}"),
        game.new_id("PL!S-bp2-011-N"),
        game.new_id("PL!S-bp2-013-N"),
    ];
    game.state.player2.stage.stage = p2_stage;
    fill_unique_decks(&mut game);
    let live_cards = [
        game.new_id("PL!SP-sd1-023-SD"),
        game.new_id("PL!SP-sd1-023-SD"),
    ];
    let stages = [p1_stage.as_slice(), p2_stage.as_slice()];
    for owner in 0..2 {
        if succeeds[owner] {
            game.state
                .mods
                .add_heart_modifier(stages[owner][1], HeartColor::Heart06, 1);
        }
        if sets_live[owner] {
            game.add_to_hand_for(
                if owner == 0 { Side::P1 } else { Side::P2 },
                live_cards[owner],
            );
        }
    }
    for _ in 0..3 {
        game.state.recalculate_constants();
        assert_eq!(game.state.delayed_gained_effects.len(), 2);
    }
    advance_to_live_card_set(&mut game);
    assert_eq!(game.state.current_phase, Phase::LiveCardSetFirstAttacker);
    if sets_live[0] {
        game.set_live_card(live_cards[0]);
    }
    game.pass();
    assert_eq!(game.state.current_phase, Phase::LiveCardSetSecondAttacker);
    if sets_live[1] {
        game.set_live_card(live_cards[1]);
    }
    game.pass();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.current_phase, Phase::FirstAttackerPerformance);
    let yell_lives: [Vec<i16>; 2] = std::array::from_fn(|owner| {
        (0..reveal_counts[owner])
            .map(|_| game.new_id("PL!SP-sd1-023-SD"))
            .collect()
    });
    for (owner, cards) in yell_lives.iter().enumerate() {
        let deck = if owner == 0 {
            &mut game.state.player1.main_deck.cards
        } else {
            &mut game.state.player2.main_deck.cards
        };
        for &card in cards.iter().rev() {
            deck.insert(0, card);
        }
    }
    for expected in [
        Phase::SecondAttackerPerformance,
        Phase::LiveVictoryDetermination,
    ] {
        game.pass();
        assert!(!game.has_pending_choice());
        assert_eq!(game.state.current_phase, expected);
    }
    TurnEngine::execute_live_victory_determination(&mut game.state);
    assert!(!game.has_pending_choice());
    for owner in 0..2 {
        let player_id = if owner == 0 { "p1" } else { "p2" };
        let snapshot = game
            .state
            .performance_snapshots
            .iter()
            .find(|snapshot| snapshot.player_id == player_id)
            .unwrap();
        let success = sets_live[owner] && succeeds[owner];
        let bonus = if success {
            match reveal_counts[owner] {
                0 => 0,
                1 | 2 => 1,
                _ => 2,
            }
        } else {
            0
        };
        assert_eq!(
            snapshot.success, success,
            "owner={owner} reveals={reveal_counts:?}"
        );
        assert_eq!(snapshot.lives.len(), usize::from(sets_live[owner]));
        if sets_live[owner] {
            assert_eq!(snapshot.lives[0].card_id, live_cards[owner]);
            assert_eq!(snapshot.lives[0].passed, succeeds[owner]);
            let revealed_lives: Vec<_> = snapshot
                .yell_cards
                .iter()
                .filter(|card| {
                    game.db.get_card(card.card_id).unwrap().card_no == "PL!SP-sd1-023-SD"
                })
                .map(|card| card.card_id)
                .collect();
            assert_eq!(revealed_lives, yell_lives[owner]);
        }
        if success {
            assert_eq!(
                snapshot.total_score,
                1 + reveal_counts[owner] as u8 + bonus as u8,
                "owner={owner} reveals={reveal_counts:?}"
            );
        } else {
            assert_eq!(snapshot.total_score, 0);
        }
        let actual_bonus = if owner == 0 {
            game.state.mods.p1_constant_total_score_bonus
        } else {
            game.state.mods.p2_constant_total_score_bonus
        };
        assert_eq!(
            actual_bonus, bonus,
            "owner={owner} reveals={reveal_counts:?}"
        );
        assert_eq!(game.state.mods.get_score_modifier(live_cards[owner]), 0);
        assert_eq!(game.state.mods.get_score_modifier(stages[owner][0]), 0);
    }
}

/// Helper: evaluate the delayed gained effect for `card_id` with the
/// current `game.state.revealed_cards`. Returns true if an effect was applied.
fn evaluate_delayed_mari(game: &mut TestGame, card_id: i16) -> bool {
    let idx = game
        .state
        .delayed_gained_effects
        .iter()
        .position(|(cid, _)| *cid == card_id)
        .expect("Mari should have a delayed gained effect");
    let (_, gained) = game.state.delayed_gained_effects.remove(idx);
    assert_eq!(
        gained.action,
        rabuka_engine::ability::enums::ActionType::ConditionalAlternative
    );
    let ctx = ConditionContext::new(&game.state);
    let alt_cond = gained.compound.alternative_condition.as_ref();
    let base_cond = gained.condition.as_ref();
    let alt_met = alt_cond.is_some_and(|c| ctx.evaluate_condition(c));
    let base_met = base_cond.is_some_and(|c| ctx.evaluate_condition(c));
    if !alt_met && !base_met {
        return false;
    }
    let effect_to_apply = if alt_met {
        gained.alternative_effect_any()
    } else {
        gained.compound.primary_effect.as_ref().map(|v| &**v)
    };
    let mut resolver = AbilityResolver::new(game.state.card_database.clone(), Some(card_id));
    resolver.activating_card_id = Some(card_id);
    let _ = resolver.execute_effect(&mut game.state, effect_to_apply.unwrap());
    true
}

/// 1 live card in yell → +1 score.
#[test]
fn one_live_card_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = setup_stage_with_3_aqours(&mut game).0;
    let live_card = game.id("PL!SP-sd1-023-SD");

    game.state.player1.live_card_zone.cards.push(live_card);
    game.state.recalculate_constants();

    game.state.revealed_cards.push(live_card);
    assert!(evaluate_delayed_mari(&mut game, mari));

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "1 live card among reveals → live total score +1"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "printed target is the live total score, not a per-card modifier"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(mari),
        0,
        "printed target is the live total score, not a per-card modifier"
    );
}

/// 3 live cards in yell → alternative condition met → +2 score.
#[test]
fn three_live_cards_plus_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = setup_stage_with_3_aqours(&mut game).0;
    let live_card = game.id("PL!SP-sd1-023-SD");

    game.state.player1.live_card_zone.cards.push(live_card);
    game.state.recalculate_constants();

    game.state
        .revealed_cards
        .extend([live_card, live_card, live_card]);
    assert!(evaluate_delayed_mari(&mut game, mari));

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 2,
        "3 live cards among reveals → alternative +2 applied once"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "printed target is the live total score, not a per-card modifier"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(mari),
        0,
        "printed target is the live total score, not a per-card modifier"
    );
}
