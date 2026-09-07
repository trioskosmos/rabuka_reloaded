/// PL!SP-bp2-011-R (鬼塚冬毬) Q118
///
/// {{toujyou.png|登場}}自分の控え室にある、カード名の異なるライブカードを2枚選ぶ。
/// 選択した場合、相手はそのカードのうち1枚を選ぶ。相手に選ばれたカードを
/// 自分の手札に加える。
///
/// Q118: If you can't select 2 different-named live cards (e.g. only 1 in discard),
/// can you still select 1 and add it to hand? A: No — the effect requires 2 distinct
/// names to proceed.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

/// Assert the pending prompt is Toubatsu's jidou 3-option (blades / wait /
/// draw), take the blade bullet, and assert +2 blade lands. This pins the
/// choice identity, not just pendency: the 登場 prompt or any unrelated
/// choice fails the option match.
fn take_blades_option(game: &mut TestGame, toubatsu: i16) {
    match game.get_pending_choice().clone() {
        Choice::SelectTarget { target, description, .. } => {
            assert_eq!(
                target.as_str(),
                "choice",
                "expected the jidou 3-option prompt"
            );
            assert!(
                description.contains("ブレード"),
                "3-option prompt must offer the blade bullet, got {:?}",
                description
            );
        }
        other => panic!("expected jidou 3-option prompt, got {:?}", other),
    }
    game.select_choice_option(0);
    scan_autos_both(game);
    assert_eq!(
        game.state.mods.get_blade_modifier(toubatsu),
        2,
        "blade bullet grants +2 blade until live end"
    );
}

/// Positive: 2 distinct live cards in discard → ability proceeds.
#[test]
fn toubatsu_q118_2_distinct_live_cards_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let toubatsu = game.id("PL!SP-bp2-011-R");
    let live_a = game.id("PL!-sd1-019-SD"); // START:DASH!!
    let live_b = game.id("PL!N-sd1-028-SD"); // Dream with You (different name)
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(toubatsu);
    game.add_to_hand(filler);

    // Discard: 2 live cards with different names
    game.add_to_discard(live_a);
    game.add_to_discard(live_b);

    game.give_energy(15);
    game.play_to_stage(toubatsu, MemberArea::Center);

    // Debut fires: select 2 distinct live cards from discard
    assert!(game.has_pending_choice(), "First select choice expected");
    // First choice should be routed to self
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .as_ref()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "First select choice should be routed to activator (self)"
    );
    // Select both cards at once
    game.try_select_indices(&[0, 1]).unwrap();

    // Opponent chooses 1 of the 2 selected cards
    assert!(game.has_pending_choice(), "Opponent select choice expected");
    {
        let entry = game.state.ability_queue.current_entry();
        assert_eq!(
            entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
            Some("p2"),
            "Opponent-select choice should be routed to opponent"
        );
    }
    game.select_option(0); // opponent selects first card (index in selected_cards)

    // Opponent's chosen card goes to player1's hand
    let in_hand = game.state.player1.hand.cards.contains(&live_a)
        || game.state.player1.hand.cards.contains(&live_b);
    assert!(
        in_hand,
        "One of the 2 distinct live cards should be added to hand by opponent choice"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_a)
            || !game.state.player1.waitroom.cards.contains(&live_b),
        "At least one live card should have moved out of discard"
    );
    // Player2 should NOT have the card in hand
    assert!(
        !game.state.player2.hand.cards.contains(&live_a),
        "Card should not go to opponent's hand"
    );
    assert!(
        !game.state.player2.hand.cards.contains(&live_b),
        "Card should not go to opponent's hand"
    );
}

