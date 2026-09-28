/// Integration tests for energy-under-member and member-under-member mechanics.
///
/// Rules verified: 4.5.5, 4.5.5.3, 10.5.3, 10.5.4, Q157, Q184
///
/// Remaining items for future work (not blocking functionality):
///   - Parser: emit "under_member" for 下にある per-unit references
///   - Web UI: under-cards get energy-type/member-type CSS class
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn place_energy_under(game: &mut TestGame, area: MemberArea, count: usize) {
    let energy_id = game.id("LL-E-001-SD");
    for _ in 0..count {
        game.state.player1.stage.place_under_card(area, energy_id);
    }
}

fn seed_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ====================================================================
// PL!N-pb1-002 中須かすみ (cost 13) — Debut: place 2 energy under (optional)
// ====================================================================

#[test]
fn kasumi_debut_skip_optional_no_energy_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-pb1-002-R");
    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(13);
    game.play_to_stage(kasumi, MemberArea::Center);
    assert!(game.has_pending_choice(), "should prompt optional cost");
    game.select_indices(&[]);
    assert_eq!(game.state.player1.energy_zone.cards.len(), 13);
    assert_eq!(game.state.player1.stage.stage[1], kasumi);
}

#[test]
fn kasumi_debut_place_two_energy_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-pb1-002-R");
    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(13);
    game.play_to_stage(kasumi, MemberArea::Center);
    assert!(game.has_pending_choice(), "should prompt optional cost");
    game.select_energy_from_zone(2); // select the 2 energy cards to place under
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        11,
        "2 energy removed from zone"
    );
    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert_eq!(under.len(), 2, "2 energy under center member");
}

#[test]
fn kasumi_q184_under_energy_not_counted_in_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-pb1-002-R");
    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(13);
    game.play_to_stage(kasumi, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_energy_from_zone(2); // select the 2 energy cards to place under
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        11,
        "Q184: 2 energy under member, 11 in zone"
    );
    // Verify under member
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        2,
        "2 energy under center member"
    );
}

#[test]
fn kasumi_rule_1054_under_energy_recycles_to_energy_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-pb1-002-R");
    game.state.player1.stage.stage[1] = kasumi;
    place_energy_under(&mut game, MemberArea::Center, 2);
    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    game.state
        .player1
        .remove_member_from_stage_with_recycling(1, &game.db);
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        energy_deck_before + 2,
        "Rule 10.5.4: 2 energy returned to energy deck"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
}

#[test]
fn kasumi_rule_4553_under_cards_follow_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-pb1-002-R");
    let left = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [left, kasumi, -1];
    place_energy_under(&mut game, MemberArea::Center, 2);
    game.state
        .player1
        .stage
        .position_change(MemberArea::Center, MemberArea::LeftSide)
        .ok();
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::LeftSide)
            .len(),
        2
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
    assert_eq!(game.state.player1.stage.stage[0], kasumi);
}

// ====================================================================
// PL!N-pb1-011 ミア・テイラー (cost 15) — Activation: place 1 energy under, retrieve 虹ヶ咲 live
// ====================================================================

#[test]
fn activation_under_energy_cost_removes_one_energy_from_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-pb1-011-R");
    let niji_live = game.id("PL!N-sd1-026-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, mia, -1];
    game.state.player1.waitroom.cards.push(niji_live);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(3);
    let energy_before = game.state.player1.energy_zone.cards.len();
    game.activate_ability(mia);
    // Cost: place 1 energy under member — must be offered
    assert!(
        game.has_pending_choice(),
        "mia cost (place 1 energy under) must offer energy select"
    );
    game.select_indices(&[0]);
    let energy_after = game.state.player1.energy_zone.cards.len();
    let under = game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::Center)
        .len();
    assert_eq!(
        energy_after,
        energy_before - 1,
        "cost removes 1 energy from zone"
    );
    assert_eq!(under, 1, "1 energy placed under member");
}

