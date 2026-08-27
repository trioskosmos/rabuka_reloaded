/// BP07 CLEAN-G5: character-name condition (「X」か「Y」の場合) reduced to
/// `custom`/always-true + follow-up targeting arbitrary discard instead of the
/// specific placed card.
///
/// Card: `PL!S-bp7-008-R` 小原鞠莉 ab#1 (ライブ開始時):
///   自分のデッキの一番下のカードを控え室に置いてもよい。それが「松浦果南」か
///   「黒澤ダイヤ」の場合、それを手札に加える。
///
/// The defect was: (a) "それが「松浦果南」か「黒澤ダイヤ」の場合" became
/// `custom` → AlwaysTrue (so the add-to-hand ALWAYS ran), and (b) the follow-up
/// was `move_cards{source:"discard", count:1}` which grabbed ANY discard card,
/// not the specific bottom card just placed. Fixed to emit
/// `condition{characters, source:"preceding_moved"}` and
/// `move_cards{source:"preceding_moved"}`.
///
/// Edge cases covered:
///   1. bottom card IS 松浦果南 → discarded, then added to hand
///   2. bottom card IS 黒澤ダイヤ → discarded, then added to hand
///   3. bottom card is 平安名すみれ (neither) → discarded, STAYS in waitroom
///   4. player skips the optional discard → nothing discarded, nothing added
///   5. discard contains ANOTHER 果南/ダイヤ but the placed bottom card is NOT
///      them → the OTHER card must NOT be grabbed (preceding_moved targets the
///      specific placed card, not any discard card)
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Seed P1's deck so the bottom-most card is `bottom_card`. Returns that id.
/// The top marker is a 津島善子 (never touched by deck-bottom reads).
fn seed_deck_with_bottom(game: &mut TestGame, bottom_card: i16) -> i16 {
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler); // top
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.push(bottom_card); // last = bottom
    bottom_card
}

fn setup_mari(game: &mut TestGame, bottom_card: i16) -> i16 {
    let mari = game.id("PL!S-bp7-008-R");
    game.state.player1.stage.stage = [-1, mari, -1];

    game.give_energy(3);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(game);
    // Seed AFTER the live-card-set draws so the bottom marker stays at the end.
    let bottom = seed_deck_with_bottom(game, bottom_card);
    game.set_live_card(live);
    advance_to_live_start(game);
    bottom
}

/// Same as `setup_mari` but ALSO pre-places `extra_waitroom` into the waitroom
/// BEFORE the live-start trigger resolves (so it is present when the follow-up
/// runs, and cannot be shuffled away by an earlier deck refresh). Returns the
/// bottom card id.
fn setup_mari_with_waitroom(game: &mut TestGame, bottom_card: i16, extra_waitroom: i16) -> i16 {
    let mari = game.id("PL!S-bp7-008-R");
    game.state.player1.stage.stage = [-1, mari, -1];

    game.give_energy(3);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(game);
    // Seed AFTER the live-card-set draws so the bottom marker stays at the end.
    let bottom = seed_deck_with_bottom(game, bottom_card);
    // Pre-place the extra waitroom card after the deck draws so it is NOT
    // refreshed/shuffled into the deck.
    game.state.player1.waitroom.cards.push(extra_waitroom);
    game.set_live_card(live);
    advance_to_live_start(game);
    bottom
}

/// Accept the optional discard (["No","Yes"] → option 1) and resolve the
/// follow-up choice if one appears.
fn accept_optional_discard(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        game.select_option(1);
        guard += 1;
    }
}

// ====================================================================
// Positive: bottom card IS one of the named characters → added to hand
// ====================================================================