/// Q118: Only 1 live card in discard → ability fails, nothing added to hand.
#[test]
fn toubatsu_q118_1_live_card_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let toubatsu = game.id("PL!SP-bp2-011-R");
    let live_a = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(toubatsu);
    game.add_to_hand(filler);
    game.give_energy(15);

    // Discard: only 1 live card → can't pick 2 distinct
    game.add_to_discard(live_a);

    game.play_to_stage(toubatsu, MemberArea::Center);

    // Debut fires: select 2 distinct live cards — only 1 available
    // Engine returns early (no choice created) since distinct filter fails
    assert!(
        !game.has_pending_choice(),
        "Q118: No choice should be created — insufficient distinct cards"
    );

    // Q118: Live card should NOT be in hand (effect required 2 distinct)
    assert!(
        !game.state.player1.hand.cards.contains(&live_a),
        "Live card should not be added: effect needs 2 distinct cards"
    );
}

/// Bug repro: discard has duplicate-named live cards — player picks 2 distinct,
/// opponent must select from only those 2, not from the full discard set.
#[test]
fn toubatsu_duplicate_names_in_discard_opponent_only_sees_correct_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let toubatsu = game.id("PL!SP-bp2-011-R");
    let live_a = game.id("PL!-sd1-019-SD"); // START:DASH!!
    let live_b = game.id("PL!N-sd1-028-SD"); // Dream with You (different name)
    let live_a2 = game.id("PL!-sd1-019-SD"); // second copy of live_a (same name)
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(toubatsu);
    game.add_to_hand(filler);

    // Discard: [live_a, live_a2, live_b] — live_a appears twice (same name)
    game.add_to_discard(live_a);
    game.add_to_discard(live_a2);
    game.add_to_discard(live_b);

    game.give_energy(15);
    game.play_to_stage(toubatsu, MemberArea::Center);

    // Player selects 2 distinct-name live cards
    assert!(game.has_pending_choice(), "First select choice expected");
    game.try_select_indices(&[0, 1]).unwrap();

    // Opponent selects 1 of the 2
    assert!(game.has_pending_choice(), "Opponent select choice expected");
    {
        let entry = game.state.ability_queue.current_entry();
        assert_eq!(
            entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
            Some("p2"),
            "Opponent-select choice should be routed to opponent"
        );
    }
    game.select_option(0);

    // The card added to hand must be one of the two originally selected (distinct)
    let got = if game.state.player1.hand.cards.contains(&live_a) {
        live_a
    } else if game.state.player1.hand.cards.contains(&live_b) {
        live_b
    } else {
        panic!(
            "Hand must contain one of the two distinct selected cards, got: {:?}",
            game.state
                .player1
                .hand
                .cards
                .iter()
                .filter(|&&c| c != filler)
                .collect::<Vec<_>>()
        );
    };

    // live_a2 (the duplicate) must never have left discard
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a2),
        "Duplicate live_a2 must remain in discard — opponent should not select it"
    );

    // The first of the two distinct picks must have moved (either to hand or left discard)
    assert!(
        !game.state.player1.waitroom.cards.contains(&got),
        "Selected card should have moved out of discard"
    );
}

/// Q263: Auto ability triggers when member moves from center area to another area.
/// The auto ability offers a choice of 3 options: +2 blades until live end,
/// weigh 1 opponent member with ≤2 blades, or draw 1 card.
#[test]
fn toubatsu_q263_center_to_area_move_triggers_auto() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let toubatsu = game.id("PL!SP-pb2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Place on center (index 1)
    game.state.player1.stage.stage[1] = toubatsu;

    // Simulate area move: center → left (index 0)
    let old_pos = 1u8;
    let new_pos = 0u8;
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[0] = toubatsu;
    game.state
        .position_change_events
        .push(rabuka_engine::types::PositionChangeEvent {
            moved_card_id: toubatsu,
            old_position: old_pos,
            new_position: new_pos,
            cause_card_id: None,
            cause_player_id: "p1".to_string(),
            effect_only: false,
        });
    game.state.record_card_movement(toubatsu);
    game.state
        .push_movement_event(toubatsu, "stage", "stage", None, "p1", false);
    game.state.position_change_occurred_this_turn = true;

    let pid = game.state.player1.id.clone();
    // TAS scan: finds non-self_target position_change abilities
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    // Auto should fire with a 3-option choice (blades / wait / draw).
    assert!(
        game.has_pending_choice(),
        "Q263: Auto ability should create a choice on center→area move"
    );
    take_blades_option(&mut game, toubatsu);
}

