use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

/// Helper: fill both decks with the given filler card (30 copies each).
fn fill_deck(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Helper: trigger a live_start ability on a card and return so choices
/// can be handled manually (unlike the auto-resolve `trigger()` helper).
fn trigger_live_start(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .cloned()
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    // Drain any SelectAutoAbility prompts before the actual effect choice
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// 1 member → look at 1 card, choose to keep it (put on deck top)
// ─────────────────────────────────────────────────────────────────────
#[test]
fn one_member_keep_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let niji_member = game.id("PL!N-bp1-001-R"); // 虹ヶ咲 member cost 9

    game.state.player1.stage.stage = [-1, niji_member, -1];
    game.add_to_hand(tsunagaru);
    // Fill deck with a known card
    let deck_filler = game.new_id("PL!-sd1-010-SD");
    fill_deck(&mut game, deck_filler);

    // Trigger the live_start ability
    trigger_live_start(&mut game, tsunagaru);

    // Should have a SelectCard choice to pick from looked_at cards (1 card)
    assert!(
        game.has_pending_choice(),
        "Should have pending choice for look_and_select"
    );
    game.assert_select_card("looked_at", 1, true);

    // Keep the card: select the looked-at card (index 0)
    game.select_indices(&[0]);

    // After choice: top card of deck should be deck_filler (the looked-at card was placed on top)
    assert!(
        game.state
            .player1
            .main_deck
            .cards
            .first()
            .is_some_and(|&c| c == deck_filler),
        "Deck top should be the kept card"
    );
    // No cards should be in waitroom (none discarded - we kept the 1 card)
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "Waitroom should be empty (card was kept)"
    );

    // The reveal ran: top card was revealed (deck_filler, not a live)
    // No score bonus since it's not a live card
    // The live card's score should be unchanged (score=1)
    let card = game.db.get_card(tsunagaru).unwrap();
    assert_eq!(card.score, Some(1), "Base score should be 1 (not boosted)");
}

// ─────────────────────────────────────────────────────────────────────
// 1 member → look at 1 card, discard it (select skip)
// ─────────────────────────────────────────────────────────────────────
#[test]
fn one_member_discard_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let niji_member = game.id("PL!N-bp1-001-R");

    game.state.player1.stage.stage = [-1, niji_member, -1];
    game.add_to_hand(tsunagaru);
    let deck_filler = game.new_id("PL!-sd1-010-SD");
    fill_deck(&mut game, deck_filler);

    trigger_live_start(&mut game, tsunagaru);

    assert!(game.has_pending_choice());
    // Skip: empty selection means "don't keep any"
    game.select_indices(&[]);

    // The looked-at card goes to waitroom (discard_remaining=true)
    assert!(
        game.state.player1.waitroom.cards.contains(&deck_filler),
        "Looked-at card should be in waitroom (skipped)"
    );
    // Deck top should now be the next card in deck (which is also deck_filler)
    assert!(
        game.state
            .player1
            .main_deck
            .cards
            .first()
            .is_some_and(|&c| c == deck_filler),
        "Deck top should be a deck_filler"
    );
}

// ─────────────────────────────────────────────────────────────────────
// 3 members → look at 3 cards, keep 1 on top, rest to waitroom
// ─────────────────────────────────────────────────────────────────────
#[test]
fn three_members_keep_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let niji_member = game.id("PL!N-bp1-001-R");

    game.state.player1.stage.stage = [niji_member, niji_member, niji_member];
    game.add_to_hand(tsunagaru);
    let deck_filler = game.new_id("PL!-sd1-010-SD");
    fill_deck(&mut game, deck_filler);

    trigger_live_start(&mut game, tsunagaru);

    assert!(game.has_pending_choice());
    // Keep the first looked-at card (index 0)
    game.select_indices(&[0]);

    // The kept card goes on top of deck
    assert!(
        game.state
            .player1
            .main_deck
            .cards
            .first()
            .is_some_and(|&c| c == deck_filler),
        "Deck top should be the kept card"
    );
    // The other 2 looked-at cards go to waitroom
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        2,
        "2 cards should be in waitroom (discarded)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// 3 members → look at 3 cards, skip → all 3 go to waitroom
