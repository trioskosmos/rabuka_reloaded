use crate::helpers::*;

/// Verify that move_cards actions respect heart_colors filtering.
/// Rin bp6 (PL!-bp6-005-R) debut: optional discard 2 → retrieve
/// heart03 member + heart03 live card from discard.
#[test]
fn rin_bp6_heart03_filter_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp6-005-R");
    // Cards with heart03 (should be selectable)
    let hc_member = game.id("PL!-sd1-001-SD"); // member, heart03 in base_heart
    let hc_live = game.id("PL!-sd1-019-SD"); // live,  heart03 in need_heart
                                             // Card WITHOUT heart03 (should NOT be selectable)
    let no_hc = game.id("PL!-sd1-002-SD"); // member, no heart03
                                           // Filler card with heart03 for discard cost (gets discarded to waitroom)
    let filler_hc = game.id("PL!-sd1-010-SD"); // has heart03 but only serves as discard fodder

    game.state.player1.hand.cards.push(rin);
    game.state.player1.hand.cards.push(filler_hc);
    game.state.player1.hand.cards.push(filler_hc);

    // Put heart03 cards and the non-heart03 card in discard
    game.state.player1.waitroom.cards.push(hc_member);
    game.state.player1.waitroom.cards.push(hc_live);
    game.state.player1.waitroom.cards.push(no_hc);

    // Fill decks
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler_hc);
        game.state.player2.main_deck.cards.push(filler_hc);
    }
    game.give_energy(11);

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, rabuka_engine::zones::MemberArea::Center);

    // Pay optional discard cost: discard 2 from hand
    assert!(
        game.has_pending_choice(),
        "Should have optional cost choice"
    );
    game.select_indices(&[0, 1]);

    // First sequential sub-action: select heart03 card from discard
    assert!(game.has_pending_choice(), "Should have first select choice");
    {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard {
                zone,
                filtered_indices,
                ..
            } => {
                assert_eq!(zone, "discard", "First choice zone should be discard");
                let indices = filtered_indices.as_ref().unwrap();
                let zone_cards = &game.state.player1.waitroom.cards;
                for &idx in indices {
                    assert!(idx < zone_cards.len(), "Index {} out of bounds", idx);
                    let cid = zone_cards[idx];
                    let card = game.db.get_card(cid).unwrap();
                    let has_hc03 = card.base_heart.as_ref().is_some_and(|bh| {
                        bh.hearts
                            .contains_key(&rabuka_engine::card::HeartColor::Heart03)
                    }) || card.need_heart.as_ref().is_some_and(|nh| {
                        nh.hearts
                            .contains_key(&rabuka_engine::card::HeartColor::Heart03)
                    });
                    assert!(
                        has_hc03,
                        "Card at index {} ({}) should have heart03",
                        idx, card.name
                    );
                }
                // Non-heart03 card must NOT be in filtered_indices
                let no_hc_idx = zone_cards.iter().position(|&c| c == no_hc).unwrap();
                assert!(
                    !indices.contains(&no_hc_idx),
                    "Non-heart03 card should not be selectable"
                );
            }
            _ => panic!("Expected SelectCard choice, got {:?}", choice),
        }
    }

    // Select first available heart03 card
    game.select_indices(&[0]);

    // Second sequential sub-action: heart03 live_card select from discard
    assert!(
        game.has_pending_choice(),
        "second heart03 live-card select prompt expected"
    );
    {
        let choice2 = game.get_pending_choice();
        match choice2 {
            rabuka_engine::ability::types::Choice::SelectCard {
                zone,
                filtered_indices,
                ..
            } => {
                assert_eq!(zone, "discard", "Second choice zone should be discard");
                let indices = filtered_indices.as_ref().unwrap();
                let zone_cards = &game.state.player1.waitroom.cards;
                for &idx in indices {
                    assert!(idx < zone_cards.len(), "Index {} out of bounds", idx);
                    let cid = zone_cards[idx];
                    let card = game.db.get_card(cid).unwrap();
                    let has_hc03 = card.base_heart.as_ref().is_some_and(|bh| {
                        bh.hearts
                            .contains_key(&rabuka_engine::card::HeartColor::Heart03)
                    }) || card.need_heart.as_ref().is_some_and(|nh| {
                        nh.hearts
                            .contains_key(&rabuka_engine::card::HeartColor::Heart03)
                    });
                    assert!(
                        has_hc03,
                        "Card at index {} ({}) should have heart03 in second choice",
                        idx, card.name
                    );
                }
                let no_hc_idx = zone_cards.iter().position(|&c| c == no_hc).unwrap();
                assert!(
                    !indices.contains(&no_hc_idx),
                    "Non-heart03 card should not be in second choice"
                );
            }
            _ => panic!("Expected SelectCard choice, got {:?}", choice2),
        }

        // Observed: this prompt has allow_skip=true (live retrieval is optional);
        // answering empty finalizes the ability.
        let allow_skip = match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { allow_skip, .. } => *allow_skip,
            _ => false,
        };
        assert!(allow_skip, "observed: live-card select allows skip");
        let hand_before_skip = game.state.player1.hand.cards.len();
        game.select_indices(&[]);
        assert_eq!(
            game.state.player1.hand.cards.len(),
            hand_before_skip,
            "skip branch must not add cards to hand"
        );
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // At least one heart03 card was retrieved
    let has_member = game.state.player1.hand.cards.contains(&hc_member);
    let has_live = game.state.player1.hand.cards.contains(&hc_live);
    assert!(
        has_member || has_live,
        "At least one heart03 card should be in hand"
    );

    // Non-heart03 card stays in discard
    assert!(
        game.state.player1.waitroom.cards.contains(&no_hc),
        "Non-heart03 card must remain in discard"
    );
}