#[test]
fn activation_paid_under_energy_recovers_exact_nijigasaki_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-pb1-011-R");
    let niji_live = game.id("PL!N-sd1-026-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, mia, -1];
    game.state.player1.waitroom.cards.push(niji_live);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(3);
    let energy = game.state.player1.energy_zone.cards[0];
    game.activate_ability(mia);
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.energy_zone.cards.len(), 2);
    assert!(!game.state.player1.energy_zone.cards.contains(&energy));
    assert_eq!(game.state.player1.stage.get_under_cards(MemberArea::Center), &[energy]);
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[filler, niji_live]);
    assert!(game.state.player1.waitroom.cards.is_empty());
}

#[test]
fn activation_once_per_turn_blocks_second_under_energy_payment_and_live_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-pb1-011-R");
    let niji1 = game.id("PL!N-sd1-026-SD");
    let niji2 = game.id("PL!N-sd1-027-SD");
    game.state.player1.stage.stage = [-1, mia, -1];
    game.state.player1.waitroom.cards.push(niji1);
    game.state.player1.waitroom.cards.push(niji2);
    game.state
        .player1
        .hand
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.give_energy(5);
    game.activate_ability(mia);
    game.select_energy_from_zone(1); // pay the 1-energy cost
    // Must offer live retrieval selection
    assert!(
        game.has_pending_choice(),
        "mia must offer niji live retrieval selection"
    );
    game.select_indices(&[0]); // select which niji live to retrieve
    assert!(
        game.state.player1.hand.cards.contains(&niji1),
        "First activation works"
    );
    // use_limit=1: second activation is blocked by use_limit check
    let hand_before = game.state.player1.hand.cards.len();
    let energy_before = game.state.player1.energy_zone.cards.len()
        + game
            .state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len();
    let zone_before = game.state.player1.energy_zone.cards.clone();
    let under_before = game.state.player1.stage.get_under_cards(MemberArea::Center).to_vec();
    assert!(game.try_activate_ability(mia).is_err());
    assert_eq!(game.state.player1.energy_zone.cards, zone_before);
    assert_eq!(game.state.player1.stage.get_under_cards(MemberArea::Center), under_before.as_slice());
    // Verify no change — use_limit blocked effect AND cost
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "use_limit=1 prevented second activation from retrieving another live"
    );
    let energy_after = game.state.player1.energy_zone.cards.len()
        + game
            .state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len();
    assert_eq!(
        energy_after, energy_before,
        "use_limit=1 prevented second activation cost from being paid"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&niji2),
        "use_limit=1 prevented second activation from retrieving another live"
    );
}

// ====================================================================
// PL!N-bp3-013-N 上原歩夢 (cost 9) — Debut: place 1 energy under (optional) → draw 2
// ====================================================================

#[test]
fn ayumu_bp3n_debit_place_one_under_draw_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-bp3-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    // Add extra fillers to hand so we can track card count after play_to_stage moves the played card
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(9);
    let hand_before = game.state.player1.hand.cards.len(); // 3 (ayumu + 2 filler)
    let energy_before = game.state.player1.energy_zone.cards.len();
    game.play_to_stage(ayumu, MemberArea::Center);
    // play_to_stage removes ayumu from hand → hand_before - 1 = 2
    assert!(game.has_pending_choice(), "should prompt optional cost");
    game.select_energy_from_zone(1); // place 1 energy under
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before - 1,
        "1 energy removed from zone"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        1,
        "1 energy under center member"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "2 cards drawn (+1 net from hand before play)"
    );
}

#[test]
fn ayumu_bp3n_debut_skip_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-bp3-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(9);
    let hand_before = game.state.player1.hand.cards.len(); // 3
    game.play_to_stage(ayumu, MemberArea::Center);
    // play_to_stage removes ayumu → 2 cards in hand
    assert!(game.has_pending_choice(), "should prompt");
    game.select_indices(&[]); // skip
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "0 cards drawn when skipped (2 remain = 3 before - 1 for play)"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
}

#[test]
fn ayumu_bp3n_q157_wait_energy_placed_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-bp3-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(9);
    // Add wait energy (pushed at end = top of pop)
    game.state
        .player1
        .energy_zone
        .cards
        .push(game.id("LL-E-001-SD"));
    game.play_to_stage(ayumu, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_energy_from_zone(1); // place 1 energy under (the wait energy)
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        1,
        "Q157: wait energy placed under"
    );
}

// ====================================================================
// PL!N-bp5-013-N 上原歩夢 (cost 2) — LiveStart: if energy under any member → heart01
// ====================================================================

