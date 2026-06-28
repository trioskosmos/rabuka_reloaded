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
use rabuka_engine::zones::MemberArea;

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
    let old_pos = 1usize;
    let new_pos = 0usize;
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

    // Auto should fire with a 3-option choice (blades / wait / draw)
    assert!(
        game.has_pending_choice(),
        "Q263: Auto ability should create a choice on center→area move"
    );
}
