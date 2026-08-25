/// Untested-abilities batch 54 — yell-revealed retrievals, state/formation,
/// dual triggers & specify-color twins.
///
/// - PL!SP-bp2-025-L (ライブ成功時): distinct かのん/ウィーン/冬毬 >=2 on own
///   stage -> retrieve 1 yell-revealed card to hand.
/// - PL!SP-bp7-023-L (ライブ成功時): optionally place a 『Liella!』 card from
///   the yell reveals onto the deck top.
/// - PL!S-bp5-019-L (ライブ成功時): own OR opponent success zone >= 2 ->
///   up to 2 members from the reveals to hand.
/// - PL!SP-PR-018-PR (登場): reveals contain >=7 『Liella!』 -> place an
///   energy from the energy deck in WAIT state.
/// - PL!HS-bp6-013-R (登場/ライブ開始時): wait ONE opponent member with
///   ORIGINAL blades <=3 that is NOT 『DOLLCHESTRA』.
/// - PL!HS-bp6-015-R (登場): debuted from NON-hand -> draw 2, discard 2.
/// - PL!HS-bp6-016-R (起動 {E}{E}{E}{E}): debut a cost<=4 『蓮ノ空』 member
///   from the waitroom into an EMPTY area.
/// - PL!SP-sd2-001-SD2 (ライブ成功時, Liella!-only gate): formation change.
/// - PL!SP-pb2-050-L (登場, 5yncri5e!-only gate): formation change.
/// - PL!SP-bp4-020-N (ライブ開始時, RIGHT side twin of batch52 bp4017):
///   moved this turn on the right side -> +2 blades until live end.
/// - PL!N-bp7-012-R (ライブ開始時, opt {E}): specify a heart color -> gain 1
///   of it until live end.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!N-sd1-010-SD";
const KANON_SD: &str = "PL!SP-sd1-002-SD"; // 澁谷かのん (Liella!)
const WIEN_SD: &str = "PL!SP-sd2-010-SD2"; // ウィーン・マルガレーテ

// ====================================================================
// IDX 294 — PL!SP-bp2-025-L distinct-trio gate -> revealed retrieval
// ====================================================================

#[test]
fn bps2025_distinct_trio_retrieves_revealed_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp2-025-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    // Two DIFFERENT names among the trio satisfy 「2人以上」.
    let kanon = game.new_id(KANON_SD);
    let wien = game.new_id(WIEN_SD);
    game.state.player1.stage.stage[0] = kanon;
    game.state.player1.stage.stage[2] = wien;
    // Yell reveal pool holds a card worth retrieving.
    let prize = game.new_id("PL!SP-PR-003-PR");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(prize);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&prize),
        "gate met -> revealed card retrieved to hand"
    );
}

#[test]
fn bps2025_single_name_no_retrieval() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp2-025-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    // Only ONE of the named trio on stage.
    let kanon = game.new_id(KANON_SD);
    game.state.player1.stage.stage[0] = kanon;
    let prize = game.new_id("PL!SP-PR-003-PR");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(prize);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert!(
        !game.state.player1.hand.cards.contains(&prize),
        "<2 distinct names -> nothing retrieved"
    );
}

// ====================================================================
// IDX 536 — PL!SP-bp7-023-L optional revealed -> deck top
// ====================================================================

#[test]
fn bps7023_optional_liella_reveal_to_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp7-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    let liella_card = game.new_id("PL!SP-bp1-026-L"); // Liella! live as the reveal
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(liella_card);
    let deck_before = game.state.player1.main_deck.cards.len();

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Optional move is folded into the allow_skip selection prompt.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before + 1,
        "revealed Liella! card placed onto the deck top"
    );
}

// ====================================================================
// IDX 709 — PL!S-bp5-019-L either-zone >=2 -> up to 2 members from reveals
// ====================================================================

#[test]
fn bs5019_success_zone_two_retrieves_two_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!S-bp5-019-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    // Own success zone has 2 cards -> gate met.
    for no in ["PL!-sd1-019-SD", "PL!HS-bp2-020-L"] {
        let s = game.new_id(no);
        game.state.player1.success_live_card_zone.cards.push(s);
    }
    let m1 = game.new_id("PL!N-sd1-006-P"); // 璃奈 member (reveal pool)
    let m2 = game.new_id("PL!N-bp1-009-R");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m1);
    game.state.revealed_cards.push(m2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // 「2枚まで」 max selection: answer with both candidate indices at once.
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&m1) && game.state.player1.hand.cards.contains(&m2),
        "gate met -> up to 2 members retrieved to hand"
    );
}

