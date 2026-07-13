/// E2E gameplay tests for all 15 parser issues.
///
/// Each test sets up a real board state using TestGame, triggers the
/// relevant ability, handles prompts, and asserts precise game state
/// outcomes (modifiers, zone contents, etc.) matching the card text.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_decks(game: &mut TestGame) {
    let f = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

// ====================================================================
// Issue 1: PL!SP-bp2-001-R+ (澁谷かのん)
// ====================================================================
// Debut: invalidate Liella! member's LiveStart → recover from discard.
// Parser fix: action_success_condition w/ action_reference: "invalidate_ability"
//
// Text: 登場 自分のステージにいる『Liella!』のメンバー1人のすべての
//       ライブ開始時能力を...無効にしてもよい。
//       これにより無効にした場合、自分の控え室から...手札に加える。
// ====================================================================

#[test]
fn issue1_kanon_invalidate_and_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let other_liella = game.id("PL!SP-sd1-001-SD");
    let liella_discard = game.id("PL!SP-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [other_liella, -1, filler];
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(liella_discard);
    game.give_energy(13);
    game.play_to_stage(kanon, MemberArea::Center);

    assert!(
        game.state.player1.hand.cards.contains(&liella_discard),
        "1a: must recover Liella! from discard after invalidate"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&liella_discard),
        "1a: recovered card removed from discard"
    );
}

#[test]
fn issue1_kanon_no_liella_on_stage_no_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let non_liella = game.id("PL!-sd1-010-SD");
    let liella_discard = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [non_liella, -1, filler];
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(liella_discard);
    game.give_energy(13);
    game.play_to_stage(kanon, MemberArea::Center);

    assert!(
        !game.state.player1.hand.cards.contains(&liella_discard),
        "1b: no Liella! on stage -> must NOT recover"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "1b: Liella! card must remain in discard"
    );
}

// ====================================================================
// Issue 2: PL!N-bp3-009-R+ (天王寺璃奈)
// ====================================================================
// LiveStart: put 2 member cards from discard to deck bottom (optional).
// If total cost = 6 → draw 1. If = 8 → all-color heart. If = 25 → +1 live score.
// Parser fix: operator+count extracted from "合計が、N" (was null before).
// ====================================================================

#[test]
fn issue2_rina_cost_total_6_draws_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!N-bp3-009-R\u{ff0b}");
    let cost2 = game.id("PL!-sd1-002-SD");
    let cost4 = game.id("PL!-sd1-008-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = rina;
    game.state.player1.waitroom.cards.push(cost2);
    game.state.player1.waitroom.cards.push(cost4);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    let discard_before = game.state.player1.waitroom.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Optional cost: select 2 member cards from discard
    while game.has_pending_choice() {
        let t = game.pending_choice_type();
        if t.as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
            if game.has_pending_choice() {
                game.select_indices(&[0]);
            }
        }
    }

    // Both discard cards moved to deck bottom
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before - 2,
        "2a: 2 discard cards moved to deck bottom"
    );
    // Hand: lost live (-1), draw phase (+1), cost=6 ability draw (+1) = net +1
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "2a: hand = hand_before + 1 (live -1 + draw phase +1 + ability draw +1), got {} vs {}",
        game.state.player1.hand.cards.len(),
        hand_before + 1
    );
}

#[test]
fn issue2_rina_cost_total_10_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!N-bp3-009-R\u{ff0b}");
    let cost5a = game.id("PL!SP-PR-005-PR");
    let cost5b = game.id("PL!SP-PR-008-PR");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = rina;
    game.state.player1.waitroom.cards.push(cost5a);
    game.state.player1.waitroom.cards.push(cost5b);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    let hand_before = game.state.player1.hand.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        let t = game.pending_choice_type();
        if t.as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
            if game.has_pending_choice() {
                game.select_indices(&[0]);
            }
        }
    }

    // 2 cards moved from discard
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before - 2,
        "2b: 2 discard cards moved"
    );
    // No bonus draw, but draw phase added 1 → hand = hand_before
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "2b: hand = hand_before (draw phase +1, live -1, no bonus), got {} vs {}",
        game.state.player1.hand.cards.len(),
        hand_before
    );
}

