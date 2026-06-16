/// Tests for CardFilter::has_filter and or_ability_filters filtering.
///
/// Bug: execute_select_cards skipped filtering when the only filter fields
/// were group_names/or_ability_filters (not card_type/heart_colors/cost_limit),
/// allowing the user to select any μ's card regardless of abilities.
///
/// Card: PL!-bp6-002-R (絢瀬絵里) — debut: look top 2, select a μ's card
///       with no ability OR with 常時 ability, discard the rest.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Helper: verify the `has_filter` method on CardFilter works for all field types.
#[test]
fn has_filter_detects_all_fields() {
    let filter_type = rabuka_engine::ability::util::CardFilter::new().card_type("member_card");
    assert!(
        filter_type.has_filter(),
        "card_type should make has_filter true"
    );

    let filter_group = rabuka_engine::ability::util::CardFilter::new().group("μ's");
    assert!(
        filter_group.has_filter(),
        "group should make has_filter true"
    );

    let empty = rabuka_engine::ability::util::CardFilter::new();
    assert!(
        !empty.has_filter(),
        "empty filter should have has_filter=false"
    );
}

/// 絢瀬絵里's filter rejects non-μ's cards via group check.
/// Uses PL!-bp6-005-R (μ's unit, NOT EdelNote or other groups).
#[test]
fn ayase_group_filter_rejects_non_mus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayase = game.id("PL!-bp6-002-R");
    // Use two NON-μ's cards from a different franchise
    let non_mus = game.id("PL!HS-sd1-010-SD"); // 蓮ノ空 — not μ's

    game.state.player1.hand.cards.push(ayase);
    game.give_energy(2);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(non_mus);
    game.state.player1.main_deck.cards.push(non_mus);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(ayase, MemberArea::Center);

    // Drain — no selectable cards remain after group filter
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let non_mus_waitroom = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .filter(|&&id| id == non_mus)
        .count();
    assert_eq!(
        non_mus_waitroom, 2,
        "Both non-μ's cards must be discarded to waitroom"
    );
}

/// 絢瀬絵里's filter rejects μ's cards with debut/登場 ability (not 常時).
/// The debut card has abilities (fails no_ability) and trigger 登場 ≠ 常時.
#[test]
fn ayase_rejects_debut_trigger_mus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayase = game.id("PL!-bp6-002-R");
    let debut_mus = game.id("PL!-bp6-004-R"); // μ's, trigger=登場
                                              // Second card is also μ's with debut trigger
    let second_debut = game.id("PL!-bp6-005-R"); // μ's, trigger=登場

    game.state.player1.hand.cards.push(ayase);
    game.give_energy(2);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(debut_mus);
    game.state.player1.main_deck.cards.push(second_debut);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(ayase, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Both debut cards should be rejected (has abilities, not 常時)
    assert!(
        game.state.player1.waitroom.cards.contains(&debut_mus),
        "Debut μ's card must be filtered out"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&second_debut),
        "Second debut μ's card must be filtered out"
    );
}

/// 絢瀬絵里's filter KEEPS μ's cards with 常時 ability and rejects
/// those with debut (登場) ability, keeping only the 常時 one.
#[test]
fn ayase_accepts_jyouji_rejects_debut() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayase = game.id("PL!-bp6-002-R");
    let jyouji_mus = game.id("PL!-bp6-012-N"); // μ's, trigger=常時
    let debut_mus = game.id("PL!-bp6-004-R"); // μ's, trigger=登場

    game.state.player1.hand.cards.push(ayase);
    game.give_energy(2);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(jyouji_mus);
    game.state.player1.main_deck.cards.push(debut_mus);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(ayase, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Must have a choice when a 常時 μ's card is available"
    );

    // Only 1 card should pass the filter (the 常時 one)
    let looked_at = game.state.looked_at_cards.clone();
    assert_eq!(looked_at.len(), 1, "Only 1 card should pass filter");
    assert_eq!(
        looked_at[0], jyouji_mus,
        "The 常時 card must be the one available"
    );

    // Select it
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // 常時 card goes to hand, debut card goes to waitroom
    assert!(
        game.state.player1.hand.cards.contains(&jyouji_mus),
        "常時 card should be added to hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&debut_mus),
        "Debut card should be discarded"
    );
}