#[test]
fn live_start_with_under_energy_grants_one_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-bp5-013-N");
    let filler_live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, ayumu, -1];
    game.state.player1.hand.cards.push(filler_live);
    seed_deck(&mut game);
    game.give_energy(3);
    place_energy_under(&mut game, MemberArea::Center, 1);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);
    let heart = game
        .state
        .mods
        .get_heart_modifier(ayumu, rabuka_engine::card::HeartColor::Heart01);
    assert_eq!(
        heart, 1,
        "heart01=1 on Ayumu after LiveStart condition passes"
    );
}

// ====================================================================
// PL!N-PR-026-PR 天王寺璃奈 (cost ~15) — Debut: place member under; gains LiveSuccess
// ====================================================================

#[test]
fn rina_debit_triggers_with_target_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let rina = game.id("PL!N-PR-026-PR");
    let target = game.id("PL!N-PR-009-PR"); // 優木せつ菜, cost 2 (<=9)
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = rina;
    game.state.player1.waitroom.cards.push(target);
    game.state.player1.waitroom.cards.push(filler);
    game.give_energy(3);
    // Fire the debut for real (staging her would cost ~15 energy).
    // Single eligible candidate (only Setsuna is cost<=9 Nijigasaki), so
    // the engine auto-picks with no prompt.
    fire_trigger(
        &mut game,
        rina,
        rabuka_engine::core::types::AbilityTrigger::Debut,
        "登場",
    );
    scan_autos_both(&mut game);

    assert!(
        !game.state.player1.waitroom.cards.contains(&target),
        "target leaves the waitroom"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        1,
        "exactly the cost<=9 member parked under Rina"
    );
    assert!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .contains(&target),
        "the parked card is the target"
    );
}

#[test]
fn rina_rule_1053_under_member_goes_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!N-PR-026-PR");
    let target = game.id("PL!N-PR-009-PR");
    game.state.player1.stage.stage[1] = rina;
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, target);
    game.state
        .player1
        .remove_member_from_stage_with_recycling(1, &game.db);
    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "Rule 10.5.3: under member goes to waitroom"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
}

#[test]
fn rina_rule_4553_under_member_follows_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!N-PR-026-PR");
    let target = game.id("PL!N-PR-009-PR");
    let other = game.id("PL!-sd1-013-SD");
    game.state.player1.stage.stage = [other, rina, -1];
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, target);
    game.state
        .player1
        .stage
        .position_change(MemberArea::Center, MemberArea::LeftSide)
        .ok();
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::LeftSide)
            .len(),
        1
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
    assert_eq!(game.state.player1.stage.stage[0], rina);
}

// ====================================================================
// PL!HS-pb1-002 村野さやか — Activation: reveal & place under; LiveStart: per-under heart05
// ====================================================================

#[test]
fn activation_reveals_same_name_hand_member_and_moves_it_under_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    let sayaka_hand = game.new_id("PL!HS-pb1-002-R");
    game.state.player1.stage.stage = [-1, sayaka, -1];
    game.state.player1.hand.cards.push(sayaka_hand);
    game.state
        .player1
        .hand
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.give_energy(3);
    game.activate_ability(sayaka);
    // Cost: reveal 1 sayaka from hand → must be offered (select the first sayaka)
    assert!(
        game.has_pending_choice(),
        "sayaka cost (reveal sayaka from hand) must offer hand select"
    );
    game.select_indices(&[0]);
    // Effect: move revealed card under sayaka (Center)
    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert_eq!(
        under.len(),
        1,
        "1 card should be under sayaka after reveal→under_member"
    );
    assert_eq!(
        under[0], sayaka_hand,
        "The revealed sayaka should be under sayaka"
    );
    // Card must NOT remain in hand (duplication bug fix)
    assert!(
        !game.state.player1.hand.cards.contains(&sayaka_hand),
        "revealed card should be removed from hand, not duplicated"
    );
}

