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

    let excluded_groups = vec!["DOLLCHESTRA".to_string()];
    let filter_exclude = rabuka_engine::ability::util::CardFilter {
        exclude_group_names: Some(excluded_groups.as_slice()),
        ..Default::default()
    };
    assert!(
        filter_exclude.has_filter(),
        "exclude_group_names should make has_filter true"
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
    // Answer what cannot be declined, decline what can: a mandatory prompt
    // left parked mid-ability, and the absence assertion that usually
    // follows is exactly what a wrong answer produces.
    game.drain_choices();

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
    // Answer what cannot be declined, decline what can: a mandatory prompt
    // left parked mid-ability, and the absence assertion that usually
    // follows is exactly what a wrong answer produces.
    game.drain_choices();

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

    // All looked-at cards remain visible; non-matching ones are greyed out
    let looked_at = game.state.looked_at_cards.clone();
    assert_eq!(looked_at.len(), 2, "All looked-at cards should be visible");
    // Verify the choice has filtered_indices (only matching cards are selectable)
    let pending = game.state.get_pending_choice_json();
    let filtered = pending
        .as_ref()
        .and_then(|v| v.get("filtered_indices"))
        .and_then(|v| v.as_array());
    assert!(filtered.is_some(), "Choice should have filtered_indices");
    assert_eq!(
        filtered.unwrap().len(),
        1,
        "Only 1 card should be selectable"
    );

    // Select it
    game.select_indices(&[0]);
    // Answer what cannot be declined, decline what can: a mandatory prompt
    // left parked mid-ability, and the absence assertion that usually
    // follows is exactly what a wrong answer produces.
    game.drain_choices();

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

/// 絢瀬絵里's filter ACCEPTS a μ's card with NO abilities (the no_ability branch).
#[test]
fn ayase_accepts_no_ability_mus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayase = game.id("PL!-bp6-002-R");
    let no_ability_mus = game.id("PL!-bp6-018-N"); // μ's, no abilities
    let non_mus = game.id("PL!HS-sd1-010-SD"); // not μ's, will be rejected

    game.state.player1.hand.cards.push(ayase);
    game.give_energy(2);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(non_mus);
    game.state.player1.main_deck.cards.push(no_ability_mus);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(ayase, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Must have a choice when a no-ability μ's card is available"
    );

    let pending = game.state.get_pending_choice_json();
    let filtered = pending
        .as_ref()
        .and_then(|v| v.get("filtered_indices"))
        .and_then(|v| v.as_array());
    assert!(filtered.is_some(), "Choice should have filtered_indices");
    assert_eq!(
        filtered.unwrap().len(),
        1,
        "Only 1 card should be selectable (the no-ability one)"
    );

    game.select_indices(&[0]);
    // Answer what cannot be declined, decline what can: a mandatory prompt
    // left parked mid-ability, and the absence assertion that usually
    // follows is exactly what a wrong answer produces.
    game.drain_choices();

    assert!(
        game.state.player1.hand.cards.contains(&no_ability_mus),
        "No-ability μ's card should be added to hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_mus),
        "Non-μ's card should be discarded"
    );
}

/// Both looked-at cards match the filter (no_ability + 常時), but only 1 selectable.
#[test]
fn ayase_both_match_only_one_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayase = game.id("PL!-bp6-002-R");
    let no_ability_mus = game.id("PL!-bp6-018-N");
    let jyouji_mus = game.id("PL!-bp6-012-N");

    game.state.player1.hand.cards.push(ayase);
    game.give_energy(2);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(no_ability_mus);
    game.state.player1.main_deck.cards.push(jyouji_mus);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(ayase, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "Must have a choice when both cards match"
    );

    let pending = game.state.get_pending_choice_json();
    let filtered = pending
        .as_ref()
        .and_then(|v| v.get("filtered_indices"))
        .and_then(|v| v.as_array());
    assert!(filtered.is_some(), "Choice should have filtered_indices");
    assert_eq!(
        filtered.unwrap().len(),
        2,
        "Both cards should be selectable"
    );

    game.select_indices(&[1]);
    // Answer what cannot be declined, decline what can: a mandatory prompt
    // left parked mid-ability, and the absence assertion that usually
    // follows is exactly what a wrong answer produces.
    game.drain_choices();

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "count=1 enforced: only 1 card in hand"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "The other card should be in waitroom"
    );
}