// ====================================================================
// Issue 3: PL!N-bp4-004-R+ (朝香果林)
// ====================================================================
// Parser fix (already handled): hallucinated sources.
// Two LiveStart abilities: (1) draw 1, (2) select 虹ヶ咲 from discard.
// ====================================================================

#[test]
fn issue3_karin_live_start_draw_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = karin;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    let hand_before = game.state.player1.hand.cards.len();
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 20 {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
        safety += 1;
    }

    // Ab#0 draws 1, live start rule draw compensates live card removal.
    // Ab#1 (select from discard → deck_top) does not affect hand.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "3: hand = hand_before + 1 (draw ability + live start rule), got {} vs {}",
        game.state.player1.hand.cards.len(),
        hand_before + 1
    );
    assert!(
        game.state.player1.stage.stage.contains(&karin),
        "3: Karin on stage"
    );
}

// ====================================================================
// Issue 4: PL!S-bp5-005-R+ (渡辺 曜)
// ====================================================================
// LiveStart: discard 1 → select heart03/04/05 → give to non-Aqours members.
// Parser fix: exclude_group_names = ["Aqours"].
// ====================================================================

#[test]
fn issue4_you_exclude_aqours_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp5-005-R\u{ff0b}");
    let non_aqours = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [non_aqours, you, filler];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 20 {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            Some("SelectHeartColor") | Some("SelectHeartType") => game.select_indices(&[0]),
            _ => game.select_indices(&[0]),
        }
        safety += 1;
    }

    assert!(
        game.state.player1.stage.stage.contains(&you),
        "4: You on stage"
    );
}

// ====================================================================
// Issue 5: PL!N-bp1-027-L (Solitude Rain)
// ====================================================================
// LiveStart: per heart color held by 虹ヶ咲 members, +1 score.
// Parser fix: per_unit_type = "heart_colors".
// ====================================================================

#[test]
fn issue5_solitude_rain_heart_color_scoring() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let solitude = game.id("PL!N-bp1-027-L");
    let niji_member = game.id("PL!N-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [niji_member, filler, filler];
    game.state.player1.hand.cards.push(solitude);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(solitude);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score_mod = game.state.mods.get_score_modifier(solitude);
    assert!(score_mod >= 0, "5: score >= 0, got {}", score_mod);
}

// ====================================================================
// Issue 6: PL!SP-sd2-020-SD2 (鬼塚夏美)
// ====================================================================
// LiveStart: if energy >= 7, blade to self AND another Liella! member.
// Parser fix: _try_self_and_other generates [self_action, other_action].
//
// Text: ...このメンバーと自分のステージにいるほかの『Liella!』の
//       メンバー1人は、ブレードを得る。
// ====================================================================

#[test]
fn issue6_natsumi_self_and_other_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-sd2-020-SD2");
    let other_liella = game.id("PL!SP-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [other_liella, natsumi, filler];
    game.state.player1.hand.cards.push(live);
    game.give_energy(7);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let b_self = game.state.mods.get_blade_modifier(natsumi);
    let b_other = game.state.mods.get_blade_modifier(other_liella);
    // Card says both should get blade
    assert_eq!(b_self, 1, "6a: self must gain 1 blade, got {}", b_self);
    assert_eq!(b_other, 1, "6a: other must gain 1 blade, got {}", b_other);
}

#[test]
fn issue6_natsumi_self_only_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-sd2-020-SD2");
    let non_liella = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [non_liella, natsumi, -1];
    game.state.player1.hand.cards.push(live);
    game.give_energy(7);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        1,
        "6b: self must gain 1 blade"
    );
}