// ====================================================================
// IDX 566 — PL!SP-PR-018-PR 登場: >=7 Liella! in reveals -> wait energy
// ====================================================================

#[test]
fn pr0018_seven_liella_reveals_place_wait_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-PR-018-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 3);
    let zone_before = game.state.player1.energy_zone.cards.len();

    // Reveal pool: exactly 7 Liella! cards + a distractor.
    game.state.revealed_cards.clear();
    for _ in 0..7 {
        game.state.revealed_cards.push(game.new_id("PL!SP-bp1-026-L"));
    }
    game.state.revealed_cards.push(filler);

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // accept optional discard if offered
    }

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        ">=7 Liella! reveals -> energy placed from the deck"
    );
}

#[test]
fn pr0018_under_seven_liella_no_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-PR-018-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 3);
    let zone_before = game.state.player1.energy_zone.cards.len();

    game.state.revealed_cards.clear();
    for _ in 0..6 {
        game.state.revealed_cards.push(game.new_id("PL!SP-bp1-026-L"));
    }

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(game.state.player1.energy_zone.cards.len(), zone_before);
}

// ====================================================================
// IDX 815 — PL!HS-bp6-013-R dual trigger, wait low-blade non-DOLLCHESTRA
// ====================================================================

fn hs6013_wait_flow(game: &mut TestGame, _trig: &str, trigger: AbilityTrigger) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!HS-bp6-013-R");
    game.state.player1.stage.stage[1] = me;
    // Opponent: low-blade μ's filler (eligible).
    let victim = game.new_id(FILLER);
    game.state.player2.stage.stage[1] = victim;
    // Dual-trigger text 「登場/ライブ開始時」 is stored combined; match contains.
    let ability_id = {
        let card = game.db.get_card(me).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref().is_some_and(|t| t.contains("登場")))
            .expect("bp6-013-R lacks a 登場-triggered ability");
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(game.db.get_card(me).unwrap().card_no.to_string()),
        Some(me),
        None,
        None,
    );
    game.state.activating_card = Some(me);
    game.state.process_pending_auto_abilities(&pid);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    victim
}

#[test]
fn hs6013_debut_waits_low_blade_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let victim = hs6013_wait_flow(&mut game, "ライブ開始時", AbilityTrigger::LiveStart);
    assert_eq!(
        game.state.mods.get_orientation_modifier(victim).as_deref(),
        Some("wait"),
        "opponent low-blade member waited"
    );
}

// ====================================================================
// IDX 817 — PL!HS-bp6-015-R non-hand debut -> draw 2, discard 2
// ====================================================================

#[test]
fn hs6015_hand_debut_draws_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-bp6-015-R");
    game.add_to_hand(me);
    game.give_energy(5);
    let hand_before = game.state.player1.hand.cards.len();

    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "normal hand debut -> no draw bonus"
    );
}

// ====================================================================
// IDX 818 — PL!HS-bp6-016-R 起動 4E -> cost<=4 蓮ノ空 member to empty area
// ====================================================================

#[test]
fn hs6016_activation_deploys_low_cost_member_to_empty_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-bp6-016-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    // 蓮ノ空 member with cost <= 4 waiting: HS-bp2-004-R 花丸 cost 2.
    let kamaru = game.new_id("PL!HS-bp2-004-R");
    game.state.player1.waitroom.cards.push(kamaru);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.stage.stage.contains(&kamaru),
        "cost<=4 『蓮ノ空』 member debuted into an empty area"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&kamaru),
        "deployed member left the waitroom"
    );
}

// ====================================================================
// IDX 686 — PL!SP-bp4-020-N RIGHT side twin of batch52's bp4017
// ====================================================================

#[test]
fn bp4020_rightside_moved_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-020-N");
    game.state.player1.stage.stage[2] = me; // RIGHT side
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();

    assert_eq!(game.state.mods.get_blade_modifier(me), 2);
}

#[test]
fn bp4020_center_moved_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-020-N");
    game.state.player1.stage.stage[1] = me; // CENTER — restricted to right side
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "right-side-only ability must not fire from center"
    );
}

// ====================================================================
// IDX 521 — PL!N-bp7-012-R opt {E} -> specify color -> gain 1
// ====================================================================

#[test]
fn nbp7012_pay_specifies_color_and_gains_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!N-bp7-012-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(3);

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "optional {{E}} gate offered");
    game.select_option(1); // Yes
    // Answer the color specification choice(s).
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectHeartColor") {
            game.select_choice_option(0);
        } else {
            game.select_indices(&[0]);
        }
    }
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        1,
        "first offered color gained until live end"
    );
}