#[test]
fn mari_bottom_kanan_added_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanan = game.id("PL!S-bp7-003-R＋"); // 松浦果南
    let bottom = setup_mari(&mut game, kanan);
    accept_optional_discard(&mut game);

    // Discarded (left the deck).
    let deck: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();
    assert!(!deck.contains(&bottom), "果南 should leave the deck");
    // Since it's 松浦果南, it must now be in hand.
    assert!(
        game.state.player1.hand.cards.contains(&bottom),
        "果南 should be added to hand"
    );
    // Not left in the waitroom.
    assert!(
        !game.state.player1.waitroom.cards.contains(&bottom),
        "果南 should not remain in waitroom"
    );
}

#[test]
fn mari_bottom_dia_added_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let dia = game.id("PL!S-bp7-004-R"); // 黒澤ダイヤ
    let bottom = setup_mari(&mut game, dia);
    accept_optional_discard(&mut game);

    let deck: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();
    assert!(!deck.contains(&bottom), "ダイヤ should leave the deck");
    assert!(
        game.state.player1.hand.cards.contains(&bottom),
        "ダイヤ should be added to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&bottom),
        "ダイヤ should not remain in waitroom"
    );
}

// ====================================================================
// Negative: bottom card is NOT one of the named characters
// ====================================================================

#[test]
fn mari_bottom_suimire_stays_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let suimire = game.id("PL!SP-bp7-004-R"); // 平安名すみれ (neither 果南 nor ダイヤ)
    let bottom = setup_mari(&mut game, suimire);
    accept_optional_discard(&mut game);

    // Discarded (left the deck).
    let deck: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();
    assert!(!deck.contains(&bottom), "すみれ should leave the deck");
    // NOT one of the named characters → stays in the waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&bottom),
        "すみれ should remain in the waitroom (condition must be false)"
    );
    // NOT added to hand.
    assert!(
        !game.state.player1.hand.cards.contains(&bottom),
        "すみれ must NOT be added to hand"
    );
}

// ====================================================================
// Skip: player declines the optional discard
// ====================================================================

#[test]
fn mari_skip_optional_discard_moves_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanan = game.id("PL!S-bp7-003-R＋"); // 松浦果南 (would match if discarded)
    let bottom = setup_mari(&mut game, kanan);

    // Decline the optional discard: ["No","Yes"] → option 0.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        game.select_option(0);
        guard += 1;
    }

    // Nothing discarded, nothing added to hand.
    let deck: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();
    assert!(
        deck.contains(&bottom),
        "果南 should remain in the deck when skipped"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&bottom),
        "no discard when optional is declined"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&bottom),
        "no add-to-hand when optional is declined"
    );
}

// ====================================================================
// Edge: discard has a DIFFERENT matching card, placed card is not one
// ====================================================================

/// The follow-up must target the SPECIFIC card just placed (preceding_moved),
/// not any 果南/ダイヤ in the discard. Here the bottom (placed) card is すみれ
/// (non-matching) but the discard ALREADY contains a 松浦果南. If the old buggy
/// `source:"discard"` behavior were active, the pre-existing 果南 would be
/// grabbed to hand even though the placed card is not 果南/ダイヤ.
#[test]
fn mari_does_not_grab_other_discard_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let suimire = game.id("PL!SP-bp7-004-R"); // 平安名すみれ (placed, non-matching)
    let other_kanan = game.id("PL!S-bp3-003-R＋"); // 松浦果南 already in discard

    let bottom = setup_mari_with_waitroom(&mut game, suimire, other_kanan);
    accept_optional_discard(&mut game);

    // The placed すみれ stays in the waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&bottom),
        "すみれ should stay in the waitroom"
    );
    // The OTHER 果南 in the discard must NOT be grabbed (preceding_moved targets
    // only the just-placed card).
    assert!(
        !game.state.player1.hand.cards.contains(&other_kanan),
        "the pre-existing discard 果南 must NOT be grabbed"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&other_kanan),
        "the pre-existing discard 果南 stays in the waitroom"
    );
    // Neither reached the hand.
    assert!(
        !game.state.player1.hand.cards.contains(&bottom),
        "すみれ must not reach the hand"
    );
}
