/// Untested-abilities batch 53 — look-and-select family & cost-gated looks.
///
/// Group/character-filtered top-deck looks (positive fetch + decline/no-match):
/// - PL!-pb1-016-R (登場, opt discard): look 4 -> 『lilywhite』 to hand.
/// - PL!SP-pb1-017-N (登場, opt discard): look 5 -> 『5yncri5e!』 to hand.
/// - PL!HS-pb1-018-N (登場, opt discard): look 5 -> 『DOLLCHESTRA』 to hand.
/// - PL!N-pb1-021-R (登場): look 2 -> 「天王寺璃奈」 member to hand.
/// - PL!N-pb1-024-R (登場): look 2 -> 「鐘嵐珠」 member to hand.
/// - PL!SP-bp1-010-R (起動 turn1, {E}{E}+hand): look 5 -> 『Liella!』 to hand.
/// - PL!SP-bp2-005-R (登場, opt {E}{E}): look 7 -> 『Liella!』 to hand.
/// - PL!SP-pb2-007-R (ライブ成功時, opt {E}{E}{E}): 『Liella!』 live from waitroom.
/// - PL!HS-pb1-004-R (登場, {E}+opt hand): mill 3 -> recover 『スリーズブーケ』 live.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

fn fire_debut(game: &mut TestGame, cid: i16) {
    fire_trigger(game, cid, AbilityTrigger::Debut, "登場");
}

const FILLER: &str = "PL!N-sd1-010-SD";

/// Shared idiom: holder on center, a group card on deck top under one filler,
/// an optional hand-discard gate, then answer everything.
fn look_fetch_flow(game: &mut TestGame, holder_no: &str, seed_no: &str) -> (i16, i16) {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id(holder_no);
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER)); // discard-cost target
    let seed = game.new_id(seed_no);
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, seed);
    fire_debut(game, me);
    assert!(
        game.has_pending_choice(),
        "{holder_no}: optional discard gate must be offered"
    );
    game.select_option(0); // hand-cost gates list pay first
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // take the matching card if offered
    }
    (me, seed)
}

// ====================================================================
// IDX 355 — PL!-pb1-016-R lilywhite look4
// ====================================================================

#[test]
fn pb1016_m_looks_four_fetches_lilywhite() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, seed) = look_fetch_flow(&mut game, "PL!-pb1-016-R", "PL!-PR-007-PR"); // 東條希 lilywhite
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "lilywhite card revealed to hand"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&seed),
        "fetched card left the deck"
    );
}

#[test]
fn pb1016_m_decline_leaves_deck_intact() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!-pb1-016-R");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let seed = game.new_id("PL!-PR-007-PR");
    game.state.player1.main_deck.cards.insert(0, seed);
    let deck_before = game.state.player1.main_deck.cards.len();

    fire_debut(&mut game, me);
    game.select_option(1); // decline

    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before);
    assert!(!game.state.player1.hand.cards.contains(&seed));
}

// ====================================================================
// IDX 599 — PL!SP-pb1-017-N 5yncri5e! look5
// ====================================================================

#[test]
fn spb1017_looks_five_fetches_5yncri5e() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, seed) = look_fetch_flow(&mut game, "PL!SP-pb1-017-N", "PL!SP-PR-005-PR"); // 嵐千砂都
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "『5yncri5e!』 card revealed to hand"
    );
}

// ====================================================================
// IDX 776 — PL!HS-pb1-018-N DOLLCHESTRA look5
// ====================================================================

#[test]
fn hspb1018_looks_five_fetches_dollchestra() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, seed) = look_fetch_flow(&mut game, "PL!HS-pb1-018-N", "PL!HS-bp2-008-R");
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "『DOLLCHESTRA』 card revealed to hand"
    );
}

// ====================================================================
// IDX 416 / 419 — named-member look2 (no cost)
// ====================================================================

fn named_look_flow(game: &mut TestGame, holder_no: &str, seed_no: Option<&str>) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id(holder_no);
    game.state.player1.stage.stage[1] = me;
    let seed = game.new_id(seed_no.unwrap_or(holder_no)); // same name matches its own filter
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, seed);
    fire_debut(game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    seed
}

#[test]
fn pb1021_looks_two_reveals_rinana_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // Deck top is another copy of the holder herself — same character name.
    let seed = named_look_flow(&mut game, "PL!N-pb1-021-R", None);
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "「天王寺璃奈」 member revealed to hand"
    );
}