/// Edge case: discard contains ONLY non-heart03 cards → both choices should
/// offer 0 selectable cards (allow_skip=true, no filtered_indices).
#[test]
fn rin_bp6_no_heart03_cards_skips_cleanly() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp6-005-R");
    let filler = game.id("PL!-sd1-002-SD"); // member, no heart03

    game.state.player1.hand.cards.push(rin);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    // Only non-heart03 cards in discard
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(11);

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, rabuka_engine::zones::MemberArea::Center);

    // Pay optional discard cost
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[0, 1]);

    // Observed: with zero heart03 candidates BOTH sequential sub-actions auto-skip —
    // no SelectCard prompt is created at all (FINALIZE_MOVE cards=[] with pending=false).
    assert!(
        !game.has_pending_choice(),
        "zero-candidate selects should auto-skip without prompting"
    );

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Verify no cards were moved to hand
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "No cards should be in hand (only Rin played)"
    );
}

/// Edge case: only one type matches (e.g. only heart03 members in discard,
/// no heart03 live cards). Should allow skip for the unmatched type.
#[test]
fn rin_bp6_only_member_matches_skips_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp6-005-R");
    let hc_member = game.id("PL!-sd1-001-SD"); // member with heart03
    let no_hc = game.id("PL!-sd1-002-SD"); // member without heart03 (for discard)

    game.state.player1.hand.cards.push(rin);
    game.state.player1.hand.cards.push(no_hc);
    game.state.player1.hand.cards.push(no_hc);

    // Only a heart03 MEMBER in discard (no heart03 live card)
    game.state.player1.waitroom.cards.push(hc_member);
    game.state.player1.waitroom.cards.push(no_hc);

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(no_hc);
        game.state.player2.main_deck.cards.push(no_hc);
    }
    game.give_energy(11);

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(rin, rabuka_engine::zones::MemberArea::Center);

    // Pay optional discard cost
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[0, 1]);

    // First sub-action: member_card select — observed: engine prompts with exactly
    // hc_member selectable (filtered=[0], allow_skip=true).
    assert!(
        game.has_pending_choice(),
        "first sub-action (heart03 member select) prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for heart03 member select"
    );
    {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard {
                zone,
                filtered_indices,
                ..
            } => {
                assert_eq!(zone, "discard");
                let indices = filtered_indices.as_ref().unwrap();
                assert_eq!(indices.len(), 1, "only hc_member should be selectable");
                let zone_cards = &game.state.player1.waitroom.cards;
                for &idx in indices {
                    let cid = zone_cards[idx];
                    let card = game.db.get_card(cid).unwrap();
                    let has_hc03 = card.base_heart.as_ref().is_some_and(|bh| {
                        bh.hearts
                            .contains_key(&rabuka_engine::card::HeartColor::Heart03)
                    });
                    assert!(has_hc03, "Selectable card must have heart03");
                }
            }
            _ => panic!("Expected SelectCard, got {:?}", choice),
        }
    }
    game.select_indices(&[0]);

    // Second sub-action: zero eligible heart03 live cards → engine auto-skips, no prompt.
    assert!(
        !game.has_pending_choice(),
        "live-card select should auto-skip without prompting when no live matches"
    );

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // hc_member should be in hand
    assert!(
        game.state.player1.hand.cards.contains(&hc_member),
        "Heart03 member card should have been retrieved"
    );
}