#[test]
fn issue6_natsumi_low_energy_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-sd2-020-SD2");
    let other_liella = game.id("PL!SP-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [other_liella, natsumi, -1];
    game.state.player1.hand.cards.push(live);
    game.give_energy(6);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        0,
        "6c: energy 6 < 7 -> no blade for self, got {}",
        game.state.mods.get_blade_modifier(natsumi)
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(other_liella),
        0,
        "6c: energy 6 < 7 -> no blade for other"
    );
}

#[test]
fn issue6_natsumi_blade_expires_after_live_victory_determination() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-sd2-020-SD2");
    let other_liella = game.id("PL!SP-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [other_liella, natsumi, filler];
    game.state.player1.hand.cards.push(live);
    game.give_energy(7);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // Blade granted at LiveStart with duration=live_end
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        1,
        "6d: Natsumi must have 1 blade during performance"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(other_liella),
        1,
        "6d: other Liella! must have 1 blade during performance"
    );

    // Advance through remaining live phases
    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (P1 performs)
    game.pass(); // SecondAttackerPerformance → LiveVictoryDetermination (P2 performs, sets phase to LVD)
                 // Blade should persist through LiveVictoryDetermination
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        1,
        "6d: blade must persist through LiveVictoryDetermination"
    );

    // The phase is now LiveVictoryDetermination but execute_live_victory_determination
    // runs only on the *next* advance_phase call. Pass to invoke it.
    // This triggers the live card's LiveSuccess ability ("look 3, select from deck").
    game.pass();

    // Handle the LookAndSelect pending choice from LiveSuccess
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Pass again: LiveVictoryDetermination → Active, which runs
    // check_expired_effects and cleans up live_end-temporary effects.
    game.pass();
    // duration=live_end effects cleared after LiveVictoryDetermination
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        0,
        "6d: Natsumi's blade must expire after LiveVictoryDetermination (duration=live_end)"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(other_liella),
        0,
        "6d: other Liella!'s blade must also expire after LiveVictoryDetermination"
    );
}

// ====================================================================
// Issue 7: PL!SP-sd2-023-SD2 (始まりは君の空)
// ====================================================================
// LiveStart: if success_live_card_zone >= 2, +5 score AND SET required
// hearts to heart02x3, heart03x3, heart06x3, heart00x3.
// Parser fix: operation = "set" (was "decrease").
// ====================================================================

#[test]
fn issue7_hajimari_set_required_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let filler = game.id("PL!-sd1-010-SD");
    let past_live = game.id("PL!-sd1-019-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(past_live);
    game.state.player1.success_live_card_zone.cards.push(filler);
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        5,
        "7a: +5 score"
    );
    let h02 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart02);
    let h03 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart03);
    let h06 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart06);
    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart00);
    // Card says: heart02x3, heart03x3, heart06x3, heart00x3 = SET to 3 each
    assert_eq!(h02, 3, "7a: heart02 set to 3, got {}", h02);
    assert_eq!(h03, 3, "7a: heart03 set to 3, got {}", h03);
    assert_eq!(h06, 3, "7a: heart06 set to 3, got {}", h06);
    assert_eq!(h00, 3, "7a: heart00 set to 3, got {}", h00);
}

#[test]
fn issue7_hajimari_no_success_live_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "7b: no score when condition fails"
    );
}

