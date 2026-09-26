use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

/// 渡辺 曜 bp6-005-R: 登場 look at the top 2 cards of your deck, then you MAY
/// reveal 1 member carrying heart02 + heart04 + heart05 into hand; the rest go
/// to the waitroom. Plays the card for real so the debut comes from an actual
/// play, and empties the hand first so the outcomes below are exact.
fn play_watanabe_looking_at(game: &mut TestGame, top_cards: &[i16]) {
    let you = game.id("PL!S-bp6-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for cid in top_cards {
        game.state.player1.main_deck.cards.push(*cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(5);
    game.add_to_hand(you);
    game.play_to_stage(you, MemberArea::Center);
}

/// The offer 渡辺 曜 must present, reduced to the fields her text pins: the two
/// looked-at cards, at most 1 take, and only cards carrying all three required
/// hearts selectable.
fn assert_looked_at_offer(game: &TestGame, expected_filtered: &[usize]) {
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            filtered_indices,
            target_player_id,
            ..
        } => {
            assert_eq!(zone, "looked_at", "the look must select from the revealed cards");
            assert_eq!(*count, 1, "…1枚公開して手札に加えてもよい");
            assert!(*allow_skip, "手札に加えて「もよい」 → declining must be legal");
            assert_eq!(
                filtered_indices.as_deref(),
                Some(expected_filtered),
                "only members with heart02+heart04+heart05 may be selectable"
            );
            assert_eq!(
                target_player_id, &None,
                "the look is the active player's own deck"
            );
        }
        other => panic!("expected looked-at SelectCard, got {other:?}"),
    }
}

#[test]
fn all_three_required_hearts_member_is_added_to_hand() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // heart02, heart04, heart05 — all three
    let filler = g.id("PL!-sd1-010-SD"); // heart01, heart03 — no match

    play_watanabe_looking_at(&mut g, &[qualifying, filler]);
    assert_looked_at_offer(&g, &[0]);

    g.select_indices(&[0]);

    assert!(!g.has_pending_choice());
    assert_eq!(
        g.state.player1.hand.cards.as_slice(),
        &[qualifying],
        "the all-three-hearts member is the only card that may enter the hand"
    );
    assert_eq!(
        g.state.player1.waitroom.cards.as_slice(),
        &[filler],
        "残りを控え室に置く — the non-matching looked-at card goes to the waitroom"
    );
    assert_eq!(
        g.state.player1.main_deck.cards.len(),
        38,
        "exactly the two looked-at cards leave the deck"
    );
}

#[test]
fn two_matching_copies_select_exact_physical_instance() {
    use rabuka_engine::ability::types::Choice;
    use rabuka_engine::core::types::AbilityTrigger;

    let db = load_real_database();
    let mut g = TestGame::new(db);
    let source = g.id("PL!S-bp6-005-R");
    let matching_a = g.id("PL!S-sd1-001-SD");
    let matching_b = g.new_id("PL!S-sd1-001-SD");
    let filler = g.id("PL!-sd1-010-SD");
    assert_ne!(matching_a, matching_b);

    g.state.player1.stage.stage = [-1, source, -1];
    g.give_energy(5);
    g.state.player1.main_deck.cards.clear();
    g.state.player1.main_deck.cards.push(matching_a);
    g.state.player1.main_deck.cards.push(matching_b);
    while g.state.player1.main_deck.cards.len() < 40 {
        g.state.player1.main_deck.cards.push(filler);
    }

    let card = g.db.get_card(source).unwrap();
    let ability = card
        .resolved_abilities()
        .find(|ability| ability.triggers.as_deref() == Some("登場"))
        .unwrap();
    let pid = g.state.player1.id.clone();
    g.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ability.full_text),
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(source),
        None,
        None,
    );
    g.state.activating_card = Some(source);
    g.state.process_pending_auto_abilities(&pid);

    match g.get_pending_choice() {
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            filtered_indices,
            target_player_id,
            ..
        } => {
            assert_eq!(zone, "looked_at");
            assert_eq!(*count, 1);
            assert!(*allow_skip);
            assert_eq!(filtered_indices.as_deref(), Some(&[0, 1][..]));
            assert_eq!(target_player_id, &None);
        }
        other => panic!("expected looked-at selection, got {other:?}"),
    }

    g.select_indices(&[1]);

    assert!(!g.has_pending_choice());
    assert!(g.state.player1.hand.cards.contains(&matching_b));
    assert!(!g.state.player1.hand.cards.contains(&matching_a));
    assert!(g.state.player1.waitroom.cards.contains(&matching_a));
    assert!(!g.state.player1.waitroom.cards.contains(&matching_b));
    assert!(!g.state.player2.hand.cards.contains(&matching_a));
    assert!(!g.state.player2.hand.cards.contains(&matching_b));
    assert_eq!(g.state.player1.main_deck.cards.len(), 38);
}

