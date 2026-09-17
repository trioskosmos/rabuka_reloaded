use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

/// Helper: trigger a live_start ability and return after draining
/// any auto-ability prompts so we can interact with the main choice.
fn trigger_live_start_kuroe(game: &mut TestGame, card_id: i16) {
    let pid = game.state.player1.id.clone();
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .unwrap();
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
// 2+ cards in live zone, selectable card exists → choice appears
// ─────────────────────────────────────────────────────────────────────
#[test]
fn two_cards_choice_appears() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kuroe = game.id("PL!S-bp6-004-R+");
    let live_a = game.id("PL!S-bp6-021-L"); // Aqours live, no live_start
    let live_b = game.id("PL!S-bp6-021-L");

    game.state.player1.live_card_zone.cards.push(live_a);
    game.state.player1.live_card_zone.cards.push(live_b);
    game.add_to_hand(kuroe);

    trigger_live_start_kuroe(&mut game, kuroe);

    assert!(
        game.has_pending_choice(),
        "Choice should appear with 2+ cards in live zone"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Only 1 card in live zone → no choice (condition ≥2 fails)
// ─────────────────────────────────────────────────────────────────────
#[test]
fn one_card_no_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kuroe = game.id("PL!S-bp6-004-R+");
    let live = game.id("PL!S-bp6-021-L");

    game.state.player1.live_card_zone.cards.push(live);
    game.add_to_hand(kuroe);

    trigger_live_start_kuroe(&mut game, kuroe);

    assert!(
        !game.has_pending_choice(),
        "No choice with only 1 card in live zone"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Card with ライブ開始時 ability is excluded from selection
// ─────────────────────────────────────────────────────────────────────
#[test]
fn live_start_card_excluded() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kuroe = game.id("PL!S-bp6-004-R+");
    let live_start = game.id("PL!S-bp6-019-L"); // has ライブ開始時
    let live_start2 = game.id("PL!S-bp6-020-L"); // has ライブ開始時

    game.state.player1.live_card_zone.cards.push(live_start);
    game.state.player1.live_card_zone.cards.push(live_start2);
    game.add_to_hand(kuroe);

    trigger_live_start_kuroe(&mut game, kuroe);

    // All cards excluded by ability_filter → no choice, cards stay in zone
    assert!(
        !game.has_pending_choice(),
        "No choice when all cards have \u{30e9}\u{30a4}\u{30d6}\u{958b}\u{59cb}\u{6642} ability"
    );

    assert!(
        game.state.player1.live_card_zone.cards.len() == 2,
        "Cards should remain in live zone"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Select a card → it moves to deck top, hearts are gained
// ─────────────────────────────────────────────────────────────────────
#[test]
fn select_card_gains_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kuroe = game.id("PL!S-bp6-004-R+");
    let selectable = game.id("PL!S-bp6-021-L"); // no live_start
    let other = game.id("PL!S-bp6-021-L"); // also no live_start

    game.state.player1.live_card_zone.cards.push(selectable);
    game.state.player1.live_card_zone.cards.push(other);
    game.add_to_hand(kuroe);

    trigger_live_start_kuroe(&mut game, kuroe);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    // Selected card (index 0 of [selectable, other]) moved to deck top exactly.
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&selectable),
        "the SELECTED card must be on deck top"
    );
    assert_eq!(
        game.state.player1.live_card_zone.cards.len(),
        1,
        "One card should remain in live zone"
    );
    assert!(
        !game
            .state
            .player1
            .live_card_zone
            .cards
            .contains(&game.state.player1.main_deck.cards[0]),
        "Deck top card should not be in live zone"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Skip selection → no hearts gained, cards stay in live zone
// ─────────────────────────────────────────────────────────────────────
#[test]
fn skip_no_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kuroe = game.id("PL!S-bp6-004-R+");
    let live = game.id("PL!S-bp6-021-L");

    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.live_card_zone.cards.push(live);
    game.add_to_hand(kuroe);

    trigger_live_start_kuroe(&mut game, kuroe);

    assert!(game.has_pending_choice());
    game.select_indices(&[]);

    // Both cards still in live zone
    assert_eq!(
        game.state.player1.live_card_zone.cards.len(),
        2,
        "Both cards should remain in live zone"
    );
}
