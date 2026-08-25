/// Untested-abilities batch 32 — ライブ成功時 conditional draws/looks/retrievals.
///
/// - PL!HS-sd1-017-SD 夏めきRain: 蓮ノ空 member on stage → draw 1, discard 1.
/// - PL!N-sd2-007-P 優木せつ菜: draw 1; opponent also succeeded this turn →
///   draw 1 more + discard 1.
/// - PL!-bp6-016-N 東條希: look at top 3, put them back on deck top in any order.
/// - PL!S-bp7-019-L: up to 2 『Aqours』 cards from waitroom to deck bottom.
/// - PL!SP-bp5-027-L HOT PASSION!!: optional waited energy from energy deck;
///   if placed, OPPONENT draws 1.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

// ====================================================================
// PL!HS-sd1-017-SD 夏めきRain — gated draw+discard
// ====================================================================

#[test]
fn natsumi_rain_draws_and_discards_with_hasunosora_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-017-SD");
    let hino = game.id("PL!HS-bp5-001-P"); // 蓮ノ空 member

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    // Direct stage placement: testing the trigger gate, not the debut pipeline.
    game.state.player1.stage.stage[0] = hino;
    // A discard target for the second step.
    let hand_filler = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_filler);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    if game.has_pending_choice() {
        game.select_indices(&[0]); // choose which hand card to discard
    }

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "gate met -> draw 1"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hand_filler),
        "second step discards 1 hand card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_filler),
        "discarded card lands in the waitroom"
    );
}

#[test]
fn natsumi_rain_no_hasunosora_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!HS-sd1-017-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    // Stage holds a NON-蓮ノ空 member (μ's filler).
    game.state.player1.stage.stage[0] = game.id("PL!-sd1-010-SD");

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        0,
        "no 蓮ノ空 member on stage -> nothing happens"
    );
}

// ====================================================================
// PL!N-sd2-007-P 優木せつ菜 — opponent-live-success conditional
// ====================================================================

#[test]
fn sd2_007_extra_draw_when_opponent_also_succeeded() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-sd2-007-P");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(setsuna);
    // Discard fuel for the follow-up hand-discard.
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));

    // The engine tracks "opponent also succeeded this turn" via this flag
    // (set by the live-victory flow when P2's live succeeds).
    game.state.opponent_live_success_this_turn = true;

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, setsuna, AbilityTrigger::LiveSuccess, "ライブ成功時");
    if game.has_pending_choice() {
        game.select_indices(&[0]); // choose which hand card to discard
    }

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "opponent succeeded too -> base draw + extra draw"
    );
}

#[test]
fn sd2_007_single_draw_without_opponent_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-sd2-007-P");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(setsuna);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, setsuna, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "opponent did not succeed -> only the base draw"
    );
}

// ====================================================================
// PL!-bp6-016-N 東條希 — look top 3, reorder onto deck top
// ====================================================================

#[test]
fn bp6_016_look_three_reorder_back_on_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp6-016-N");
    let a = game.new_id("PL!-sd1-010-SD");
    let b = game.new_id("PL!S-sd1-001-SD");
    let c = game.new_id("PL!N-sd1-025-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(nozomi);
    // Deck top order (index 0 = top): [a, b, c].
    game.state.player1.main_deck.cards.insert(0, c);
    game.state.player1.main_deck.cards.insert(0, b);
    game.state.player1.main_deck.cards.insert(0, a);
    game.give_energy(6);

    game.play_to_stage(nozomi, rabuka_engine::zones::MemberArea::Center);

    // Answer every look/placement prompt by taking the first offered card.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    let top3: Vec<i16> = game.state.player1.main_deck.cards[..3].to_vec();
    let mut sorted = top3.clone();
    sorted.sort();
    let mut expected = vec![a, b, c];
    expected.sort();
    assert_eq!(
        sorted, expected,
        "all three looked-at cards return to the deck top (any order)"
    );
}

// ====================================================================
// PL!S-bp7-019-L — up to 2 Aqours cards from waitroom to deck bottom
// ====================================================================

#[test]
fn bp7_019_places_two_aqours_cards_under_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp7-019-L");
    let aq1 = game.id("PL!S-sd1-003-SD"); // Aqours member
    let aq2 = game.id("PL!S-sd1-017-SD"); // Aqours member (鞠莉)
    let non_aq = game.new_id("PL!-sd1-010-SD"); // μ's — must stay behind

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.waitroom.cards.push(non_aq);
    game.state.player1.waitroom.cards.push(aq1);
    game.state.player1.waitroom.cards.push(aq2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "valid candidates -> selection prompt");
    game.select_indices(&[0, 1]);

    assert!(
        !game.state.player1.waitroom.cards.contains(&aq1)
            && !game.state.player1.waitroom.cards.contains(&aq2),
        "both Aqours cards left the waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_aq),
        "non-Aqours card stays in the waitroom"
    );
    let deck = &game.state.player1.main_deck.cards;
    assert_eq!(deck.len(), 32, "30 fillers + 2 returned cards");
    assert!(
        deck.ends_with(&vec![aq1, aq2]) || deck.ends_with(&vec![aq2, aq1]),
        "the two Aqours cards sit at the deck BOTTOM (either order)"
    );
}

// ====================================================================
// PL!SP-bp5-027-L HOT PASSION!! — optional waited energy; opponent draws
// ====================================================================

fn hot_passion_setup(game: &mut TestGame) -> i16 {
    let live = game.id("PL!SP-bp5-027-L");
    fill_decks(game, { let f = game.new_id("PL!-sd1-010-SD"); f });
    game.state.player1.live_card_zone.cards.push(live);
    fill_energy_deck(game, 0, 2);
    live
}

#[test]
fn hot_passion_accept_energy_opponent_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = hot_passion_setup(&mut game);

    let p2_hand_before = game.state.player2.hand.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "optional energy placement prompted");
    // Pay/skip gate: options ["No", "Yes"] — pick "Yes".
    game.select_option(1);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        1,
        "one energy card placed into the energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "the placed energy arrives WAITED"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before + 1,
        "placement accepted -> opponent draws 1"
    );
}

#[test]
fn hot_passion_decline_no_opponent_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = hot_passion_setup(&mut game);

    let p2_hand_before = game.state.player2.hand.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "optional energy placement prompted");
    game.select_indices(&[]); // decline

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        0,
        "declined -> no energy placed"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before,
        "declined -> opponent draws nothing"
    );
}
