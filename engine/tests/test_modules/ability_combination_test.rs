//! Ability-COMBINATION tests: both abilities on ONE card instance interacting.
//!
//! Coverage today proves each ability works alone; these tests prove siblings
//! compose — one ability's resolution creating its sibling's trigger event,
//! shared use_limits, stale-modifier isolation.
//!
//! Chains exercised:
//! - PL!S-bp5-111-R      起動 position-change → 自動 area-move waits low-blade opponent
//! - PL!HS-pb1-003-R     登場 hand-discard → 自動 per-discard heart01+blade
//! - PL!SP-bp7-005-R＋   自動×2: zone→deck placement feeds the other's own-effect gate
//! - PL!N-bp3-005-R＋    ライブ開始時 conditional constant grant coexisting with
//!                       the debut-counting 自動 draw-to-five

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

// ====================================================================
// PL!S-bp5-111-R — 起動 chains into 自動
// ====================================================================

/// Activating the position-change must ALSO arm the area-move auto:
/// opponent's blade<=2 (original) member ends up waited.
#[test]
fn bp5_111_activation_chain_waits_low_blade_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let saint_snow = game.id("PL!S-bp5-111-R");
    let aqours = game.new_id("PL!S-bp2-015-PR");
    let opp_member = game.new_id("PL!-sd1-010-SD"); // original blade 1 (<= 2)

    game.state.player1.stage.stage = [aqours, saint_snow, -1];
    game.state.player2.stage.stage = [-1, opp_member, -1];
    game.give_energy(1); // {E} activation cost

    game.activate_ability(saint_snow);
    if game.has_pending_choice() {
        game.select_generated(0); // destination area choice
    }
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage[0], saint_snow,
        "activation moved the member to the Aqours area"
    );
    assert!(
        game.state.mods.get_orientation_modifier(opp_member) == Some("wait"),
        "chain: area-move auto waits opponent's blade<=2 member"
    );
}

/// Negative chain arm: no opponent member -> nothing to wait, activation itself
/// still resolves cleanly.
#[test]
fn bp5_111_chain_neg_empty_opponent_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let saint_snow = game.id("PL!S-bp5-111-R");
    let aqours = game.new_id("PL!S-bp2-015-PR");

    game.state.player1.stage.stage = [aqours, saint_snow, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.give_energy(1);

    game.activate_ability(saint_snow);
    if game.has_pending_choice() {
        game.select_generated(0);
    }
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage[0], saint_snow,
        "activation still moved the member"
    );
    assert!(
        game.state.player2.stage.stage.iter().all(|&c| c == -1),
        "negative: empty opponent stage stays empty"
    );
}

// ====================================================================
// PL!HS-pb1-003-R — 登場 discards feed the 自動 per-discard gain
// ====================================================================

/// Debut discarding 1 みらくらぱーく！ member: draws discarded+1 AND the auto
/// grants heart01+blade once for that hand->waitroom discard.
#[test]
fn hs_pb1_003_debut_discards_feed_auto_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mirakura = game.new_id("PL!HS-bp1-005-PR"); // unit: みらくらぱーく!
    let filler = game.new_id(FILLER);
    let card = game.new_id("PL!HS-pb1-003-R");

    game.give_energy(15);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(mirakura);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(card, MemberArea::Center);

    // Any-number discard choice: drop the みらくらぱーく！ member, then finalize.
    assert!(
        game.has_pending_choice(),
        "debut discard choice must be offered"
    );
    game.select_indices_sequential(&[0]);

    // 登場 half: started with 3, played 1, discarded 1, drew 2.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "hand: -played -discarded +drawn(1+1) = 3"
    );


    // 自動 half: the discard was >=1 card hand->waitroom this turn.
    assert!(
        game.state.player1.waitroom.cards.contains(&mirakura),
        "みらくらぱーく！ member went to the waitroom"
    );
    assert!(
        game.state.mods.get_blade_modifier(card) >= 1,
        "chain: debut discard arms the per-discard auto -> +1 blade"
    );
    assert!(
        game.state.mods.get_heart_modifier(card, HeartColor::Heart01) >= 1,
        "chain: per-discard auto -> +1 heart01"
    );
}

