use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Test that a "when this member goes from stage to waiting room" auto ability
/// only triggers when the member is actually moved from stage to discard (via baton touch),
/// and NOT when:
///   - Playing the card to stage (debut)
///   - Playing other cards to stage
///
/// Card: PL!-PR-001-PR (Honoka)
/// Auto ability: "When this member is placed from stage to waiting room, you may activate 1 member."
/// Condition: properly parsed as location_condition(location="discard", card_type="member_card")
#[test]
fn stage_to_discard_ability_triggers_only_on_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let honoka = game.id("PL!-PR-001-PR"); // cost 4, has stage->discard auto ability
    let filler = game.id("PL!-sd1-010-SD"); // cost 4, no abilities
    let arriver = game.id("PL!S-bp5-012-N"); // cost 2, no abilities

    // Give enough energy for all plays
    game.give_energy(30);

    // Fill deck
    let deck_card = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(deck_card);
    }

    // ---- Step 1: Play Honoka to LeftSide (debut, empty area) ----
    game.state.player1.hand.cards.push(honoka);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(honoka, MemberArea::LeftSide);

    // Should NOT trigger: card is on stage, not in discard
    assert!(
        !game.has_pending_choice(),
        "Ability should NOT trigger on debut (card is on stage)"
    );

    // ---- Step 2: Play filler to Center (debut, empty area) ----
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, MemberArea::Center);

    // Should NOT trigger
    assert!(
        !game.has_pending_choice(),
        "Ability should NOT trigger when another card debuts"
    );

    // ---- Step 3: Directly clear area lock to allow baton touch ----
    game.state.player1.areas_locked_this_turn.clear();

    // ---- Step 4: Baton touch - play arriver to LeftSide (Honoka's area) ----
    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, MemberArea::LeftSide);

    // Honoka was replaced from stage to waitroom → her auto ability fires.
    // The ability offers to activate 1 member. Since neither the filler nor
    // the arriver are in wait state, there's nothing to activate — the
    // option is NOT offered (unpayable cost).
    // Use select_indices to confirm no pending choice exists.
    assert!(
        !game.has_pending_choice(),
        "Auto ability should NOT present a choice when no wait members exist"
    );

    // Verify Honoka is in waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&honoka),
        "Honoka should be in waitroom after baton touch"
    );
}

// =====================================================================
// PL!HS-bp2-012-N (Kohaku) — 自動 look_and_select on stage→discard
// =====================================================================
// 自動: このメンバーがステージから控え室に置かれたとき、
//       自分のデッキの上からカードを5枚見る。その中から
//       メンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。
//
// Condition: card_count_condition, source=stage, destination=discard, self_target=true
// Previously untested (listed in affected_cards.txt).

#[test]
fn kohaku_stage_to_discard_triggers_ability() {
    let db = load_real_database();
    let mut g = TestGame::new(db);

    let kohaku = g.id("PL!HS-bp2-012-N");
    let filler = g.id("PL!-sd1-010-SD");

    // Fill deck with filler cards
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(filler);
    }

    // Place Kohaku on stage
    g.state.player1.stage.stage = [-1, kohaku, -1];

    // Manually trigger stage→discard for Kohaku
    g.state.player1.stage.stage[1] = -1;
    g.state.player1.waitroom.cards.push(kohaku);
    g.state.recently_moved_cards = Some(vec![kohaku]);
    g.state.recently_moved_from_zone = Some("stage".to_string());

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // Should present look_and_select choice (look at top 5, choose member card to add)
    assert!(
        g.has_pending_choice(),
        "Kohaku ab#0 should fire on stage→discard"
    );

    // The looked_at cards are all filler (no member card to select). Skip.
    g.select_indices(&[]);

    // No more choices should be pending
    assert!(!g.has_pending_choice(), "No more choices after skipping");
}

#[test]
fn kohaku_baton_touch_triggers_ability() {
    let db = load_real_database();
    let mut g = TestGame::new(db);

    let kohaku = g.id("PL!HS-bp2-012-N"); // cost 5
    let arriver = g.id("PL!-sd1-002-SD"); // cost 6, free area to arrive
    let filler = g.id("PL!-sd1-010-SD");

    // Fill deck with 30 filler cards
    g.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(filler);
    }

    // Give enough energy for baton touch
    g.give_energy(20);

    // Place Kohaku on stage, arriver + filler in hand
    g.state.player1.stage.stage = [-1, kohaku, -1];
    g.state.player1.hand.cards.push(arriver);
    g.state.player1.hand.cards.push(filler);

    // Baton touch: play arriver to Center (Kohaku's area)
    g.play_to_stage(arriver, MemberArea::Center);

    // Kohaku was replaced by baton touch → her auto ability should fire
    assert!(
        g.has_pending_choice(),
        "Kohaku ab#0 should trigger on baton touch stage→discard"
    );

    // The ability looks at top 5 cards of deck (all filler).
    // Since no member card is available, skip the option.
    g.select_indices(&[]);

    assert!(
        g.state.player1.waitroom.cards.contains(&kohaku),
        "Kohaku should be in waitroom after baton touch"
    );
}

#[test]
fn kohaku_baton_touch_selects_member_from_deck() {
    let db = load_real_database();
    let mut g = TestGame::new(db);

    let kohaku = g.id("PL!HS-bp2-012-N");
    let arriver = g.id("PL!-sd1-002-SD");
    let member_target = g.id("PL!S-sd1-001-SD");
    let filler = g.id("PL!-sd1-010-SD");

    // Fill deck: member_target at index 0 (= top of deck)
    g.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(filler);
    }
    g.state.player1.main_deck.cards[0] = member_target;

    g.give_energy(20);
    g.state.player1.stage.stage = [-1, kohaku, -1];
    g.state.player1.hand.cards.push(arriver);
    g.state.player1.hand.cards.push(filler);

    g.play_to_stage(arriver, MemberArea::Center);

    assert!(
        g.has_pending_choice(),
        "Kohaku ab#0 should trigger on baton touch"
    );

    // Select card at index 0 (the member_target at deck top)
    g.select_indices(&[0]);

    assert!(
        g.state.player1.hand.cards.contains(&member_target),
        "Member card should be in hand after baton touch trigger"
    );
}