/// Q263 via a REAL ability: Shiki swaps Toubatsu center → area (own effect).
/// Same expectation as the synthetic q263 test, driven by an actual 起動.
/// Both cards are stage-assigned (no debuts) so no debut scan can interfere.
#[test]
fn toubatsu_real_swap_move_triggers_auto() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let toubatsu = game.id("PL!SP-pb2-011-R");
    let shiki = game.id("PL!SP-bp2-008-R");
    game.state.player1.stage.stage = [-1, toubatsu, shiki];
    game.give_energy(1); // E for Shiki's kidou
    assert!(
        !game.has_pending_choice(),
        "no trigger before the swap"
    );

    // Shiki Right → Center: Toubatsu (at Center) is swapped to the Right
    // by our own card effect.
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert!(
        game.has_pending_choice(),
        "Shiki swap should offer target areas"
    );
    let actions = game.generated_actions();
    let idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == "center")
        })
        .expect("center target not offered");
    game.select_generated(idx);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.stage.stage,
        [-1, shiki, toubatsu],
        "swap must have moved Toubatsu center→area via our own effect"
    );
    // TAS scan, mirroring the synthetic q263 test above.
    // Pin the identity, not just pendency (see q263 note above).
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(
        game.has_pending_choice(),
        "Q263 via real swap: auto ability should create a choice on center→area move"
    );
    take_blades_option(&mut game, toubatsu);
}

/// Unrelated real ability (Kahori debut placing energy, no movement) with
/// Toubatsu armed in center → silent.
#[test]
fn toubatsu_unrelated_debut_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let toubatsu = game.id("PL!SP-pb2-011-R");
    game.state.player1.stage.stage = [-1, toubatsu, -1];
    game.state
        .player1
        .energy_deck
        .cards
        .push(game.id("LL-E-001-SD"));
    let placer = game.id("PL!SP-pb1-005-R");
    game.state.player1.hand.cards.push(placer);
    game.give_energy(13);
    game.play_to_stage(placer, MemberArea::RightSide);
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage[1], toubatsu,
        "Toubatsu still in center"
    );
    assert!(
        !game.has_pending_choice(),
        "unrelated debut with no movement must not arm the center-move watcher"
    );
}

/// Two-seat probe: P1's swap must arm P1's center watcher, never P2's.
///
/// The in-resolution movement scan derives its seat from the current queue
/// entry; while the resolver holds the entry that can resolve to the wrong
/// seat, and position filters don't check seats. A P2 arming here proves a
/// cross-seat scan.
#[test]
fn toubatsu_swap_arms_own_seat_not_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let toubatsu1 = game.new_id("PL!SP-pb2-011-R");
    let shiki = game.new_id("PL!SP-bp2-008-R");
    let toubatsu2 = game.new_id("PL!SP-pb2-011-R");
    game.state.player1.stage.stage = [-1, toubatsu1, shiki];
    game.state.player2.stage.stage = [-1, toubatsu2, -1];
    game.give_energy(1); // E for Shiki's kidou

    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == "center")
        })
        .expect("center target not offered");
    game.select_generated(idx);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.stage.stage,
        [-1, shiki, toubatsu1],
        "swap must have moved P1 Toubatsu center→area"
    );

    // P1 first: own watcher must fire with its 3-option.
    let p1 = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &p1);
    game.state.process_pending_auto_abilities(&p1);
    assert!(
        game.has_pending_choice(),
        "P1 swap must arm P1's center watcher"
    );
    take_blades_option(&mut game, toubatsu1);

    // P2: nothing of hers moved — must stay silent.
    let p2 = game.state.player2.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &p2);
    game.state.process_pending_auto_abilities(&p2);
    assert!(
        !game.has_pending_choice(),
        "P1's swap must not arm P2's center watcher"
    );
}
#[test]
fn toubatsu_unrelated_debut_no_trigger() {