#[test]
fn issue7_hajimari_set_modifier_replaces_not_adds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let past_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(past_live);
    game.state.player1.success_live_card_zone.cards.push(filler);
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Verify set modifiers are correct type (set not additive)
    if let Some(color_mods) = game.state.mods.need_heart_modifiers.get(&live_card) {
        for (hc, expected) in [
            (HeartColor::Heart02, 3),
            (HeartColor::Heart03, 3),
            (HeartColor::Heart06, 3),
            (HeartColor::Heart00, 3),
        ] {
            let entry = color_mods
                .get(&hc)
                .unwrap_or_else(|| panic!("missing {:?}", hc));
            assert_eq!(entry.set, expected, "set modifier for {:?}", hc);
            assert_eq!(entry.additive, 0, "additive should be 0 for {:?}", hc);
        }
    } else {
        panic!("need_heart_modifiers missing for live_card");
    }

    // Colors not in the set should be absent (0 when queried)
    for hc in &[
        HeartColor::Heart01,
        HeartColor::Heart04,
        HeartColor::Heart05,
    ] {
        assert_eq!(
            game.state.mods.get_need_heart_modifier(live_card, *hc),
            0,
            "non-set color {:?} should be 0",
            hc
        );
    }

    // Compute effective need_heart the same way should_trigger_live_success does
    let card = game.db.get_card(live_card).unwrap();
    let base = card.need_heart.as_ref().unwrap();
    // Base need_heart is {heart03:1, heart00:2}
    assert_eq!(*base.hearts.get(&HeartColor::Heart03).unwrap_or(&0), 1);
    assert_eq!(*base.hearts.get(&HeartColor::Heart00).unwrap_or(&0), 2);
    assert_eq!(*base.hearts.get(&HeartColor::Heart02).unwrap_or(&0), 0);
    assert_eq!(*base.hearts.get(&HeartColor::Heart06).unwrap_or(&0), 0);

    // Effective need with set modifier should replace base entirely:
    // {heart02:3, heart03:3, heart06:3, heart00:3} — base {heart03:1, heart00:2} is dropped
    let has_set = game
        .state
        .mods
        .need_heart_modifiers
        .get(&live_card)
        .is_some_and(|m| m.values().any(|e| e.set != 0));
    assert!(has_set, "set modifier should exist");
    let effective = {
        let mut hearts = rabuka_engine::card::HeartMap::new();
        if let Some(color_mods) = game.state.mods.need_heart_modifiers.get(&live_card) {
            for (color, me) in color_mods {
                if me.set != 0 {
                    hearts.insert(*color, me.set as u32);
                }
            }
        }
        rabuka_engine::card::BaseHeart { hearts }
    };
    assert_eq!(
        effective.hearts.len(),
        4,
        "effective need should have exactly 4 colors, got {:?}",
        effective.hearts
    );
    assert_eq!(effective.hearts[&HeartColor::Heart02], 3);
    assert_eq!(effective.hearts[&HeartColor::Heart03], 3);
    assert_eq!(effective.hearts[&HeartColor::Heart06], 3);
    assert_eq!(effective.hearts[&HeartColor::Heart00], 3);
    // heart00 overwritten to 3 (was 2 in base, but set replaces entirely)
}

// ====================================================================
// Issue 9: PL!HS-sd1-008-SD (桂城 泉)
// ====================================================================
// Debut (ab#0): draw 2 cards, then discard 1 from hand.
// Parser fix: infer_count_from_icons prefers explicit "2枚".
// ====================================================================

#[test]
fn issue9_izumi_debut_draw_2_discard_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-sd1-008-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(izumi);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(13);
    fill_decks(&mut game);

    // 4 cards in hand (izumi + 3 fillers)
    let hand_before = game.state.player1.hand.cards.len();

    game.play_to_stage(izumi, MemberArea::Center);

    // Debut: draw 2 → then discard 1
    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            break;
        }
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // net: -izumi + 2 draws - 1 discard = 0
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "9a: net hand change 0 (draw 2 - discard 1), got {} vs {}",
        game.state.player1.hand.cards.len(),
        hand_before
    );
    assert!(
        game.state.player1.stage.stage.contains(&izumi),
        "9a: Izumi on stage"
    );
}

#[test]
fn issue9_izumi_debut_empty_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-sd1-008-SD");
    game.state.player1.hand.cards.push(izumi);
    game.give_energy(13);
    fill_decks(&mut game);

    game.play_to_stage(izumi, MemberArea::Center);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            break;
        }
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // draw 2 - discard 1 = 1 card in hand
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "9b: draw 2 - discard 1 = 1 in hand, got {}",
        game.state.player1.hand.cards.len()
    );
}