#[test]
fn sayaka_use_limit_enforced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    let s1 = game.new_id("PL!HS-pb1-002-R");
    let s2 = game.new_id("PL!HS-pb1-002-R");
    game.state.player1.stage.stage = [-1, sayaka, -1];
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.give_energy(3);
    game.activate_ability(sayaka);
    // First activation: reveal cost choice must be offered
    assert!(
        game.has_pending_choice(),
        "sayaka first activation must offer reveal cost choice"
    );
    game.select_indices(&[0]);
    let under_after_first = game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::Center)
        .len();
    assert_eq!(
        under_after_first, 1,
        "1 card should be under sayaka after first activation"
    );
    // Second activation should fail due to use_limit=1
    let result = game.try_activate_ability(sayaka);
    assert!(
        result.is_err(),
        "use_limit=1 now enforced for Sayaka activation"
    );
    let under_after_second = game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::Center)
        .len();
    assert_eq!(
        under_after_second, under_after_first,
        "no additional card placed under"
    );
}

#[test]
fn live_start_three_under_members_grant_three_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    game.state.player1.stage.stage = [-1, sayaka, -1];
    let filler_live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(filler_live);
    game.state
        .player1
        .hand
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    game.give_energy(3);
    for _ in 0..3 {
        let s = game.new_id("PL!HS-pb1-002-R");
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::Center, s);
    }
    seed_deck(&mut game);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);
    let heart = game
        .state
        .mods
        .get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05);
    assert_eq!(
        heart, 3,
        "3 heart05 from 3 members under (per_unit_type=under_member)"
    );
}

#[test]
fn two_live_start_sayaka_instances_count_only_their_own_under_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka_left = game.id("PL!HS-pb1-002-R");
    let sayaka_center = game.new_id("PL!HS-pb1-002-R");
    game.state.player1.stage.stage = [sayaka_left, sayaka_center, -1];
    for _ in 0..3 {
        let under = game.new_id("PL!HS-pb1-002-R");
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::LeftSide, under);
    }
    for _ in 0..3 {
        let under = game.new_id("PL!HS-pb1-002-R");
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::Center, under);
    }

    trigger_sayaka_live_start(&mut game, sayaka_left);
    trigger_sayaka_live_start(&mut game, sayaka_center);

    for sayaka in [sayaka_left, sayaka_center] {
        assert_eq!(
            game.state
                .mods
                .get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05),
            3
        );
        assert_eq!(game.state.mods.get_cost_modifier(sayaka), 12);
    }
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::LeftSide)
            .len(),
        3
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        3
    );
}

#[test]
fn live_start_no_under_material_ignores_same_name_in_hand_and_grants_no_heart_or_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    let material = game.new_id("PL!HS-pb1-002-R");
    let live = game.id("PL!-sd1-020-SD");
    seed_deck(&mut game);
    game.state.player1.hand.cards.extend([sayaka, material, live]);
    game.give_energy(2);
    assert_eq!(game.db.get_card(sayaka).unwrap().card_no, "PL!HS-pb1-002-R");
    assert_eq!(game.db.get_card(sayaka).unwrap().cost, Some(2));
    game.play_to_stage(sayaka, MemberArea::Center);
    assert_eq!(game.state.player1.stage.stage, [-1, sayaka, -1]);
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[material, live]);
    assert!(game.state.player1.stage.get_under_cards(MemberArea::Center).is_empty());
    assert_eq!(game.state.mods.get_cost_modifier(sayaka), 0);
    assert_eq!(game.state.mods.get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05), 0);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [-1, sayaka, -1]);
    assert!(game.state.player1.stage.get_under_cards(MemberArea::Center).is_empty());
    assert_eq!(game.state.player1.hand.cards.iter().filter(|&&id| id == material).count(), 1);
    assert!(!game.state.player1.waitroom.cards.contains(&material));
    assert_eq!(game.state.mods.get_cost_modifier(sayaka), 0);
    assert_eq!(game.state.mods.get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05), 0);
}