#[test]
fn two_of_three_required_hearts_member_is_discarded() {
    // A member with only heart02+heart04 is not selectable; mixing in a fully
    // qualifying member keeps the prompt offered, so this pins the FILTER
    // rather than the no-match auto-skip path.
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // heart02+04+05 — matches
    let two_hearts = g.id("PL!S-PR-015-PR"); // heart02+04 only — missing heart05

    play_watanabe_looking_at(&mut g, &[qualifying, two_hearts]);
    assert_looked_at_offer(&g, &[0]);

    g.select_indices(&[0]);

    assert!(!g.has_pending_choice());
    assert_eq!(
        g.state.player1.hand.cards.as_slice(),
        &[qualifying],
        "the qualifying member enters the hand"
    );
    assert_eq!(
        g.state.player1.waitroom.cards.as_slice(),
        &[two_hearts],
        "残りを控え室に置く — the two-hearts card was never selectable"
    );
    assert_eq!(g.state.player1.main_deck.cards.len(), 38);
}

#[test]
fn one_of_three_required_hearts_member_is_discarded() {
    // heart02+heart06 only: missing heart04 and heart05, so unselectable.
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // all 3 hearts
    let blade_card = g.id("PL!SP-sd1-001-SD"); // heart02, heart06 only

    play_watanabe_looking_at(&mut g, &[qualifying, blade_card]);
    assert_looked_at_offer(&g, &[0]);

    g.select_indices(&[0]);

    assert!(!g.has_pending_choice());
    assert_eq!(
        g.state.player1.hand.cards.as_slice(),
        &[qualifying],
        "only the qualifying member enters the hand"
    );
    assert_eq!(
        g.state.player1.waitroom.cards.as_slice(),
        &[blade_card],
        "残りを控え室に置く — the one-heart card was never selectable"
    );
    assert_eq!(g.state.player1.main_deck.cards.len(), 38);
}

#[test]
fn no_all_three_heart_match_offers_no_prompt_and_sends_both_looked_at_members_to_waitroom() {
    // With nothing matching, 渡辺 曜 has no card to offer: no choice is raised
    // and both looked-at cards are discarded. partial = heart02+heart05,
    // other = heart04+heart05, so neither has all three.
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let partial = g.id("PL!S-PR-017-PR"); // heart02, heart05 — missing heart04
    let other = g.id("PL!S-bp2-015-PR"); // heart04, heart05 — missing heart02

    play_watanabe_looking_at(&mut g, &[partial, other]);

    assert!(
        !g.has_pending_choice(),
        "an unmatchable look must not ask the player to choose"
    );
    assert!(
        g.state.player1.hand.cards.is_empty(),
        "no looked-at card qualifies, so the hand stays empty"
    );
    assert_eq!(
        g.state.player1.waitroom.cards.as_slice(),
        &[partial, other],
        "残りを控え室に置く in looked-at order"
    );
    assert_eq!(g.state.player1.main_deck.cards.len(), 38);
}

#[test]
fn optional_discard_any_of_three_heart_search_accepts_one_matching_color() {
    // Verify that bp2-005 (か = OR) still works with ANY match
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let you = g.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = g.id("PL!-sd1-010-SD");
    let blade_card = g.id("PL!SP-sd1-001-SD"); // has heart02, heart06
    g.state.player1.hand.cards.push(you);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.main_deck.cards.extend(vec![
        blade_card, filler, filler, filler, filler, filler, filler,
    ]);
    while g.state.player1.main_deck.cards.len() < 40 {
        g.state.player1.main_deck.cards.push(filler);
    }
    g.give_energy(13);
    g.state.player1.stage.stage[0] = -1;
    g.play_to_stage(you, rabuka_engine::zones::MemberArea::LeftSide);
    // bp2-005 has optional cost (discard from hand) — must be offered
    assert!(
        g.has_pending_choice(),
        "bp2-005 must offer optional cost (hand discard)"
    );
    g.select_indices(&[0]);
    // Must then offer looked_at selection
    assert!(
        g.has_pending_choice(),
        "bp2-005 must offer looked_at selection after cost"
    );
    g.select_indices(&[0]);
    decline_pending_choices_with_limit(&mut g, 30);
    // With OR semantics, heart02 alone should match
    assert!(
        g.state.player1.hand.cards.contains(&blade_card),
        "bp2-005 OR semantics: card with only heart02 should be selectable"
    );
}

#[test]
fn declining_the_look_discards_both_looked_at_cards_and_leaves_the_hand_empty() {
    // 手札に加えて「もよい」: taking is optional, and 残りを控え室に置く is not.
    // Declining must therefore still cost the two looked-at cards.
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // has all 3 hearts
    let filler = g.id("PL!-sd1-010-SD"); // heart01, heart03 — no match

    play_watanabe_looking_at(&mut g, &[qualifying, filler]);
    assert_looked_at_offer(&g, &[0]);

    g.select_indices(&[]);

    assert!(!g.has_pending_choice());
    assert!(
        g.state.player1.hand.cards.is_empty(),
        "declining the optional take must put nothing in the hand"
    );
    assert_eq!(
        g.state.player1.waitroom.cards.as_slice(),
        &[qualifying, filler],
        "残りを控え室に置く applies even when the take is declined"
    );
    assert_eq!(g.state.player1.main_deck.cards.len(), 38);
}