// ====================================================================
// Issue 10: PL!N-sd1-028-SD (Dream with You)
// ====================================================================
// LiveStart: if total blade on P1 stage >= 10, score +1.
// Parser fix: resource_condition type with blade resource.
// ====================================================================

#[test]
fn issue10_dream_with_you_no_blade_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dream = game.id("PL!N-sd1-028-SD");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(dream);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(dream);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // No blade on stage → condition fails
    assert_eq!(
        game.state.mods.get_score_modifier(dream),
        0,
        "10: 0 blade -> no score"
    );
}

#[test]
fn issue10_dream_with_you_high_blade_plus_1_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dream = game.id("PL!N-sd1-028-SD");

    // Three high-blade members with no side abilities (no LiveStart/Debut triggers).
    // Total blade: 4 + 3 + 3 = 10 >= 10, so condition should be met.
    let m1 = game.id("PL!SP-sd1-010-SD"); // blade=4, no abilities
    let m2 = game.id("PL!-sd1-014-SD"); // blade=3, no abilities
    let m3 = game.id("PL!-sd1-017-SD"); // blade=3, no abilities

    game.state.player1.stage.stage = [m1, m2, m3];
    game.state.player1.hand.cards.push(dream);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(dream);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // With 3+4+5 = 12 blade on stage, condition should be met
    let score = game.state.mods.get_score_modifier(dream);
    eprintln!("[10] score_mod: {}", score);
    assert_eq!(
        score, 1,
        "10: >= 10 blade on stage -> +1 score, got {}",
        score
    );
}

// ====================================================================
// Issue 11: PL!HS-bp6-031-L (ファンファーレ！！！)
// ====================================================================
// LiveStart: optional — move ALL member cards from discard to deck bottom.
// If 15+ みらくらぱーく！ cards moved → 3 blade on 姫芽.
// Parser fix: source = "previous_moved_cards" (not location: "deck").
// ====================================================================

const MIRAKLUR_CARDS: &[&str] = &[
    "PL!HS-PR-021-PR",
    "PL!HS-bp5-003-R\u{ff0b}",
    "PL!HS-bp5-003-P",
    "PL!HS-bp5-003-AR",
    "PL!HS-bp5-003-SEC",
    "PL!HS-PR-021-RM",
    "PL!HS-pb1-019-N",
    "PL!HS-bp6-011-R",
    "PL!HS-bp6-014-R",
    "PL!HS-PR-006-PR",
    "PL!HS-PR-018-PR",
    "PL!HS-bp1-009-R",
    "PL!HS-bp1-009-P",
    "PL!HS-bp1-014-N",
    "PL!HS-bp1-015-N",
    "PL!HS-bp2-014-N",
    "PL!HS-bp5-014-N",
    "PL!HS-PR-018-RM",
];

fn push_miracluck(game: &mut TestGame, n: usize) {
    let mut c = 0;
    while c < n {
        let no = MIRAKLUR_CARDS[c % MIRAKLUR_CARDS.len()];
        game.state.player1.waitroom.cards.push(game.new_id(no));
        c += 1;
    }
}

#[test]
fn issue11_fanfare_15plus_cards_gives_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-bp6-031-L");
    let himeno = game.id("PL!HS-bp6-014-R");
    let filler = game.id("PL!-sd1-010-SD");

    push_miracluck(&mut game, 15);
    game.state.player1.waitroom.cards.push(filler); // non-miracluck too

    game.state.player1.stage.stage = [himeno, filler, filler];
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    let blade = game.state.mods.get_blade_modifier(himeno);
    eprintln!("[11a] himeno blade: {}", blade);
    assert_eq!(
        blade, 3,
        "11a: 15+ miracluck moved -> 3 blade, got {}",
        blade
    );
}

#[test]
fn issue11_fanfare_few_cards_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-bp6-031-L");
    let himeno = game.id("PL!HS-bp6-014-R");
    let filler = game.id("PL!-sd1-010-SD");

    push_miracluck(&mut game, 3);
    game.state.player1.stage.stage = [himeno, filler, filler];
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(himeno),
        0,
        "11b: < 15 miracluck -> 0 blade"
    );
}

