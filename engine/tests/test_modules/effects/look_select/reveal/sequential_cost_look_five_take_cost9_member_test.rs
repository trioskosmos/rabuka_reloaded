/// Tests for PL!-bp5-002-R (Ayase Eli) — sequential_cost with wait + optional discard
use crate::helpers::*;

/// The subject (PL!-bp5-002-R, cost 4) and the cost-9 look target
/// (PL!-bp3-002-R) are both 絢瀬絵里 — the same character printed twice — and
/// they are one bp number apart. The whole effect is 「コスト9以上のμのメンバー」,
/// so a transposition would swap a cost-9 eligible card for a cost-4 ineligible
/// one. Pin both prints and both costs.
fn eli_id(game: &mut TestGame) -> i16 {
    let id = game.id("PL!-bp5-002-R");
    game.assert_card_identity(id, "PL!-bp5-002-R");
    game.assert_card_cost(id, 4);
    id
}

fn cost9_mu_member_id(game: &mut TestGame) -> i16 {
    let id = game.id("PL!-bp3-002-R");
    game.assert_card_identity(id, "PL!-bp3-002-R");
    game.assert_card_cost(id, 9);
    id
}

/// Sequential cost: put to wait (optional), discard 1 (optional) → look at 5 from deck,
/// select cost≥9 μ's member to hand, discard rest.
#[test]
fn eli_bp5_sequential_wait_then_discard_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = eli_id(&mut game);
    let filler = game.id("PL!-sd1-010-SD");
    let mus_high_cost = cost9_mu_member_id(&mut game); // Eli, cost 9, μ's member
    game.assert_same_card_name(mus_high_cost, eli, "both prints are 絢瀬絵里");

    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);

    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(mus_high_cost);
    }
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);

    // Choice 1: wait is auto-paid (binary cost), discard prompt shows directly
    assert!(game.has_pending_choice(), "Discard prompt");
    assert_eq!(
        game.state
            .get_pending_choice_json()
            .as_ref()
            .and_then(|v| v.get("zone"))
            .and_then(|v| v.as_str()),
        Some("hand"),
        "Should be hand discard, got {:?}",
        game.state.get_pending_choice_json()
    );
    game.select_indices(&[0]);

    // Remaining: looked_at selection — the first 5 are μ's cost 9 → match
    // Pick 1 to hand
    assert!(
        game.has_pending_choice(),
        "looked_at selection prompt expected after hand discard"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (looked_at)"
    );
    game.select_indices(&[0]);

    // Hand had 2 cards after play, cost discarded 1 → hand = 1,
    // then looked_at selection added 1 → hand = 2
    assert_eq!(game.state.player1.hand.cards.len(), 2);
    let w = game.state.mods.get_orientation_modifier(eli);
    assert!(w.is_some());
    assert_eq!(w.unwrap(), "wait");
}

/// Declined optional cost skips the whole effect: the bundled rest-self +
/// discard choice offers skip, and skipping completes the ability with no
/// look — member stays active, hand/deck/waitroom untouched.
#[test]
fn eli_bp5_skip_costs_skips_look_entirely() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = eli_id(&mut game);
    let filler = game.id("PL!-sd1-010-SD");
    let mus_high = cost9_mu_member_id(&mut game);
    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(mus_high); }
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    let deck_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice(), "cost prompt expected");
    game.select_indices(&[]); // skip the bundled rest + discard cost
    assert!(
        !game.has_pending_choice(),
        "skipped cost → effect never starts, no look prompt"
    );
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[filler]);
    assert!(
        game.state.mods.get_orientation_modifier(eli).is_none(),
        "skipped rest cost → member stays active"
    );
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "deck untouched: no cards looked at"
    );
    assert!(game.state.looked_at_cards.is_empty());
}

