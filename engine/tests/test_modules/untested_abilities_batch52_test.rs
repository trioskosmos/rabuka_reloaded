/// Untested-abilities batch 52 — named look-fetch, optional-cost retrievals,
/// area-move gates, granted hearts & energy-comparison alternatives.
///
/// - PL!N-pb1-018-R 近江彼方 (登場): look at top 2; optionally reveal a
///   「近江彼方」 member to hand, rest to waitroom.
/// - PL!SP-sd1-007-SD 米女メイ (登場, opt. {E}{E}): retrieve a 『Liella!』
///   member from the waitroom to hand.
/// - PL!SP-sd2-006-SD2 桜小路きな子 (起動 turn1, {E}{E}+hand cost): retrieve a
///   『Liella!』 live card from the waitroom to hand.
/// - PL!-PR-004-PR 園田海未 (起動 turn1, hand x2 cost): retrieve a live card
///   whose required hearts include >=3 {{heart01}} from the waitroom.
/// - PL!SP-bp4-017-N 桜小路きな子 (ライブ開始時, left side): if this member
///   moved areas this turn -> +2 blades until live end.
/// - PL!-bp4-013-N 園田海未 (ライブ開始時, opt. discard 1): another stage
///   member gains {{heart01}} until live end.
/// - PL!S-bp6-010-N (ライブ開始時): if the required hearts of your live card
///   contain >=4 {{heart02}} in total -> gain {{heart02}} until live end.
/// - PL!S-bp7-023-L (ライブ開始時): if 2+ 『Aqours』 members on stage,
///   optionally return an energy to the energy deck; then if the opponent has
///   exactly 1 more active energy than you -> live scores +1; 2+ more ->
///   instead +2.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(trig))
            .unwrap_or_else(|| panic!("card {} lacks a '{trig}' ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn fire_debut(game: &mut TestGame, cid: i16) {
    fire_trigger(game, cid, AbilityTrigger::Debut, "登場");
}

const LIVE_START: &str = "ライブ開始時";
const FILLER: &str = "PL!-sd1-010-SD";

// ====================================================================
// PL!N-pb1-018-R 近江彼方 — named-member look2 fetch (IDX 413)
// ====================================================================

#[test]
fn pb1018_look_two_reveals_kanata_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    // Deck top: [kanata variant member, filler].
    let kanata = game.new_id("PL!N-bp1-018-N"); // a DIFFERENT 近江彼方 member
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, kanata);

    let me = game.id("PL!N-pb1-018-R");
    game.state.player1.stage.stage[1] = me;
    fire_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.main_deck.cards.contains(&kanata),
        "Kanata left the deck"
    );
    assert!(
        game.state.player1.hand.cards.contains(&kanata),
        "Kanata revealed to hand"
    );
}

#[test]
fn pb1018_no_kanata_in_look_stays_in_waitroom_flow() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    // Deck top: two fillers — no 近江彼方 among them.
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, filler);

    let hand_before = game.state.player1.hand.cards.len();
    let me = game.id("PL!N-pb1-018-R");
    game.state.player1.stage.stage[1] = me;
    fire_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no Kanata in the looked two -> nothing added to hand"
    );
}

// ====================================================================
// PL!SP-sd1-007-SD 米女メイ — optional {E}{E} Liella! member retrieval (IDX 594)
// ====================================================================

fn mei_setup(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!SP-sd1-007-SD");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(10);
    // A Liella! member waiting in the waitroom.
    let kanon = game.new_id("PL!SP-sd1-002-SD"); // 澁谷かのん SD, Liella!
    game.state.player1.waitroom.cards.push(kanon);
    (me, kanon)
}

#[test]
fn sd1007_accept_pay_fetches_liella_member_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, kanon) = mei_setup(&mut game);

    fire_debut(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "optional pay gate must be offered"
    );
    game.select_option(1); // accept: pay 2 energy

    assert!(
        game.state.player1.hand.cards.contains(&kanon),
        "Liella! member retrieved from the waitroom to hand"
    );
}

#[test]
fn sd1007_decline_pay_no_fetch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (me, kanon) = mei_setup(&mut game);

    fire_debut(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "optional pay gate must be offered"
    );
    game.select_option(0); // decline

    assert!(
        !game.state.player1.hand.cards.contains(&kanon),
        "declined -> Kanata stays in the waitroom"
    );
}

// ====================================================================
// PL!SP-sd2-006-SD2 桜小路きな子 — 起動 cost-gated Liella! live retrieval (IDX 845)
// ====================================================================

#[test]
fn sd2006_activation_cost_retrieves_liella_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-sd2-006-SD2");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    // Hand holds the discard-cost target; waitroom holds the Liella live.
    let cost_target = game.new_id(FILLER);
    game.add_to_hand(cost_target);
    let live = game.new_id("PL!SP-bp1-026-L"); // 未来予報ハレルヤ！ Liella live
    game.state.player1.waitroom.cards.push(live);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Liella! live card retrieved from the waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "live left the waitroom"
    );
}

// ====================================================================
// PL!-PR-004-PR 園田海未 — heart-requirement-filtered live retrieval (IDX 554)
// ====================================================================

fn umi_pr_setup(game: &mut TestGame, lives: &[&str]) -> (i16, Vec<i16>) {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!-PR-004-PR");
    game.state.player1.stage.stage[1] = me;
    // Two hand cards as the discard cost.
    game.add_to_hand(game.new_id(FILLER));
    game.add_to_hand(game.new_id(FILLER));
    let mut live_ids = Vec::new();
    for l in lives {
        let cid = game.new_id(l);
        live_ids.push(cid);
        game.state.player1.waitroom.cards.push(cid);
    }
    (me, live_ids)
}