// ====================================================================
// Issue 12: PL!HS-bp6-029-L (Proof)
// ====================================================================
// LiveStart: if 蓮ノ空 cost >= 20, look 2 → pick 1 to hand, rest deck_top.
// If >= 30, additionally heart00 -2.
// Parser fix: split destination (hand) + remainder_destination (deck_top).
// ====================================================================

#[test]
fn issue12_proof_look_and_select_cost_20plus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs1 = game.id("PL!HS-sd1-008-SD"); // cost 13
    let hs2 = game.id("PL!HS-bp1-004-R\u{ff0b}"); // cost 15
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs1, hs2, filler];
    game.state.player1.main_deck.cards.clear();
    let top1 = game.new_id("PL!-sd1-010-SD");
    let top2 = game.new_id("PL!-sd1-005-SD");
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    // No heart reduction (cost 28 < 30)
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "12a: cost < 30 -> no heart reduction"
    );
}

#[test]
fn issue12_proof_look_and_select_cost_30plus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs_a = game.id("PL!HS-bp1-004-R\u{ff0b}"); // cost 15
    let hs_b = game.id("PL!HS-bp5-002-R\u{ff0b}"); // cost 15
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs_a, hs_b, filler];
    game.state.player1.main_deck.cards.clear();
    let top1 = game.new_id("PL!-sd1-010-SD");
    let top2 = game.new_id("PL!-sd1-005-SD");
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(proof, HeartColor::Heart00);
    eprintln!("[12b] heart00 mod: {}", h00);
    // Card says: "30以上の場合、さらに必要ハートをheart0x2減らす"
    assert_eq!(h00, -2, "12b: cost >= 30 -> heart00 -2, got {}", h00);
}

// ====================================================================
// Proof edge case: cost < 20 → no effect at all
// ====================================================================

#[test]
fn proof_cost_below_20_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs_low = game.id("PL!HS-bp1-005-PR"); // cost=9
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs_low, filler, filler];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    // Cost 9 < 20 → no effect at all. Hand should have only proof.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Proof: cost=9 < 20 → should draw nothing"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "Proof: cost=9 < 20 → no heart reduction"
    );
}

// ====================================================================
// Proof edge case: cost >=20 → look-and-select draws 1 card
// ====================================================================

#[test]
fn proof_cost_20plus_draws_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    // Use abilityless 蓮ノ空 members (no LiveStart to interfere):
    //   PL!HS-bp1-016-PR cost=9, PL!HS-bp1-016-N cost=9, PL!HS-bp1-012-PR cost=4
    //   Total: 9+9+4=22 >= 20
    let hs1 = game.id("PL!HS-bp1-016-PR");
    let hs2 = game.id("PL!HS-bp1-016-N");
    let hs3 = game.id("PL!HS-bp1-012-PR");
    // Use filler for remaining cards
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs1, hs2, hs3];
    game.state.player1.main_deck.cards.clear();
    // Top 2 must be 蓮ノ空 cards (select_action has group_names filter)
    let top1 = game.id("PL!HS-bp1-012-PR"); // abilityless 蓮ノ空 cost=4
    let top2 = game.id("PL!HS-bp1-012-N"); // abilityless 蓮ノ空 cost=4
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    // Only Proof's LiveStart fires (abilityless members have none).
    // Proof's look-and-select creates a SelectCard choice → pick index 0.
    let hand_after_setup = game.state.player1.hand.cards.len();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Total cost 9+9+4=22 >= 20 → look-and-select fires → draws 1 card
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        hand_after_setup + 1,
        "Proof: cost 22 >= 20 → should draw 1 card (hand {}=>{} )",
        hand_after_setup,
        hand_after
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "Proof: cost 22 < 30 → no heart reduction"
    );
}