/// Q243: Sayaka's LiveStart max=3 per activation. Second activation re-counts
/// under-cards independently. If more cards were placed between activations,
/// the second activation can count those (up to 3).
///
/// Uses direct `trigger_auto_ability` to simulate COMPASS activating Sayaka's
/// LiveStart ability outside a live performance — the ability itself (cost+4,
/// heart05 per under) works regardless of whether a live is in progress.
fn trigger_sayaka_live_start(game: &mut TestGame, sayaka: i16) {
    let card = game.db.get_card(sayaka).unwrap();
    let live_start_ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("Sayaka must have LiveStart ability");
    let ability_id = format!("{}_{}", card.card_no, live_start_ab.full_text);
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        game.state.player1.id.clone(),
        Some(card.card_no.to_string()),
        Some(sayaka),
        None,
        None,
    );
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn sayaka_q243_max_three_per_activation_recounts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    game.state.player1.stage.stage = [-1, sayaka, -1];
    let _filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);

    // Place 2 under-cards before first activation
    for _ in 0..2 {
        let s = game.new_id("PL!HS-pb1-002-R");
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::Center, s);
    }
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        2,
        "2 under-cards before first LiveStart"
    );

    // First activation: 2 under → max=3 → heart05+2
    trigger_sayaka_live_start(&mut game, sayaka);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    let heart_1 = game
        .state
        .mods
        .get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05);
    assert_eq!(
        heart_1, 2,
        "Q243: First activation: 2 under → heart05=2 (within max=3)"
    );

    // Place 3 more under-cards (now 5 total)
    for _ in 0..3 {
        let s = game.new_id("PL!HS-pb1-002-R");
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::Center, s);
    }
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        5,
        "5 under-cards before second LiveStart"
    );

    // Second activation: 5 under → max=3 → heart05+3 (new cards counted)
    trigger_sayaka_live_start(&mut game, sayaka);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    let heart_total = game
        .state
        .mods
        .get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05);
    // 2 from first + 3 from second = 5 (each capped at 3 independently)
    assert_eq!(
        heart_total, 5,
        "Q243: Two activations: total heart05 = 5 (2 first + 3 second)"
    );

    // Edge: third activation with no new under-cards → still counts 3
    trigger_sayaka_live_start(&mut game, sayaka);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    let heart_final = game
        .state
        .mods
        .get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05);
    // 2 + 3 + 3 = 8 (each activation independently capped at 3)
    assert_eq!(
        heart_final, 8,
        "Q243: Three activations: total heart05 = 8 (2+3+3)"
    );
}

#[test]
fn reveal_same_name_under_then_live_start_grants_one_heart05_and_four_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    let material = game.new_id("PL!HS-pb1-002-R");
    let live = game.id("PL!-sd1-020-SD");
    seed_deck(&mut game);
    game.state.player1.hand.cards.extend([sayaka, material, live]);
    game.give_energy(2);
    assert_eq!(game.db.get_card(sayaka).unwrap().card_no, "PL!HS-pb1-002-R");
    assert_eq!(game.db.get_card(sayaka).unwrap().cost, Some(2));
    game.play_to_stage(sayaka, MemberArea::Center);
    assert_eq!(game.state.player1.stage.stage, [-1, sayaka, -1]);
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);
    assert!(game.state.player1.stage.get_under_cards(MemberArea::Center).is_empty());
    assert_eq!(game.state.mods.get_cost_modifier(sayaka), 0);
    assert_eq!(game.state.mods.get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05), 0);
    game.activate_ability(sayaka);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.get_under_cards(MemberArea::Center), &[material]);
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[live]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(game.state.mods.get_cost_modifier(sayaka), 0);
    assert_eq!(game.state.mods.get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05), 0);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.get_under_cards(MemberArea::Center), &[material]);
    assert!(!game.state.player1.hand.cards.contains(&material));
    assert!(!game.state.player1.waitroom.cards.contains(&material));
    assert_eq!(game.state.mods.get_cost_modifier(sayaka), 4);
    assert_eq!(game.state.mods.get_heart_modifier(sayaka, rabuka_engine::card::HeartColor::Heart05), 1);
}

#[test]
fn sayaka_rule_4553_under_members_follow_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    let other = game.id("PL!-sd1-013-SD");
    game.state.player1.stage.stage = [other, sayaka, -1];
    let s1 = game.new_id("PL!HS-pb1-002-R");
    let s2 = game.new_id("PL!HS-pb1-002-R");
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, s1);
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, s2);
    game.state
        .player1
        .stage
        .position_change(MemberArea::Center, MemberArea::LeftSide)
        .ok();
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::LeftSide)
            .len(),
        2
    );
    assert!(game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::LeftSide)
        .contains(&s1));
    assert!(game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::LeftSide)
        .contains(&s2));
    assert_eq!(game.state.player1.stage.stage[0], sayaka);
}