#[test]
fn pb1021_wrong_character_stays_out_of_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let seed = named_look_flow(&mut game, "PL!N-pb1-021-R", Some("PL!N-bp7-012-R"));
    assert!(
        !game.state.player1.hand.cards.contains(&seed),
        "non-璃奈 member on top must NOT be revealed to hand"
    );
}

#[test]
fn pb1024_looks_two_reveals_tomari_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let seed = named_look_flow(&mut game, "PL!N-pb1-024-R", None);
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "「鐘嵐珠」 member revealed to hand"
    );
}

// ====================================================================
// IDX 261 — PL!SP-bp1-010-R 起動 {E}{E}+hand -> Liella! look5
// ====================================================================

#[test]
fn bpsp1010_activation_cost_then_liella_look_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp1-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let kanon = game.new_id("PL!SP-sd1-002-SD"); // Liella!
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, kanon);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // pay hand cost, then take the Liella card
    }

    assert!(
        game.state.player1.hand.cards.contains(&kanon),
        "『Liella!』 card revealed to hand after paying 2E + discard"
    );
}

// ====================================================================
// IDX 290 — PL!SP-bp2-005-R 登場 opt{E}{E} -> Liella! look7
// ====================================================================

#[test]
fn bpsp2005_accept_pay_looks_seven_fetches_liella() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp2-005-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    let kanon = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, kanon);

    fire_debut(&mut game, me);
    assert!(game.has_pending_choice(), "optional {{E}}{{E}} gate offered");
    game.select_option(1); // energy gates are [No, Yes] -> Yes
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&kanon),
        "paid -> 『Liella!』 card revealed to hand"
    );
}

#[test]
fn bpsp2005_decline_pay_skips_the_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp2-005-R");
    game.state.player1.stage.stage[1] = me;
    let kanon = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.main_deck.cards.insert(0, kanon);
    let deck_before = game.state.player1.main_deck.cards.len();

    fire_debut(&mut game, me);
    // No energy given -> an unpayable optional gate is auto-skipped (Q92).
    if game.has_pending_choice() {
        game.select_option(0); // No
    }

    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before);
    assert!(!game.state.player1.hand.cards.contains(&kanon));
}

// ====================================================================
// IDX 487 — PL!SP-pb2-007-R ライブ成功時 opt{E}{E}{E} -> Liella live
// ====================================================================

#[test]
fn pbs2007_pay_three_retrieves_liella_live_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-pb2-007-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    let live = game.new_id("PL!SP-bp1-026-L"); // 未来予報ハレルヤ！
    game.state.player1.waitroom.cards.push(live);

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    game.select_option(1); // Yes
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "paid 3 -> 『Liella!』 live retrieved to hand"
    );
}

#[test]
fn pbs2007_decline_keeps_waitroom_untouched() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-pb2-007-R");
    game.state.player1.stage.stage[1] = me;
    let live = game.new_id("PL!SP-bp1-026-L");
    game.state.player1.waitroom.cards.push(live);

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // No energy given -> an unpayable optional gate is auto-skipped (Q92).
    if game.has_pending_choice() {
        game.select_option(0); // No
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "declined -> live stays in the waitroom"
    );
}

// ====================================================================
// IDX 438 — PL!HS-pb1-004-R 登場 {E}+opt hand -> mill 3 -> スリーズブーケ live
// ====================================================================

#[test]
fn hspb1004_discard_mills_three_recovers_slieszbuque_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-pb1-004-R");
    game.add_to_hand(me);
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER)); // optional hand-discard cost
    let sblive = game.new_id("PL!HS-PR-010-PR"); // Reflection in the mirror
    game.state.player1.waitroom.cards.push(sblive);
    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]); // accept hand cost, then pick the live
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "top 3 cards milled to the waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&sblive),
        "『スリーズブーケ』 live recovered to hand"
    );
}

#[test]
fn hspb1004_decline_hand_cost_no_mill_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-pb1-004-R");
    game.add_to_hand(me);
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let sblive = game.new_id("PL!HS-PR-010-PR");
    game.state.player1.waitroom.cards.push(sblive);
    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::Center);
    // Decline the optional hand discard (first choice).
    if game.has_pending_choice() {
        game.select_option(1);
    }
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined -> no mill"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&sblive),
        "declined -> live stays in the waitroom"
    );
}