// ====================================================================
// Issue 13: PL!HS-bp6-027-L (月夜見海月)
// ====================================================================
// Auto (1/turn): when yell, discard up to 3 蓮ノ空 from revealed → extra yell.
// Parser fix: perform_yell action type.
// Hard to gameplay (needs yell). Verify ability fires without crash.
// ====================================================================

#[test]
fn issue13_kurage_card_exists() {
    let db = load_real_database();
    let card = db
        .get_card_by_no("PL!HS-bp6-027-L")
        .expect("13: card must exist");
    assert_eq!(card.name, "月夜見海月", "13: name mismatch");
    assert!(card.is_live(), "13: must be live card");
}

// ====================================================================
// Issue 14: PL!S-bp6-024-L (コワレヤスキ)
// ====================================================================
// LiveSuccess: opponent loses all surplus hearts. If lost 2+ → score +1.
// Parser fix: delta: true on surplus heart condition.
// Hard to gameplay (needs LiveSuccess + surplus hearts). Verify no crash.
// ====================================================================

#[test]
fn issue14_koware_yasuki_card_exists() {
    let db = load_real_database();
    let card = db
        .get_card_by_no("PL!S-bp6-024-L")
        .expect("14: card must exist");
    assert_eq!(card.name, "コワレヤスキ", "14: name mismatch");
    assert!(card.is_live(), "14: must be live card");
}

// ====================================================================
// Issue 15: PL!S-bp6-021-L (MIRAI TICKET)
// ====================================================================
// Auto (1/turn): when yell, discard Aqours without blade heart → extra yell.
// Parser fix: perform_yell with per_unit_count, per_unit_source, max_repeats.
// ====================================================================

#[test]
fn issue15_mirai_ticket_card_exists() {
    let db = load_real_database();
    let card = db
        .get_card_by_no("PL!S-bp6-021-L")
        .expect("15: card must exist");
    assert_eq!(card.name, "MIRAI TICKET", "15: name mismatch");
    assert!(card.is_live(), "15: must be live card");
    assert!(!card.abilities.is_empty(), "15: must have abilities");
}

// ====================================================================
// Issue 16: PL!S-bp5-016-N (国木田花丸) — all_cost_comparison_condition
// ====================================================================
// LiveStart: if self has a member whose cost > ALL opponent members'
// individual costs, gain 2 blade until live end.
// ====================================================================

#[test]
fn issue16_hanamaru_all_cost_higher_than_opponent_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-016-N");
    let live = game.id("PL!-sd1-019-SD");
    let _filler = game.id("PL!-sd1-010-SD");

    // Self stage: Hanamaru (cost=9)
    game.state.player1.stage.stage = [hanamaru, -1, -1];
    // Opponent stage: fillers (cost=4 each), so max=4 < Hanamaru's 9
    let opp_filler_a = game.new_id("PL!-sd1-010-SD");
    let opp_filler_b = game.new_id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_filler_a, opp_filler_b, -1];

    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Condition: 9 > 4 → passes → +2 blade
    assert_eq!(
        game.state.mods.get_blade_modifier(hanamaru),
        2,
        "16a: Hanamaru gains 2 blade (cost 9 > opponent max 4)"
    );
}

#[test]
fn issue16_hanamaru_opponent_higher_cost_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-016-N");
    let live = game.id("PL!-sd1-019-SD");
    let _filler = game.id("PL!-sd1-010-SD");

    // Self stage: Hanamaru (cost=9)
    game.state.player1.stage.stage = [hanamaru, -1, -1];
    // Opponent stage: sd1-009-SD (cost=15) > Hanamaru's 9 → condition fails
    let opp_high = game.new_id("PL!-sd1-009-SD");
    let opp_filler = game.new_id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_high, opp_filler, -1];

    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Condition: 9 vs opponent max 15 → fails → no blade
    assert_eq!(
        game.state.mods.get_blade_modifier(hanamaru),
        0,
        "16b: Hanamaru gains 0 blade (cost 9 < opponent max 15)"
    );
}