#[test]
fn sayaka_rule_1053_under_members_go_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-002-R");
    let s1 = game.new_id("PL!HS-pb1-002-R");
    game.state.player1.stage.stage[1] = sayaka;
    game.state
        .player1
        .stage
        .place_under_card(MemberArea::Center, s1);
    game.state
        .player1
        .remove_member_from_stage_with_recycling(1, &game.db);
    assert!(game.state.player1.waitroom.cards.contains(&s1));
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
}

#[test]
fn member_with_under_energy_moving_to_hand_recycles_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = member;
    place_energy_under(&mut game, MemberArea::Center, 2);
    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    let removed = game
        .state
        .player1
        .remove_member_from_stage_with_recycling(1, &game.db)
        .expect("member should be removed");
    game.state.player1.hand.add_card(removed);

    assert!(game.state.player1.hand.cards.contains(&member));
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        energy_deck_before + 2,
        "Q140: under-energy returns to the energy deck when the member moves to hand"
    );
    assert!(game.state.player1.stage.get_under_cards(MemberArea::Center).is_empty());
}

#[test]
fn baton_touch_recycles_owner_under_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let target = game.id("PL!-sd1-010-SD");
    let arriver = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = target;
    place_energy_under(&mut game, MemberArea::Center, 2);
    game.state.player1.hand.cards.push(arriver);
    game.give_energy(4);
    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    game.play_to_stage(arriver, MemberArea::Center);

    assert_eq!(game.state.player1.stage.stage[1], arriver);
    assert!(game.state.player1.waitroom.cards.contains(&target));
    assert_eq!(game.state.player1.energy_deck.cards.len(), energy_deck_before + 2);
    assert!(game.state.player1.stage.get_under_cards(MemberArea::Center).is_empty());
}
// PL!N-bp3-025-L Awakening Promise — LiveStart: under_member → energy_deck → hearts
// ====================================================================

/// 1 member center with 2 energy under. Select 1 → 1 moves, gain heart02×3.
#[test]
fn awakening_move_1_of_2() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let a = g.id("PL!N-bp3-025-L");
    let t = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, t, -1];
    place_energy_under(&mut g, MemberArea::Center, 2);
    g.state.player1.hand.cards.push(a);
    let energy_before = g.state.player1.energy_deck.cards.len();
    advance_to_live_card_set_p1(&mut g);
    g.set_live_card(a);
    advance_to_live_start(&mut g);
    assert!(g.has_pending_choice(), "choice expected");
    assert_eq!(g.pending_choice_type().as_deref(), Some("SelectCard"), "expected SelectCard");
    g.select_indices(&[0]);
    // Up-to-N optional cost: after moving 1 of 2 under-energy, engine re-prompts
    // "Select more energy cards (or skip to finish)"; answering empty finalizes.
    assert!(g.has_pending_choice(), "'select more' re-prompt expected");
    assert_eq!(g.pending_choice_type().as_deref(), Some("SelectCard"), "expected SelectCard");
    g.select_indices(&[]);
    assert_eq!(
        g.state.player1.energy_deck.cards.len(),
        energy_before + 1,
        "1 energy moved"
    );
    assert_eq!(
        g.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        1,
        "1 remains"
    );
    let heart = g
        .state
        .mods
        .get_heart_modifier(t, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(heart, 3, "3 heart02 (1 energy × 3 per card)");
}

/// Skip → 0 moved, no heart bonus.
#[test]
fn awakening_skip_all() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let a = g.id("PL!N-bp3-025-L");
    let t = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, t, -1];
    place_energy_under(&mut g, MemberArea::Center, 2);
    g.state.player1.hand.cards.push(a);
    let energy_before = g.state.player1.energy_deck.cards.len();
    advance_to_live_card_set_p1(&mut g);
    g.set_live_card(a);
    advance_to_live_start(&mut g);
    assert!(g.has_pending_choice(), "choice expected");
    g.select_indices(&[]);
    assert_eq!(
        g.state.player1.energy_deck.cards.len(),
        energy_before,
        "0 moved on skip"
    );
    let heart = g
        .state
        .mods
        .get_heart_modifier(t, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(heart, 0, "no heart bonus when skipped");
}