#[test]
fn pr0004_heart01_three_plus_live_fetched() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // PL!N-sd1-028-SD needs heart01 x4 (>= 3) -> matches the filter.
    let (me, live_ids) = umi_pr_setup(&mut game, &["PL!N-sd1-028-SD"]);
    let live = live_ids[0];

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "live with heart01>=3 requirement retrieved"
    );
}

#[test]
fn pr0004_low_heart01_live_not_fetched() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // PL!HS-bp2-020-L needs heart01 x2 (< 3) -> filtered out.
    let (me, live_ids) = umi_pr_setup(&mut game, &["PL!HS-bp2-020-L"]);
    let live = live_ids[0];

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "live with heart01<3 must NOT be retrievable"
    );
}

// ====================================================================
// PL!SP-bp4-017-N 桜小路きな子 — leftside moved-this-turn blades (IDX 684)
// ====================================================================

#[test]
fn bp4017_leftside_moved_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-017-N");
    game.state.player1.stage.stage[0] = me; // LEFT side
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "moved-this-turn on the left side -> +2 blades until live end"
    );
}

#[test]
fn bp4017_leftside_unmoved_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-017-N");
    game.state.player1.stage.stage[0] = me; // LEFT side, never moved

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "did not move this turn -> no blades"
    );
}

#[test]
fn bp4017_center_moved_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-017-N");
    game.state.player1.stage.stage[1] = me; // CENTER — parenthetical restricts to left side
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "ability only activates in the left-side area"
    );
}

// ====================================================================
// PL!-bp4-013-N 園田海未 — optional discard grants other member heart01 (IDX 660)
// ====================================================================

#[test]
fn bp4013_accept_grants_other_stage_member_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp4-013-N");
    let mate = game.new_id(FILLER);
    game.state.player1.stage.stage[1] = me;
    game.state.player1.stage.stage[0] = mate;
    // Discard-cost target in hand.
    game.add_to_hand(game.new_id(FILLER));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    assert!(
        game.has_pending_choice(),
        "optional discard gate must be offered"
    );
    game.select_option(0); // accept (this gate lists pay first)
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // pick the first eligible member (mate)
    }
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(mate, HeartColor::Heart01),
        1,
        "the OTHER stage member gains heart01 until live end"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        0,
        "exclude_self: the ability holder gains nothing"
    );
}

#[test]
fn bp4013_decline_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp4-013-N");
    let mate = game.new_id(FILLER);
    game.state.player1.stage.stage[1] = me;
    game.state.player1.stage.stage[0] = mate;
    game.add_to_hand(game.new_id(FILLER));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.select_option(1); // decline (this gate lists pay first)
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(mate, HeartColor::Heart01),
        0,
        "declined -> no heart gain"
    );
}

// ====================================================================
// PL!S-bp6-010-N — live-card heart02 aggregate gate (IDX 801)
// ====================================================================

#[test]
fn bp6010_heart02_total_four_or_more_gains_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-bp6-010-N");
    game.state.player1.stage.stage[1] = me;
    // Live card requiring heart02 x4 -> aggregate total 4 >= 4.
    let live = game.id("PL!SP-pb1-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    // The during_live temporal gate requires an active live phase.
    game.state.current_phase = rabuka_engine::game_state::Phase::FirstAttackerPerformance;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart02),
        1,
        "required-heart02 total 4 >= 4 -> gain one heart02 until live end"
    );
}

#[test]
fn bp6010_under_threshold_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-bp6-010-N");
    game.state.player1.stage.stage[1] = me;
    // Live card with NO heart02 requirement (heart01/04/05 only).
    let live = game.id("PL!HS-bp2-020-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    game.state.current_phase = rabuka_engine::game_state::Phase::FirstAttackerPerformance;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart02),
        0,
        "aggregate heart02 total 0 < 4 -> no gain"
    );
}

// ====================================================================
// PL!S-bp7-023-L — energy-comparison conditional_alternative (IDX 885)
// ====================================================================

fn bp7023_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);

    let live = game.id("PL!S-bp7-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);

    // Two Aqours members on own stage satisfy the gate.
    let chika_a = game.new_id("PL!S-sd1-001-SD"); // 高海千歌 (CYaRon, Aqours)
    let chika_b = game.new_id("PL!S-sd1-003-SD"); // 松浦果南 (AZALEA, Aqours)
    game.state.player1.stage.stage[0] = chika_a;
    game.state.player1.stage.stage[1] = chika_b;

    // One active energy of our own (will be returned to the deck).
    let e = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e);
    game.state.player1.energy_zone.add_active(1);
    live
}

fn give_p2_active_energy(game: &mut TestGame, n: usize) {
    for _ in 0..n {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(n as u8);
}

#[test]
fn bp7023_opponent_one_energy_ahead_scores_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp7023_setup(&mut game);
    give_p2_active_energy(&mut game, 1); // after we return ours: 0 vs 1

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1); // accept every optional gate (move gate is [No, Yes])
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "opponent exactly 1 energy ahead after the return -> live +1"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "our energy was returned to the energy deck"
    );
}

#[test]
fn bp7023_opponent_two_energy_ahead_scores_plus_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp7023_setup(&mut game);
    give_p2_active_energy(&mut game, 2); // after we return ours: 0 vs 2

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1); // accept every optional gate (move gate is [No, Yes])
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "opponent 2+ energies ahead -> alternative +2 instead"
    );
}

#[test]
fn bp7023_tied_after_return_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = bp7023_setup(&mut game);
    give_p2_active_energy(&mut game, 0); // after we return ours: 0 vs 0

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1); // accept the move; no advantage either way
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "tied energy counts -> no score bonus"
    );
}