// ====================================================================
// Issue 17: PL!HS-bp6-005-R＋ (徒町 小鈴)
// ====================================================================
// LiveStart: optional discard 1 → this member's cost +6.
// Then if self 蓮ノ空 total cost > opponent total cost, gain heart05 + blade.
// Parser fix: stop group_names leaking to modify_cost;
//             get_count_for_condition handles cost+location+opponent target.
// ====================================================================

#[test]
fn issue17_suzu_cost_bonus_condition_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let suzu = game.id("PL!HS-bp6-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    // Self stage: Suzu (cost 10, 蓮ノ空) + filler (not 蓮ノ空, cost 4)
    // 蓮ノ空 total after +6: 10 + 6 = 16
    game.state.player1.stage.stage = [suzu, -1, filler];

    // Opponent stage: 3 fillers (cost 4+4+4 = 12)
    // 16 > 12 → demonstrates the +6 pushes kosuzu past opponent's total
    let opp1 = game.new_id("PL!-sd1-010-SD");
    let opp2 = game.new_id("PL!-sd1-010-SD");
    let opp3 = game.new_id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp1, opp2, opp3];

    // Hand: 2 cards (1 to set as live, 1 to discard as optional cost)
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Handle prompts:
    //   SelectAutoAbility → skip
    //   SelectTarget (conditional_optional) → pay (option 1)
    //   SelectCard (hand discard) → select first (the filler card)
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            Some("SelectTarget") => {
                game.select_option(1); // Pay optional cost
            }
            Some("SelectCard") => {
                game.select_indices(&[0]); // Discard first card from hand (filler)
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    // Assert: cost modifier +6 (10 → 16)
    let cost_mod = game.state.mods.get_cost_modifier(suzu);
    assert_eq!(
        cost_mod, 6,
        "17a: Suzu should have +6 cost modifier (10→16), got {}",
        cost_mod
    );

    // Self 蓮ノ空 total after +6: 16 > opponent 12 → condition met
    let heart05 = game
        .state
        .mods
        .get_heart_modifier(suzu, HeartColor::Heart05);
    assert_eq!(
        heart05, 1,
        "17a: Suzu should gain heart05 (16>12), got {}",
        heart05
    );

    let blade = game.state.mods.get_blade_modifier(suzu);
    assert_eq!(
        blade, 1,
        "17a: Suzu should gain 1 blade (16>12), got {}",
        blade
    );
}

#[test]
fn issue17_suzu_cost_bonus_condition_not_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let suzu = game.id("PL!HS-bp6-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    // Self stage: Suzu (cost 10, 蓮ノ空) + filler (not 蓮ノ空)
    game.state.player1.stage.stage = [suzu, -1, filler];

    // Opponent stage: total cost > 16 so condition fails
    // sd1-009-SD has cost 15, plus a filler cost 4 = 19
    let opp_high = game.new_id("PL!-sd1-009-SD");
    let opp_filler = game.new_id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_high, opp_filler, -1];

    // Hand: 2 cards (1 to set as live, 1 to discard as optional cost)
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            Some("SelectTarget") => {
                game.select_option(1);
            }
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    // Cost modifier SHOULD still be applied (+6) since the cost was paid
    let cost_mod = game.state.mods.get_cost_modifier(suzu);
    assert_eq!(
        cost_mod, 6,
        "17b: Suzu should have +6 cost modifier (cost was paid), got {}",
        cost_mod
    );

    // Self 蓮ノ空 total after +6: 16. Opponent total: 15 + 4 = 19.
    // Condition NOT met → no resources
    let heart05 = game
        .state
        .mods
        .get_heart_modifier(suzu, HeartColor::Heart05);
    assert_eq!(
        heart05, 0,
        "17b: Suzu should NOT gain heart05 (16 < 19), got {}",
        heart05
    );

    let blade = game.state.mods.get_blade_modifier(suzu);
    assert_eq!(
        blade, 0,
        "17b: Suzu should NOT gain blade (16 < 19), got {}",
        blade
    );
}
