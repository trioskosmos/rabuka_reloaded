/// Untested-abilities batch 47 — position-change activation, all-Liella gate,
/// mill + A-RISE retrieval.
///
/// - PL!SP-bp7-022-N (起動 ターン1回): pay 1 energy zone->energy deck, then
///   this member POSITION CHANGES to another area.
/// - PL!HS-bp6-047 sibling PL!SP-pb2-047-L (ライブ開始時, opt. discard 1):
///   if own staged members are ALL 『Liella!』 -> one enemy member with
///   cost <= 2 becomes WAITED.
/// - PL!-bp5-010-N (ライブ開始時): mill 3 unconditionally, then retrieve an
///   『A-RISE』 member from the waitroom if present.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!SP-bp7-022-N — pay energy -> position change
// ====================================================================

#[test]
fn bp7022_activation_moves_member_to_another_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp7-022-N");
    // Start at CENTER; left side is free for the position change.
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before + 1,
        "one energy returned to the energy deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before - 1,
        "zone lost the paid energy"
    );
    assert_ne!(
        game.state.player1.stage.stage.iter().position(|&c| c == me),
        Some(1),
        "member changed position out of the center"
    );
}

#[test]
fn bp7022_empty_energy_deck_still_changes_position_or_noops_cleanly() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp7-022-N");
    game.state.player1.stage.stage[1] = me;
    // NO energy at all -> the {E} cost cannot be paid; activation no-ops
    // without panicking.
    game.activate_ability(me);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.stage.stage.iter().any(|&c| c == me),
        "member stays on stage"
    );
}

// ====================================================================
// PL!SP-pb2-047-L — all-Liella gate waits an enemy cost<=2 member
// ====================================================================

fn fire_enemy_wait_test(game: &mut TestGame, enemy_card_no: &str) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!SP-pb2-047-L");
    game.state.player1.live_card_zone.cards.push(live);
    // Own stage: two genuine Liella! (series Superstar) members.
    let l1 = game.id("PL!SP-pb2-036-N");
    let l2 = game.id("PL!SP-pb2-037-N");
    game.state.player1.stage.stage[0] = l1;
    game.state.player1.stage.stage[1] = l2;
    let enemy = game.id(enemy_card_no);
    game.state.player2.stage.stage[0] = enemy;
    fire_live_start(game, live);
    enemy
}

#[test]
fn pb2047_all_liella_stage_waits_enemy_cost_two_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // Optional discard-1-hand cost first.
    let fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);
    let enemy = fire_enemy_wait_test(&mut game, "PL!SP-PR-010-PR"); // cost 2

    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard cost"
    );
    game.select_indices(&[0]); // accept the discard

    assert_eq!(
        game.state.mods.get_orientation_modifier(enemy),
        Some("wait"),
        "all-Liella stage -> enemy cost<=2 member waited"
    );
}

#[test]
fn pb2047_non_liella_on_stage_no_enemy_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(_fodder);
    // Replace one own member with a mu's member -> not ALL Liella!.
    let live = game.id("PL!SP-pb2-047-L");
    game.state.player1.live_card_zone.cards.push(live);
    let l1 = game.id("PL!SP-pb2-036-N");
    let outsider = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = l1;
    game.state.player1.stage.stage[1] = outsider;
    let enemy = game.id("PL!SP-PR-010-PR");
    game.state.player2.stage.stage[0] = enemy;

    fire_live_start(&mut game, live);
    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected even when the wait condition fails"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard cost"
    );
    game.select_indices(&[0]);

    assert_ne!(
        game.state.mods.get_orientation_modifier(enemy),
        Some("wait"),
        "non-Liella present -> enemy not waited"
    );
}

// ====================================================================
// PL!-bp5-010-N — unconditional mill 3 + optional A-RISE retrieval
// ====================================================================

#[test]
fn bp5010_mills_three_retrieves_arise_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp5-010-N");
    game.state.player1.live_card_zone.cards.push(me);
    // Deck top: three member cards to mill.
    let m1 = game.id("PL!N-bp3-006-R");
    let m2 = game.id("PL!SP-bp4-022-N");
    let m3 = game.id("PL!S-sd1-001-SD");
    for m in [m3, m2, m1] {
        game.state.player1.main_deck.cards.insert(0, m);
    }
    // A-RISE member already in the waitroom.
    let arise = game.id("PL!-bp5-111-R");
    game.state.player1.waitroom.cards.push(arise);
    // Hand card for the optional discard cost.
    let fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, me);
    // Accept the optional hand-discard cost by choosing which card to
    // discard (allow_skip=true SelectCard), then drain.
    game.select_indices(&[0]);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "milled exactly 3"
    );
    assert!(
        game.state.player1.hand.cards.contains(&arise),
        "A-RISE member retrieved to hand"
    );
}

#[test]
fn bp5010_mill_happens_even_without_arise() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp5-010-N");
    game.state.player1.live_card_zone.cards.push(me);
    // Waitroom has no A-RISE member, but the cost needs a hand card.
    let fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, me);
    game.select_indices(&[0]); // accept the optional discard
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "mill still happened"
    );
}