/// The group constrains BOTH branches, not just the first.
///
/// 絢瀬絵里 prints
/// 「能力を持たない『μ's』のカード**か**{{jyouji.png|常時}}能力を持つ『μ's』のカード」 —
/// 『μ's』 is named once per branch, so a card qualifies only if it is μ's **and** (has no
/// ability **or** has a 常時 ability). The group is not a property of the first branch alone.
///
/// Every other test in this file uses `PL!HS-sd1-010-SD` as its non-μ's card, and that card
/// has **no ability at all** — so it is rejected by the no-ability branch's group check and
/// never exercises the 常時 branch. A filter that applied 『μ's』 to branch 1 only would pass
/// every one of them, and a 蓮ノ空 card with a 常時 ability would wrongly reach hand.
///
/// 乙宗 梢 (`PL!HS-bp1-003-R＋`, 蓮ノ空) has a `{{jyouji.png|常時}}` ability, which is
/// exactly the card that separates the two readings. 矢澤にこ (`PL!-bp6-018-N`, μ's, no
/// ability) sits alongside so the prompt still opens and the selectability count is exact.
#[test]
fn ayase_rejects_a_non_mus_card_even_when_it_has_the_required_constant_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayase = game.id("PL!-bp6-002-R");

    // 乙宗 梢: 蓮ノ空, and its first ability is a 常時.
    let non_mus_with_jyouji = game.id("PL!HS-bp1-003-R＋");
    // 矢澤にこ: μ's with no ability — qualifies via the FIRST branch.
    let mus_no_ability = game.id("PL!-bp6-018-N");

    game.state.player1.hand.cards.push(ayase);
    game.give_energy(2);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(non_mus_with_jyouji);
    game.state.player1.main_deck.cards.push(mus_no_ability);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(ayase, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "precondition: 矢澤にこ qualifies, so the selection prompt must open"
    );
    let looked_at = game.state.looked_at_cards.clone();
    assert_eq!(
        looked_at.len(),
        2,
        "precondition: both looked-at cards stay visible, only their selectability differs"
    );
    let pending = game.state.get_pending_choice_json();
    let filtered = pending
        .as_ref()
        .and_then(|v| v.get("filtered_indices"))
        .and_then(|v| v.as_array())
        .cloned()
        .expect("the SelectCard choice must carry filtered_indices");
    assert_eq!(
        filtered.len(),
        1,
        "『μ's』のカード governs BOTH branches: a 蓮ノ空 card is not a μ's card even though it \
         has the required 常時 ability, so only 矢澤にこ is selectable (filtered={filtered:?})"
    );
    assert_eq!(
        filtered[0], 1,
        "the selectable card must be the μ's no-ability one, i.e. index 1, not the 常時 card"
    );

    game.select_indices(&[0]);
    game.drain_choices();

    assert!(
        game.state.player1.hand.cards.contains(&mus_no_ability),
        "the qualifying μ's card reaches hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&non_mus_with_jyouji),
        "常時能力を持つ『μ's』のカード — the ability branch still requires μ's, so a 蓮ノ空 \
         常時 card must not be taken (hand={:?})",
        game.state.player1.hand.cards
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_mus_with_jyouji),
        "残りを控え室に置く — the rejected card is routed to the waitroom, not lost \
         (hand={:?} waitroom={:?})",
        game.state.player1.hand.cards, game.state.player1.waitroom.cards
    );
}

/// Neither card matches → no choice, auto-discard both.
#[test]
fn ayase_neither_matches_auto_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayase = game.id("PL!-bp6-002-R");
    let debut_a = game.id("PL!-bp6-004-R");
    let debut_b = game.id("PL!-bp6-005-R");

    game.state.player1.hand.cards.push(ayase);
    game.give_energy(2);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(debut_a);
    game.state.player1.main_deck.cards.push(debut_b);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(ayase, MemberArea::Center);

    assert!(
        !game.has_pending_choice(),
        "No choice when neither card matches"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&debut_a),
        "Debut card A should be discarded"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&debut_b),
        "Debut card B should be discarded"
    );
}
