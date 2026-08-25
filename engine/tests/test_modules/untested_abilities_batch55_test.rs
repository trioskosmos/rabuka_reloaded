/// Untested-abilities batch 55 — final depth=none burn-down.
///
/// - PL!HS-bp2-003-R (ライブ開始時, opt discard): look 3 -> any number any
///   order back on top, rest to waitroom.
/// - PL!-PR-017-PR (起動, self-exile): μ's live from waitroom; if own success
///   zone score >= 9 -> activate 2 energies.
/// - PL!S-pb1-013-N / PL!S-pb1-014-N (登場, opt discard): look 4 -> member with
///   heart04/heart02 x2 OR a live REQUIRING those hearts.
/// - PL!-bp5-014-N (登場, opt discard): look 4 -> member with heart05 OR heart06.
/// - PL!N-bp5-028-L (ライブ開始時): stage member with heart02 >=4 -> live +2
///   score AND required hearts become heart02 x5.
/// - PL!SP-bp5-021-N (起動, self-exile): own energy >=6 -> energy_deck -> zone WAIT.
/// - PL!SP-bp5-023-L (ライブ成功時): either success zone >=2 AND a score-icon
///   live revealed -> this live scores +2.
/// - PL!SP-bp5-024-L (ライブ開始時): choose heart01/02/06 -> every own stage
///   member that changed areas this turn gains the chosen heart until live end.
/// - PL!HS-bp5-022-L (ライブ開始時, opt {E}{E}): cost>=9 「EdelNote」 on stage ->
///   choice: debut cost<=4 EdelNote to empty area OR required heart06 -1.
/// - PL!-bp5-024-L (ライブ開始時): 「A-RISE」 on stage -> activate a waited
///   member (+blade until live end) OR wait an opponent original-blade<=3 member.
/// - PL!HS-pb1-020-N (登場): waitroom lives >=3 -> opt discard 2 -> recover
///   「スリーズブーケ」 member AND 「蓮ノ空」 live.
/// - PL!HS-pb1-026-L (ライブ開始時): >=6 DISTINCT 「蓮ノ空」 names across stage +
///   waitroom -> this live's required heart0 -2.
/// - PL!HS-bp6-028-L (ライブ成功時): surplus heart >=1 this turn -> look 2 reorder.
/// - PL!SP-bp7-026-L (ライブ開始時, opt energy->deck cost): with 「葉月恋」 on
///   stage -> draw 2, discard 1.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

fn answer_all(game: &mut TestGame, idx: usize) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectHeartColor") {
            game.select_choice_option(idx);
        } else {
            game.select_indices(&[idx]);
        }
    }
}

// ====================================================================
// IDX 296  Ereorder look 3
// ====================================================================

#[test]
fn hs2003_skip_reorder_rest_goes_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-bp2-003-R");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER)); // optional discard target
    let d1 = game.new_id("PL!N-sd1-001-SD");
    let d2 = game.new_id("PL!N-sd1-002-SD");
    let d3 = game.new_id("PL!N-sd1-003-SD");
    for c in [d1, d2, d3] {
        game.state.player1.main_deck.cards.insert(0, c);
    }

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(0); // accept discard
    // Skip every reorder pick -> all three looked cards go to the waitroom.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        game.select_indices(&[]);
    }

    let wr = &game.state.player1.waitroom.cards;
    assert!(
        wr.contains(&d1) && wr.contains(&d2) && wr.contains(&d3),
        "skipping placement sends the looked cards to the waitroom"
    );
}

// ====================================================================
// IDX 556  Eself-exile -> μ's live + score>=9 activates 2 energies
// ====================================================================

#[test]
fn pr0017_exile_recovers_mus_live_and_activates_two_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-PR-017-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(2);
    let mus_live = game.new_id("PL!-sd1-019-SD"); // μ's live
    game.state.player1.waitroom.cards.push(mus_live);
    // Success zone: one score-9 card -> total 9 >= 9.
    let s9 = game.new_id("PL!S-pb1-023-L");
    game.state.player1.success_live_card_zone.cards.push(s9);

    game.activate_ability(me);
    answer_all(&mut game, 0);

    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "μ's live recovered to hand"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4,
        "score total 9 >= 9 -> 2 energies ACTIVATED (2->4)"
    );
}

#[test]
fn pr0017_under_nine_score_recovers_without_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-PR-017-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(2);
    let mus_live = game.new_id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(mus_live);

    game.activate_ability(me);
    answer_all(&mut game, 0);

    assert!(game.state.player1.hand.cards.contains(&mus_live));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "no success cards -> no activation"
    );
}

// ====================================================================
// IDX 621 / 622  Edual heart-property OR requirement filters
// ====================================================================

#[test]
fn s1013_look4_takes_live_requiring_two_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-pb1-013-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    // HS-bp2-020-L requires heart04 x2 -> satisfies the 必要ハート branch.
    let seed = game.new_id("PL!HS-bp2-020-L");
    game.state.player1.main_deck.cards.insert(0, seed);

    fire_debut_accept(&mut game, me);

    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "live requiring heart04x2 matches the OR filter"
    );
}