/// No energy under → no choice, no bonus.
#[test]
fn awakening_no_energy_nothing_happens() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let a = g.id("PL!N-bp3-025-L");
    let t = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, t, -1];
    g.state.player1.hand.cards.push(a);
    advance_to_live_card_set_p1(&mut g);
    g.set_live_card(a);
    advance_to_live_start(&mut g);
    assert!(!g.has_pending_choice(), "no choice when 0 energy under");
    let heart = g
        .state
        .mods
        .get_heart_modifier(t, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(heart, 0, "no heart bonus when no energy under");
}

/// 「そうした場合、そのメンバーは…」 — only THE member whose energy moved gains,
/// even when another stage member also has energy under them.
#[test]
fn awakening_targets_energy_owner_member_only() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let a = g.id("PL!N-bp3-025-L");
    let owner = g.id("PL!-sd1-010-SD");
    let bystander = g.id("PL!-sd1-011-SD");
    g.state.player1.stage.stage = [-1, owner, bystander];
    place_energy_under(&mut g, MemberArea::Center, 2);
    place_energy_under(&mut g, MemberArea::RightSide, 2);
    g.state.player1.hand.cards.push(a);
    let energy_before = g.state.player1.energy_deck.cards.len();
    advance_to_live_card_set_p1(&mut g);
    g.set_live_card(a);
    advance_to_live_start(&mut g);
    assert!(g.has_pending_choice(), "choice expected");
    assert_eq!(g.pending_choice_type().as_deref(), Some("SelectCard"), "expected SelectCard");
    // Global index 0 = first card under the CENTER member (slot order 0..3).
    g.select_indices(&[0]);
    // Up-to-N optional cost: after moving 1 of 2 under-energy, engine re-prompts
    // "Select more energy cards (or skip to finish)"; answering empty finalizes.
    assert!(g.has_pending_choice(), "'select more' re-prompt expected");
    assert_eq!(g.pending_choice_type().as_deref(), Some("SelectCard"), "expected SelectCard");
    g.select_indices(&[]);
    assert_eq!(
        g.state.player1.energy_deck.cards.len(),
        energy_before + 1,
        "1 energy moved"
    );
    let heart02 = rabuka_engine::card::HeartColor::Heart02;
    assert_eq!(
        g.state.mods.get_heart_modifier(owner, heart02),
        3,
        "energy owner member gains heart02 x3"
    );
    assert_eq!(
        g.state.mods.get_heart_modifier(bystander, heart02),
        0,
        "bystander member with its own under-energy gains nothing"
    );
}

/// Move 2 energy → 6 heart02 (2 × 3).
#[test]
fn awakening_move_all_energy() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let a = g.id("PL!N-bp3-025-L");
    let t = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, t, -1];
    place_energy_under(&mut g, MemberArea::Center, 2);
    g.state.player1.hand.cards.push(a);
    let energy_before = g.state.player1.energy_deck.cards.len();
    advance_to_live_card_set_p1(&mut g);
    g.set_live_card(a);
    advance_to_live_start(&mut g);
    assert!(g.has_pending_choice(), "choice expected");
    assert_eq!(g.pending_choice_type().as_deref(), Some("SelectCard"), "expected SelectCard");
    g.select_indices(&[0]);
    // Up-to-N optional cost re-prompts after each pick ("Select more energy cards
    // (or skip to finish)") and asks once more with no eligible cards remaining;
    // the final empty answer finalizes.
    assert!(g.has_pending_choice(), "'select more' re-prompt expected");
    assert_eq!(g.pending_choice_type().as_deref(), Some("SelectCard"), "expected SelectCard");
    g.select_indices(&[0]);
    // Observed: selecting the LAST eligible under-energy auto-finalizes the
    // up-to-N chain — the engine issues no further ask once no candidates remain.
    assert!(!g.has_pending_choice(), "chain auto-finalizes after last card moved");
    assert_eq!(
        g.state.player1.energy_deck.cards.len(),
        energy_before + 2,
        "2 energy moved"
    );
    assert_eq!(
        g.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0,
        "0 remains"
    );
    let heart = g
        .state
        .mods
        .get_heart_modifier(t, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(heart, 6, "6 heart02 (2 energy × 3 per card)");
}