/// Negative: nothing discarded from hand -> no auto gain (blade stays 0),
/// debut still drew 0+1 = 1 card.
#[test]
fn hs_pb1_003_no_discard_no_auto_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler_a = game.new_id(FILLER);
    let card = game.new_id("PL!HS-pb1-003-R");

    game.give_energy(15);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler_a);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(card, MemberArea::Center);

    // No みらくらぱーく！ members in hand: skip whatever is offered.
    if game.has_pending_choice() {
        game.select_indices_sequential(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "hand: -played +drawn(0+1) = 2 with nothing discarded"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "negative: no hand->waitroom discard, no auto blade"
    );
}

// ====================================================================
// PL!SP-bp7-005-R＋ — 自動×2 cascade
// ====================================================================

/// ab#0 places a waited energy on debut (and on zone->deck moves, ターン1回).
/// That placement is an OWN-effect energy_placed event, which is ab#1's
/// trigger: the pair cascades off the debut itself.
#[test]
fn sp_bp7_005_double_auto_cascade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.new_id("PL!SP-bp7-005-R＋");
    for _ in 0..3 {
        game.state
            .player1
            .energy_deck
            .cards
            .push(game.new_id("LL-E-001-SD"));
    }
    let deck_len_setup = game.state.player1.energy_deck.cards.len();

    // Debut arms ab#0's appearance branch: one waited energy deck->zone.
    // That placement is an OWN-effect energy_placed event, which is exactly
    // ab#1's trigger -> the pair cascades off the debut itself.
    // (ab#0 is ターン1回: the debut consumes its once-per-turn firing.)
    game.give_energy(10); // 9 pay for the member, 1 left active
    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();
    game.add_to_hand(member);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.try_play_to_stage(member, MemberArea::Center).ok();
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_len_setup - 1,
        "ab#0 placed exactly one energy from the deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "the placed energy landed in the energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before - 9,
        "placed energy is WAITED (active count only dropped by the play cost)"
    );
    assert!(
        game.state.mods.get_blade_modifier(member) >= 1,
        "cascade: ab#0's own-effect zone placement arms ab#1 -> +1 blade"
    );
}

// ====================================================================
// PL!N-bp3-005-R＋ — ライブ開始時 conditional constant + debut-counting 自動
// ====================================================================

/// PL!N-bp3-005-R＋ is a MEMBER whose two abilities share one counter:
/// 自動 (own debuts this turn reach 3 -> draw until hand holds five) and
/// ライブ開始時 (2+ own debuts this turn -> 常時 score+1 until live end).
/// Playing it as the THIRD debut arms both in a single action.
#[test]
fn n_bp3_005_constant_grant_and_counting_auto_combo() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler_tpl = FILLER;

    let member = game.new_id("PL!N-bp3-005-R＋"); // cost 15
    let d1 = game.new_id(filler_tpl);
    let d2 = game.new_id(filler_tpl);

    game.give_energy(30);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_hand(d1);
    game.add_to_hand(d2);
    game.add_to_hand(member);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(filler_tpl));
    }
    // Hand = [d1, d2, member] (3 cards).

    // Debuts 1 and 2 arm the shared counter.
    game.play_to_stage(d1, MemberArea::LeftSide);
    scan_autos_both(&mut game);
    game.play_to_stage(d2, MemberArea::RightSide);
    scan_autos_both(&mut game);

    // Third debut IS the member itself: its own counting auto draws to five.
    game.play_to_stage(member, MemberArea::Center);
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "3rd debut: auto drew until hand held 5 cards"
    );

    // Live start with 2+ own debuts this turn: 常時 score+1 until live end.
    fire_trigger(&mut game, member, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "conditional constant granted (3 own debuts >= 2)"
    );
}

/// Boundary pair on ONE card: with only 2 total own debuts the counting auto
/// (threshold 3) stays silent while the LiveStart constant (threshold 2)
/// still grants — the two thresholds diverge exactly as printed.
#[test]
fn n_bp3_005_two_debuts_auto_silent_but_constant_grants() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler_tpl = FILLER;

    let member = game.new_id("PL!N-bp3-005-R＋");
    let d1 = game.new_id(filler_tpl);

    game.give_energy(25);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_hand(d1);
    game.add_to_hand(member);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(filler_tpl));
    }

    // Debut 1, then the member itself as debut 2.
    game.play_to_stage(d1, MemberArea::LeftSide);
    scan_autos_both(&mut game);
    game.play_to_stage(member, MemberArea::Center);
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "2nd debut < 3: counting auto did not draw"
    );

    fire_trigger(&mut game, member, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "LiveStart constant still grants at 2 own debuts"
    );
}