#[test]
fn s1013_look4_plain_member_stays_out() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-pb1-013-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let plain = game.new_id(FILLER); // no heart04, no heart04 requirement
    game.state.player1.main_deck.cards.insert(0, plain);

    fire_debut_accept(&mut game, me);

    assert!(
        !game.state.player1.hand.cards.contains(&plain),
        "member without heart04 tie must NOT be taken"
    );
}

#[test]
fn s1014_look4_takes_live_requiring_two_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-pb1-014-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let seed = game.new_id("PL!SP-pb1-023-L"); // requires heart02 x4
    game.state.player1.main_deck.cards.insert(0, seed);

    fire_debut_accept(&mut game, me);

    assert!(game.state.player1.hand.cards.contains(&seed));
}

fn fire_debut_accept(game: &mut TestGame, me: i16) {
    fire_trigger(game, me, AbilityTrigger::Debut, "登場");
    if game.has_pending_choice() {
        game.select_option(0); // hand-cost gates list pay first
    }
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
}

// ====================================================================
// IDX 698  Eheart05 OR heart06 member look4
// ====================================================================

#[test]
fn b5014_takes_member_with_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp5-014-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let eli = game.new_id("PL!-PR-002-PR"); // 絵釁Eheart06 x2
    game.state.player1.main_deck.cards.insert(0, eli);

    fire_debut_accept(&mut game, me);

    assert!(
        game.state.player1.hand.cards.contains(&eli),
        "member holding heart06 matches the OR-color filter"
    );
}

// ====================================================================
// IDX 717 — stage heart02>=4 -> live +2 score, required hearts heart02 x5
// ====================================================================

#[test]
fn nb5028_sets_score_and_required_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!N-bp5-028-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    game.state.current_phase = rabuka_engine::game_state::Phase::FirstAttackerPerformance;
    // Stage member holding heart02 x4.
    let chika = game.new_id("PL!S-bp5-001-R＋");
    game.state.player1.stage.stage[0] = chika;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "heart02 x4 member on stage -> live +2"
    );
    let nh02 = game
        .state
        .mods
        .need_heart_modifiers
        .get(&live)
        .and_then(|m| m.get(&HeartColor::Heart02))
        .map(|e| e.total())
        .unwrap_or(0);
    assert!(
        nh02 >= 5,
        "required hearts must include heart02 totalling >=5 (got {nh02})"
    );
}

// ====================================================================
// IDX 728  Eself-exile; energy >=6 -> wait energy from deck
// ====================================================================

#[test]
fn spb5021_six_energy_exile_places_wait_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp5-021-N");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    fill_energy_deck(&mut game, 0, 2);
    let zone_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(me);
    answer_all(&mut game, 0);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "energy placed from the deck into the zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        6,
        "placed energy is WAITED (active count unchanged by placement)"
    );
}

#[test]
fn spb5021_five_energy_exiles_but_places_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp5-021-N");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);
    let zone_before = game.state.player1.energy_zone.cards.len();

    // Cost is unconditional: activation still exiles the member...
    game.activate_ability(me);
    answer_all(&mut game, 0);
    assert!(
        game.state.player1.waitroom.cards.contains(&me),
        "self-exile cost is paid even below the >=6 energy gate"
    );
    // ...but the gated effect places no energy.
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "energy <6 -> no energy placed from the deck"
    );
}

// ====================================================================
// IDX 729 — compound gate -> live +2
// ====================================================================

#[test]
fn spb5023_compound_gate_grants_plus_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp5-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    // Either-zone >=2: two cards in OWN success zone.
    for no in ["PL!-sd1-019-SD", "PL!HS-bp2-020-L"] {
        let s = game.new_id(no);
        game.state.player1.success_live_card_zone.cards.push(s);
    }
    // Revealed pool holds a live WITH a score icon.
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(game.new_id("PL!SP-bp1-023-L"));

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "both gates met -> live +2"
    );
}

#[test]
fn spb5023_single_success_card_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp5-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    let s = game.new_id("PL!-sd1-019-SD");
    game.state.player1.success_live_card_zone.cards.push(s); // only 1
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(game.new_id("PL!SP-bp1-023-L"));

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(game.state.mods.get_score_modifier(live), 0);
}

// ====================================================================
// IDX 730 — choose color; gain 1 PER success-zone card
// ====================================================================

#[test]
fn spb5024_color_choice_gains_area_moved_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp5-024-L");
    game.add_to_hand(live);
    game.set_live_card(live);

    // Two members on stage; only one has changed areas this turn.
    let mover = game.new_id("PL!S-bp5-001-R＋");
    let stationary = game.new_id("PL!HS-bp5-001-R＋");
    game.state.player1.stage.stage[0] = mover;
    game.state.player1.stage.stage[2] = stationary;
    game.state.push_movement_event(mover, "stage", "stage", None, "p1", true);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    answer_all(&mut game, 0); // choose first offered color
    game.state.recalculate_constants();

    let hearts = |id: i16| {
        game.state.mods.get_heart_modifier(id, HeartColor::Heart01)
            + game.state.mods.get_heart_modifier(id, HeartColor::Heart02)
            + game.state.mods.get_heart_modifier(id, HeartColor::Heart06)
    };
    assert_eq!(
        hearts(mover),
        1,
        "area-moved member gains the chosen heart until live end"
    );
    assert_eq!(
        hearts(stationary),
        0,
        "member that did not move areas gains nothing"
    );
}
