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

    // Second sequential sub-action: select the other type
    if game.has_pending_choice() {
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
        }

        if game.has_pending_choice() {
            let cb = game.get_pending_choice();
            let allow_skip = match cb {
                rabuka_engine::ability::types::Choice::SelectCard { allow_skip, .. } => *allow_skip,
                _ => false,
            };
            if allow_skip {
                // Skip branch: hand must not change from this choice
                let hand_before_skip = game.state.player1.hand.cards.len();
                game.select_indices(&[]);
                assert_eq!(
                    game.state.player1.hand.cards.len(),
                    hand_before_skip,
                    "skip branch must not add cards to hand"
                );
            } else {
                game.select_indices(&[0]);
            }
        }
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

    // First sub-action: no heart03 cards → should auto-skip (no choice created)
    // Second sub-action: also no heart03 cards → should auto-skip
    // If a choice is created, verify it shows 0 options and auto-skip
    if game.has_pending_choice() {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard {
                filtered_indices,
                allow_skip,
                ..
            } => {
                let indices = filtered_indices.as_ref().unwrap();
                assert!(
                    indices.is_empty(),
                    "No cards should be selectable when none have heart03"
                );
                assert!(*allow_skip, "Should allow skip when no cards match");
                game.select_indices(&[]);
            }
            _ => panic!("Expected SelectCard, got {:?}", choice),
        }
    }
    if game.has_pending_choice() {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard {
                filtered_indices,
                allow_skip,
                ..
            } => {
                assert!(
                    filtered_indices.as_ref().unwrap().is_empty(),
                    "No cards should be selectable"
                );
                assert!(*allow_skip, "Should allow skip");
                game.select_indices(&[]);
            }
            _ => panic!("Expected SelectCard, got {:?}", choice),
        }
    }

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

    // First sub-action: depends on abilities.json order.
    // If live_card first and no heart03 live cards → auto-skip or empty choice.
    // If member_card first → should offer hc_member.
    // Handle whichever comes first:
    if game.has_pending_choice() {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard {
                zone,
                filtered_indices,
                ..
            } => {
                assert_eq!(zone, "discard");
                let indices = filtered_indices.as_ref().unwrap();
                if indices.is_empty() {
                    // This type has no matches → skip
                    assert!(matches!(
                        choice,
                        rabuka_engine::ability::types::Choice::SelectCard {
                            allow_skip: true,
                            ..
                        }
                    ));
                    game.select_indices(&[]);
                } else {
                    // This type has matches → select one
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
                    game.select_indices(&[0]);
                }
            }
            _ => panic!("Expected SelectCard, got {:?}", choice),
        }
    }

    // Second sub-action
    if game.has_pending_choice() {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard {
                filtered_indices,
                allow_skip,
                ..
            } => {
                let indices = filtered_indices.as_ref().unwrap();
                if indices.is_empty() {
                    assert!(*allow_skip, "Should allow skip when no matches");
                    game.select_indices(&[]);
                } else {
                    game.select_indices(&[0]);
                }
            }
            _ => panic!("Expected SelectCard, got {:?}", choice),
        }
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // hc_member should be in hand
    assert!(
        game.state.player1.hand.cards.contains(&hc_member),
        "Heart03 member card should have been retrieved"
    );
}
