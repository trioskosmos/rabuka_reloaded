/// Tests for PL!SP-bp2-011-R (鬼塚冬毬) ab#0 — Q118
///
/// Ability (登場):
///   自分の控え室から、カード名の異なるライブカードを2枚選ぶ。
///   選択した場合、相手はそのカードのうち1枚を選ぶ。
///   相手に選ばれたカードを自分の手札に加える。
///
/// Flow:
///   1. Player selects 2 live cards with DIFFERENT names from discard
///   2. Opponent selects 1 of those 2
///   3. The opponent's chosen card goes to player's hand
///
/// Q118: 1枚しか選べない場合、相手が選んで手札に加えられるか？
/// Answer: いいえ。2枚選べないと効果は不発。
use crate::helpers::*;

/// Two different-named live cards in discard. Full sequence:
/// player picks both → opponent picks first (index 0) → that card is in hand.
#[test]
fn fuyumari_q118_opponent_picks_first_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-020-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    // Order in discard: live_a at index 0, live_b at index 1
    game.state.player1.waitroom.cards.push(live_a);
    game.state.player1.waitroom.cards.push(live_b);

    game.give_energy(11);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    // Step 1: Player selects 2 distinct live cards from discard (indices 0 and 1)
    assert!(game.has_pending_choice(), "Should have select choice");
    // First choice should be routed to self (no choice_player_id override or = p1)
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .as_ref()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "First select choice should be routed to activator (self)"
    );
    game.try_select_indices(&[0, 1]).unwrap();

    // Step 2: Opponent picks a card from the selected ones
    // The opponent select choice has source "selected_cards" with the 2 cards.
    // The choice presents indices 0 and 1 (live_a, live_b).
    // Opponent picks index 0 (live_a).
    assert!(
        game.has_pending_choice(),
        "Should have opponent select choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .as_ref()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Opponent-select choice should be routed to opponent"
    );
    // Verify generated_actions returns non-empty for opponent's selected_cards choice
    let actions = game.generated_actions();
    assert!(
        actions.len() >= 2,
        "Opponent should see at least 2 action options (got {})",
        actions.len()
    );
    game.select_option(0);

    // Step 3: Opponent's chosen card (live_a, index 0 of selected_cards) goes to hand
    assert!(
        game.state.player1.hand.cards.contains(&live_a),
        "Opponent-chosen card (live_a) should be in hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_a),
        "live_a should be removed from discard"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_b),
        "live_b stays in discard (not chosen by opponent)"
    );
    assert!(
        !game.state.player2.hand.cards.contains(&live_a),
        "live_a should go to player1's hand, not opponent"
    );
}

#[test]
fn fuyumari_q118_opponent_picks_second_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-020-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(live_a);
    game.state.player1.waitroom.cards.push(live_b);

    game.give_energy(11);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    // Step 1: Player selects both
    assert!(game.has_pending_choice(), "Should have select choice");
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .as_ref()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "First select choice should be routed to activator (self)"
    );
    game.try_select_indices(&[0, 1]).unwrap();

    // Step 2: Opponent picks the SECOND card (index 1 = live_b)
    assert!(
        game.has_pending_choice(),
        "Should have opponent select choice"
    );
    {
        let entry = game.state.ability_queue.current_entry();
        assert_eq!(
            entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
            Some("p2"),
            "Opponent-select choice should be routed to opponent"
        );
    }
    // Verify generated_actions returns non-empty for opponent
    let actions = game.generated_actions();
    assert!(
        actions.len() >= 2,
        "Opponent should see at least 2 action options (got {})",
        actions.len()
    );
    game.select_option(1);

    // Step 3: Opponent's chosen card (live_b, index 1 of selected_cards) goes to hand
    assert!(
        game.state.player1.hand.cards.contains(&live_b),
        "Opponent-chosen card (live_b) should be in hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_b),
        "live_b should be removed from discard"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a),
        "live_a stays in discard (not chosen by opponent)"
    );
}

/// Only 1 unique live card available → can't select 2 → effect does nothing.
#[test]
fn fuyumari_q118_only_one_live_card_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.waitroom.cards.push(filler);

    game.give_energy(11);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    // Fewer than 2 distinct live cards → select creates no choice → effect ends
    // Consume any remaining choice
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "No live card should be added when <2 available"
    );
}

