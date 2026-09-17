use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;

fn setup_all_three_heart_search_and_resolve_debut(game: &mut TestGame, top_cards: Vec<i16>) -> bool {
    let you = game.id("PL!S-bp6-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    // Place card on stage directly, then fill deck
    game.state.player1.stage.stage = [-1, you, -1];
    game.state.player1.main_deck.cards.clear();
    for cid in top_cards {
        game.state.player1.main_deck.cards.push(cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(5);
    // trigger() drains all choices; return whether the look_and_select was offered.
    // When 0 of the top cards match the filter, the engine auto-discards without
    // prompting (look.rs matching_count==0 early return) — callers decide if that's legal.
    trigger_printed_ability_and_resolve_choices(game, you, "登場")
}

#[test]
fn all_three_required_hearts_member_is_added_to_hand() {
    // Cards with ALL three hearts (heart02, heart04, heart05) should be selectable
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // has heart02, heart04, heart05
    let filler = g.id("PL!-sd1-010-SD"); // has heart01, heart03 — no match

    setup_all_three_heart_search_and_resolve_debut(&mut g, vec![qualifying, filler]);
    decline_pending_choices_with_limit(&mut g, 20);
    assert!(
        g.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card (all 3 hearts) should be in hand"
    );
}

#[test]
fn two_of_three_required_hearts_member_is_discarded() {
    // Cards with only 2 of the 3 required hearts should NOT be selectable.
    // Mix a qualifying card in so the prompt IS offered, then verify only the
    // qualifying one is selectable — this tests the filter, not the auto-skip path.
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // heart02+04+05 — matches
    let two_hearts = g.id("PL!S-PR-015-PR"); // heart02+04 only — missing heart05
    let filler = g.id("PL!-sd1-010-SD");

    let offered = setup_all_three_heart_search_and_resolve_debut(&mut g, vec![qualifying, two_hearts, filler]);
    assert!(offered, "qualifying present → look_and_select must be offered");
    decline_pending_choices_with_limit(&mut g, 20);
    assert!(
        g.state.player1.hand.cards.contains(&qualifying),
        "qualifying (all 3 hearts) should be selectable"
    );
    assert!(
        !g.state.player1.hand.cards.contains(&two_hearts),
        "Card with only heart02+heart04 should NOT be selectable (missing heart05)"
    );
    assert!(
        g.state.player1.waitroom.cards.contains(&two_hearts),
        "Rejected two-hearts card must be discarded to waitroom (残りを控え室に置く)"
    );
}

#[test]
fn one_of_three_required_hearts_member_is_discarded() {
    // Card with only heart02 (not heart04, heart05) should be rejected.
    // Mix a qualifying card so prompt is offered; verify filter rejects 1-heart card.
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // all 3 hearts
    let blade_card = g.id("PL!SP-sd1-001-SD"); // has heart02, heart06 only
    let filler = g.id("PL!-sd1-010-SD");

    let offered = setup_all_three_heart_search_and_resolve_debut(&mut g, vec![qualifying, blade_card, filler]);
    assert!(offered, "qualifying present → look_and_select must be offered");
    decline_pending_choices_with_limit(&mut g, 20);
    assert!(
        g.state.player1.hand.cards.contains(&qualifying),
        "qualifying should be selectable"
    );
    assert!(
        !g.state.player1.hand.cards.contains(&blade_card),
        "Card with only heart02+heart06 should NOT be selectable (missing heart04, heart05)"
    );
    assert!(
        g.state.player1.waitroom.cards.contains(&blade_card),
        "Rejected one-heart card must be discarded to waitroom"
    );
}

#[test]
fn no_all_three_heart_match_discards_both_looked_at_members() {
    // If neither top card matches, the ability may still resolve (optional = true)
    let db = load_real_database();
    let mut g = TestGame::new(db);
    // Use a PR card with heart02+heart05 (missing heart04) as the only candidate
    let partial = g.id("PL!S-PR-017-PR"); // heart02, heart05 only — missing heart04
    let other = g.id("PL!S-bp2-015-PR"); // heart04, heart05 only — missing heart02

    setup_all_three_heart_search_and_resolve_debut(&mut g, vec![partial, other]);
    decline_pending_choices_with_limit(&mut g, 20);
    // Neither card should be in hand (none has all 3 hearts)
    assert!(
        !g.state.player1.hand.cards.contains(&partial),
        "Partial heart card should NOT be in hand"
    );
    assert!(
        !g.state.player1.hand.cards.contains(&other),
        "Other partial heart card should NOT be in hand"
    );
    // Both must have been discarded to waitroom (残りを控え室に置く)
    assert!(
        g.state.player1.waitroom.cards.contains(&partial)
            && g.state.player1.waitroom.cards.contains(&other),
        "Both non-matching looked-at cards must go to waitroom"
    );
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
fn all_three_heart_match_enters_hand_without_nonmatching_filler() {
    // Qualifying card goes to hand, non-qualifying filler goes to waitroom
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // has all 3 hearts
    let filler = g.id("PL!-sd1-010-SD"); // heart01, heart03 — no match

    setup_all_three_heart_search_and_resolve_debut(&mut g, vec![qualifying, filler]);
    decline_pending_choices_with_limit(&mut g, 20);
    assert!(
        g.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card should be in hand"
    );
    assert!(
        !g.state.player1.hand.cards.contains(&filler),
        "Non-qualifying filler should NOT be in hand"
    );
}
