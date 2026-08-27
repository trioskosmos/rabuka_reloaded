/// Untested-abilities batch 37 — optional-cost blade grants & self-wait energy.
///
/// - PL!-bp3-006-R 西木野真姫 (ライブ開始時, opt. discard 1): until live end,
///   +2 blades PER card in own success live zone.
/// - PL!-pb1-010-R 高坂穂乃果 (ライブ開始時, opt. discard 1): until live end,
///   every OTHER staged member gains 1 blade.
/// - PL!SP-bp4-010-R ウィーン・マルガレーテ (起動 ターン1回, E): waits itself,
///   places 1 WAITED energy card from the energy deck.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, live: i16) {
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(live).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!-bp3-006-R 西木野真姫 — success-zone per-card blades behind opt. cost
// ====================================================================

#[test]
fn maki_accept_cost_three_success_cards_six_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let maki = game.id("PL!-bp3-006-R");
    game.state.player1.live_card_zone.cards.push(maki);
    // Three success-zone cards -> 2 blades each.
    for _ in 0..3 {
        let s = game.id("PL!-sd1-019-SD"); // live card
        game.state.player1.success_live_card_zone.cards.push(s);
    }

    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, maki);
    assert!(game.has_pending_choice(), "optional cost prompted");
    game.select_indices(&[0]); // accept: discard 1 hand card

    assert_eq!(
        game.state.mods.get_blade_modifier(maki),
        6,
        "3 success-zone cards x 2 blades"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hand_fodder),
        "cost fodder was discarded"
    );
}

#[test]
fn maki_decline_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let maki = game.id("PL!-bp3-006-R");
    game.state.player1.live_card_zone.cards.push(maki);
    for _ in 0..3 {
        let s = game.id("PL!-sd1-019-SD");
        game.state.player1.success_live_card_zone.cards.push(s);
    }

    fire_live_start(&mut game, maki);
    // No hand fodder exists -> the optional discard cost is unpayable and
    // auto-skips (Q92): no prompt at all.
    assert!(
        !game.has_pending_choice(),
        "unpayable optional cost (empty hand) must auto-skip without prompting"
    );

    assert_eq!(
        game.state.mods.get_blade_modifier(maki),
        0,
        "declined -> no blades despite full success zone"
    );
}

#[test]
fn maki_empty_success_zone_zero_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let maki = game.id("PL!-bp3-006-R");
    game.state.player1.live_card_zone.cards.push(maki);
    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, maki);
    assert!(
        game.has_pending_choice(),
        "optional discard cost must be prompted when a hand card exists"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard skippable discard-cost prompt"
    );
    game.select_indices(&[0]); // accept anyway

    assert_eq!(
        game.state.mods.get_blade_modifier(maki),
        0,
        "no success cards -> 0 x 2 blades"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_fodder),
        "cost was still paid even though it yielded nothing"
    );
}

// ====================================================================
// PL!-pb1-010-R 高坂穂乃果 — other members gain 1 blade behind opt. cost
// ====================================================================

#[test]
fn honoka_accept_other_members_gain_one_blade_each() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let honoka = game.id("PL!-pb1-010-R");
    game.state.player1.live_card_zone.cards.push(honoka);

    let mate_a = game.id("PL!S-sd1-001-SD");
    let mate_b = game.id("PL!-sd1-007-SD"); // 東條希, μ's member
    game.state.player1.stage.stage[0] = mate_a;
    game.state.player1.stage.stage[1] = mate_b;

    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, honoka);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]); // accept

    assert_eq!(game.state.mods.get_blade_modifier(mate_a), 1);
    assert_eq!(game.state.mods.get_blade_modifier(mate_b), 1);
    assert_eq!(
        game.state.mods.get_blade_modifier(honoka),
        0,
        "ほかのメンバー excludes Honoka herself"
    );
}

#[test]
fn honoka_decline_no_blades_anywhere() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let honoka = game.id("PL!-pb1-010-R");
    game.state.player1.live_card_zone.cards.push(honoka);
    let mate_a = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = mate_a;

    fire_live_start(&mut game, honoka);
    // No hand fodder exists -> the optional discard cost auto-skips (Q92).
    assert!(
        !game.has_pending_choice(),
        "unpayable optional cost (empty hand) must auto-skip without prompting"
    );

    assert_eq!(game.state.mods.get_blade_modifier(mate_a), 0);
}

#[test]
fn honoka_lone_member_nothing_to_boost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let honoka = game.id("PL!-pb1-010-R");
    game.state.player1.live_card_zone.cards.push(honoka);
    // No other members on stage.

    let hand_fodder = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_fodder);

    fire_live_start(&mut game, honoka);
    // The optional cost is still offered; accepting discards the fodder but
    // yields nothing (no other members exist).
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_blade_modifier(honoka),
        0,
        "no other members -> no blades, and Honoka never boosts herself"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_fodder),
        "cost fodder was discarded"
    );
}

// ====================================================================
// PL!SP-bp4-010-R ウィーン・マルガレーテ — self-wait activation placing
// one WAITED energy from the energy deck
// ====================================================================

#[test]
fn margarete_activation_waits_self_and_places_waited_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5); // pays the {E} activation cost
    fill_energy_deck(&mut game, 0, 2);

    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "activation cost waits this member"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "one energy card placed into the energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before - 1,
        "the energy cost consumed one active energy; the placed card is WAITED"
    );
}

#[test]
fn margarete_empty_energy_deck_clean_noop() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    // Energy deck left EMPTY.
    let zone_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "the wait is the cost and applies regardless"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "empty energy deck -> nothing placed"
    );
}