/// Card count integrity: total cards across all zones unchanged.
#[test]
fn fuyumari_q118_card_count_integrity() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-020-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(live_a);
    game.state.player1.waitroom.cards.push(live_b);

    game.give_energy(11);
    let total_before = total_cards(&game.state.player1);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "second live-card selection prompt expected (1 remaining)"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the remaining live-card pick"
    );
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "opponent selection prompt expected after both lives are picked"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the opponent's pick from selected_cards"
    );
    game.select_indices(&[0]); // opponent selects first of the two
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let total_after = total_cards(&game.state.player1);
    assert_eq!(
        total_after, total_before,
        "Total card count unchanged for player1"
    );
}

/// Three distinct live cards in discard. Player picks 2 → opponent picks 1 → correct card in hand.
#[test]
fn fuyumari_q118_three_distinct_cards_opponent_picks_middle() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-020-SD");
    let live_c = game.id("PL!N-sd1-028-SD"); // third distinct live card

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(live_a);
    game.state.player1.waitroom.cards.push(live_b);
    game.state.player1.waitroom.cards.push(live_c);

    game.give_energy(11);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    // Step 1: Player selects 2 of 3 distinct cards (indices 0 and 2 = live_a and live_c)
    assert!(game.has_pending_choice(), "Should have select choice");
    game.try_select_indices(&[0, 2]).unwrap();

    // Step 2: Opponent picks index 1 from selected_cards = live_c
    assert!(
        game.has_pending_choice(),
        "Should have opponent select choice"
    );
    {
        let entry = game.state.ability_queue.current_entry();
        assert_eq!(
            entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
            Some("p2"),
            "Opponent-select choice should be routed to opponent"
        );
    }
    let actions = game.generated_actions();
    assert!(
        actions.len() >= 2,
        "Opponent should see at least 2 action options (got {})",
        actions.len()
    );
    game.select_option(1);

    // Step 3: Opponent-chosen card (live_c) goes to hand
    assert!(
        game.state.player1.hand.cards.contains(&live_c),
        "Opponent-chosen card (live_c) should be in hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a),
        "Unchosen live_a stays in discard"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_b),
        "Unchosen live_b stays in discard"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_c),
        "live_c should be removed from discard"
    );
}

/// Two duplicate-named + one distinct: distinct filter means only 2 selectable (the distinct + 1 of dups).
/// Player picks the distinct + the first dup → opponent picks → correct card in hand.
#[test]
fn fuyumari_q118_duplicate_names_distinct_filter_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    // live_a and live_a2 are the SAME card name (START:DASH!!)
    let live_a = game.id("PL!-sd1-019-SD"); // START:DASH!!
    let live_a2 = game.id("PL!-sd1-019-SD"); // same card, different copy
    let live_c = game.id("PL!-sd1-020-SD"); // different name

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    // Discard: [live_a (idx0), live_a2 (idx1), live_c (idx2)]
    // distinct filter should only show live_a and live_c as valid choices
    game.state.player1.waitroom.cards.push(live_a);
    game.state.player1.waitroom.cards.push(live_a2);
    game.state.player1.waitroom.cards.push(live_c);

    game.give_energy(11);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    // Step 1: Player should have a choice
    // The choice only offers 2 distinct-named cards (live_a and live_c).
    // Player picks both by choice-relative indices 0 and 1 (looked_at_cards order).
    assert!(game.has_pending_choice(), "Should have select choice");
    game.try_select_indices(&[0, 1]).unwrap();

    // Step 2: Opponent picks from the 2 selected cards
    assert!(
        game.has_pending_choice(),
        "Should have opponent select choice"
    );
    {
        let entry = game.state.ability_queue.current_entry();
        assert_eq!(
            entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
            Some("p2"),
            "Opponent-select choice should be routed to opponent"
        );
    }
    let actions = game.generated_actions();
    assert_eq!(
        actions.len(),
        2,
        "Opponent should see exactly 2 action options (selected_cards instead of wrong card)"
    );
    game.select_option(0);

    // Step 3: Opponent-chosen card (live_a) goes to hand
    assert!(
        game.state.player1.hand.cards.contains(&live_a),
        "Opponent-chosen card (live_a) should be in hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_c),
        "Unchosen live_c stays in discard"
    );
    // live_a2 should be unaffected
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a2),
        "live_a2 (duplicate name, unchosen) stays in discard"
    );
}

fn total_cards(p: &rabuka_engine::player::Player) -> usize {
    p.hand.cards.len()
        + p.main_deck.cards.len()
        + p.waitroom.cards.len()
        + p.stage.stage.iter().filter(|&&id| id != -1).count()
        + p.energy_zone.cards.len()
        + p.live_card_zone.cards.len()
        + p.success_live_card_zone.cards.len()
}