// ─────────────────────────────────────────────────────────────────────
#[test]
fn three_members_skip_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let niji_member = game.id("PL!N-bp1-001-R");

    game.state.player1.stage.stage = [niji_member, niji_member, niji_member];
    game.add_to_hand(tsunagaru);
    let deck_filler = game.new_id("PL!-sd1-010-SD");
    fill_deck(&mut game, deck_filler);

    trigger_live_start(&mut game, tsunagaru);

    assert!(game.has_pending_choice());
    // Skip: empty selection
    game.select_indices(&[]);

    // All 3 looked-at cards go to waitroom
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        3,
        "3 cards should be in waitroom (all discarded)"
    );
    // The next deck card should now be on top
    assert!(
        game.state
            .player1
            .main_deck
            .cards
            .first()
            .is_some_and(|&c| c == deck_filler),
        "Deck top should be a deck_filler (next card)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Reveal a LIVE card → score+1
// ─────────────────────────────────────────────────────────────────────
#[test]
fn reveal_live_card_gets_score_boost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let niji_member = game.id("PL!N-bp1-001-R");
    let live_card = game.id("PL!-bp3-019-L"); // μ's live card (to be revealed)

    game.state.player1.stage.stage = [-1, niji_member, -1];
    game.add_to_hand(tsunagaru);

    // Set up deck: first card = live_card (will be looked at), second = something else
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(live_card); // top (will be looked at & revealed)
    for _ in 0..29 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }

    trigger_live_start(&mut game, tsunagaru);

    assert!(game.has_pending_choice());
    // Select the looked-at card (the live_card) and put it back on deck top
    game.select_indices(&[0]);

    // After choice: live_card is back on deck top
    assert!(
        game.state.player1.main_deck.cards.first() == Some(&live_card),
        "Deck top should be the live card"
    );

    // The reveal step ran: it revealed the live_card → score+1
    // Check the score modifier on the activating card
    let score_mod = game.state.mods.get_score_modifier(tsunagaru);
    assert_eq!(
        score_mod, 1,
        "Tsunagaru Connect should get +1 score (revealed a live card)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Reveal a NON-live card → NO score bonus
// ─────────────────────────────────────────────────────────────────────
#[test]
fn reveal_non_live_card_no_boost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let niji_member = game.id("PL!N-bp1-001-R");
    let member_card = game.id("PL!-sd1-010-SD"); // a member card (not live)

    game.state.player1.stage.stage = [-1, niji_member, -1];
    game.add_to_hand(tsunagaru);

    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(member_card); // top (looked at & revealed)
    for _ in 0..29 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }

    trigger_live_start(&mut game, tsunagaru);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    // member_card is not a live card → no score bonus
    let score_mod = game.state.mods.get_score_modifier(tsunagaru);
    assert_eq!(
        score_mod, 0,
        "No score bonus when revealing a non-live card"
    );
}

// ─────────────────────────────────────────────────────────────────────
// No 虹ヶ咲 members on stage → no peek at all
// ─────────────────────────────────────────────────────────────────────
#[test]
fn no_niji_members_no_peek() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let non_niji = game.id("PL!-sd1-010-SD"); // not 虹ヶ咲

    game.state.player1.stage.stage = [-1, non_niji, -1]; // member, but not 虹ヶ咲
    game.add_to_hand(tsunagaru);
    let filler_id = game.new_id("PL!-sd1-010-SD");
    fill_deck(&mut game, filler_id);

    trigger_live_start(&mut game, tsunagaru);

    // No pending choice — no lookup happened (0 members)
    assert!(
        !game.has_pending_choice(),
        "No pending choice expected (no 虹ヶ咲 members)"
    );

    // Nothing was looked at, so nothing moved. Deck unchanged.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        30,
        "Deck should be unchanged"
    );
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "Waitroom should be empty"
    );
}

// ─────────────────────────────────────────────────────────────────────
// All edge cases at once: 3 members, keep 0, reveal live card → score+1
// ─────────────────────────────────────────────────────────────────────
#[test]
fn three_members_skip_reveal_live_gets_boost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsunagaru = game.id("PL!N-bp3-028-L");
    let niji_member = game.id("PL!N-bp1-001-R");
    let live_card = game.id("PL!-bp3-019-L");

    game.state.player1.stage.stage = [niji_member, niji_member, niji_member];
    game.add_to_hand(tsunagaru);

    // Deck: 3 look_at cards + live_card on top (position 3 after look_at)
    // But wait: look_at draws from TOP. So we need:
    // top3: filler, filler, filler → looked_at & discarded
    // next: live_card → revealed
    let filler = game.new_id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler); // top → looked at (index 0)
    game.state.player1.main_deck.cards.push(filler); // → looked at (index 1)
    game.state.player1.main_deck.cards.push(filler); // → looked at (index 2)
    game.state.player1.main_deck.cards.push(live_card); // → revealed (after keep/skip)
    for _ in 0..26 {
        game.state.player1.main_deck.cards.push(filler);
    }

    trigger_live_start(&mut game, tsunagaru);

    assert!(game.has_pending_choice());
    // Skip all (don't keep any of the 3 looked-at cards)
    game.select_indices(&[]);

    // 3 looked-at cards should be in waitroom
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        3,
        "3 cards should be in waitroom"
    );

    // The reveal revealed live_card (now on top of deck)
    assert!(
        game.state.player1.main_deck.cards.first() == Some(&live_card),
        "live_card should now be on deck top"
    );

    // Score+1 should be applied (revealed a live card)
    let score_mod = game.state.mods.get_score_modifier(tsunagaru);
    assert_eq!(score_mod, 1, "Score should be boosted (revealed live card)");
}