/// The コスト9以上 bound itself, isolated from the 『μのメンバー』 half.
///
/// Every other negative in this file uses `PL!-sd1-010-SD`, which fails **both** tests at
/// once — it is not a μ's card (series ラブライブ！蓮ノ空…) *and* it costs 4. So no existing
/// test distinguishes 「cost 9 or more」 from 「some μ's member」: any bound from 2 upward
/// would reject that filler just as correctly, and the only positive fixture sits exactly on
/// 9, so `>= 9`, `>= 5` and `>= 4` are indistinguishable here.
///
/// `PL!-bp4-012-N` 南 ことり is a μ's member (series exactly ラブライブ！) of **cost 7** with
/// no ability. It satisfies the group clause and fails only the cost, so a deck containing it
/// alongside a cost-9 μ's member pins the bound: exactly one card may be selectable, and it
/// must be the cost-9 one.
///
/// There is no cost-8 μ's member in the corpus — the μ's cost histogram is
/// 2,4,5,7,9,10,11,13,15,17,22 — so 7 is the closest reachable card below the bound.
#[test]
fn eli_bp5_a_mu_member_below_cost_nine_is_rejected_on_cost_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = eli_id(&mut game);
    let filler = game.id("PL!-sd1-010-SD"); // the hand card paying the discard cost
    let nine = cost9_mu_member_id(&mut game);
    let seven = game.id("PL!-bp4-012-N");
    game.assert_card_identity(seven, "PL!-bp4-012-N");
    game.assert_card_cost(seven, 7);

    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);
    // Look at 5: the cost-9 card plus four cost-7 cards, all of them μ's members.
    // The below-bound cards must be excluded on cost, not on group.
    game.state.player1.main_deck.cards.push(nine);
    for _ in 0..4 {
        game.state.player1.main_deck.cards.push(game.new_id("PL!-bp4-012-N"));
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);

    // Pay the hand discard.
    assert!(game.has_pending_choice(), "cost prompt expected");
    game.select_indices(&[0]);

    assert!(
        game.has_pending_choice(),
        "precondition: the cost-9 card qualifies, so the look prompt opens"
    );
    assert_eq!(game.pending_choice_option_count(), Some(1),
        "コスト9以上のμのメンバー — the four cost-7 μ's members pass 『μの』 and fail only \
         the cost, so exactly the one cost-9 card may be selectable");
    let offered = game
        .state
        .get_pending_choice_json()
        .and_then(|v| v["selection_cards"].as_array().cloned())
        .unwrap_or_default();
    let selectable_is_nine = offered
        .iter()
        .filter(|c| c["id"].as_i64() == Some(nine as i64))
        .count()
        == 1;
    assert!(
        selectable_is_nine,
        "the single selectable card must be the cost-9 one, not one of the cost-7s"
    );
    // A cost-7 copy must NOT be selectable. `selection_cards` deliberately lists ALL
    // looked-at cards (the UI greys the non-matching ones), so the count comes from
    // `filtered_indices` via pending_choice_option_count() above, and identity is checked
    // here by confirming the cost-9 card is among the offered set at all.
    assert!(
        offered.iter().any(|c| c["id"].as_i64() == Some(nine as i64)),
        "the cost-9 card must be present in the offered set (offered={offered:?})"
    );

    game.select_indices(&[0]);
    assert!(
        game.state.player1.hand.cards.contains(&nine),
        "the cost-9 μ's member reaches hand (hand={:?})",
        game.state.player1.hand.cards
    );
    assert!(
        !game.state.player1.hand.cards.contains(&seven),
        "no cost-7 μ's member may be taken (hand={:?})",
        game.state.player1.hand.cards
    );
    let _ = seven;
}

#[test]
fn eli_bp5_no_eligible_look_discards_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = eli_id(&mut game);
    let filler = game.id("PL!-sd1-010-SD"); // filler is not μ's cost 9, so no eligible
    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    let wait_before = game.state.player1.waitroom.cards.len();
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]); // discard filler
    // Drain look choices (with no eligible, engine may auto-discard)
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Eli played (1 filler left) then that filler discarded as the look \
         cost: the hand ends empty, not '0 or 1'"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len() - wait_before,
        6,
        "The look cost card plus all 5 looked-at cards, none of which was \
         eligible; '>= 5' would pass if the engine kept any of them"
    );
}



#[test]
fn eli_bp5_look_select_optional_skip_keeps_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = eli_id(&mut game);
    let filler = game.id("PL!-sd1-010-SD");
    let mus_high = cost9_mu_member_id(&mut game);
    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);
    // Top 5: 2 eligible, 3 filler
    for _ in 0..2 { game.state.player1.main_deck.cards.push(mus_high); }
    for _ in 0..3 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    // Drain look choices, skip optional select even though eligible exists
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Skipping the optional select takes nothing from the looked-at cards \
         (which is the point), but the look's own discard cost still takes \
         the filler — so the hand ends empty rather than holding the looked \
         card. '<= 1' passed both of those, and a bug that put a looked card \
         in hand."
    );
}
