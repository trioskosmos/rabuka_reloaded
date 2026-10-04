/// You (渡辺 曜 PL!S-bp2-005-R+) — Debut look + select from deck
///
/// Ab#0 (登場): Optional discard from hand: look at top 7, select up to 3 heart02/04/05
///   member cards, add to hand, discard rest.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn resolve_all_up_to(game: &mut TestGame, max: usize) {
    for _ in 0..max {
        if !game.has_pending_choice() {
            return;
        }
        game.select_indices(&[]);
    }
    panic!("resolve_all_up_to: exceeded {} iters", max);
}

#[test]
fn you_q124_blade_heart_excluded_base_heart_included() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, blade_only, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered (filler in hand)");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        !game.state.player1.hand.cards.contains(&blade_only),
        "Blade-only NOT in hand"
    );
}

#[test]
fn you_ability_ends_and_discard_only_grows_at_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        filler, filler, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let initial_discard = game.state.player1.waitroom.cards.len();
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert_eq!(
        game.state.player1.waitroom.cards.len() - initial_discard,
        8,
        "Expected 8 in discard (1 cost + 7 looked-at)"
    );
}

#[test]
fn you_ability_select_1_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, filler, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(
        game.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card in hand"
    );
}

/// OPEN GAP — 「残りを控え室に置く」 leaves the remainder in the looked-at buffer.
///
/// `discard_remaining: true` is in the parse (`abilities.json:4569`).
///
/// # What is established
///
/// `handle_select_cards_looked_at` (`move/mod.rs`) takes the WHOLE remainder out of
/// `gs.looked_at_cards` with `core::mem::take` at :3118, into a local. That buffer is
/// emptied unconditionally.
///
/// The remainder is then placed in only two places we can see:
///
///   * the `needs_order` branch (:3127-3137), which requires a deck destination,
///     `placement_order: AnyOrder`, and `selected_cards.len() > 1` — none of which hold
///     here; and
///   * the re-prompt at :3210-3215, which puts the remainder BACK into
///     `gs.looked_at_cards` and offers it again, returning without discarding.
///
/// When the player then DECLINES that re-offer, `handle_looked_at_selection` is entered
/// again with `valid` empty, so the `if is_select_cards && !valid.is_empty()` guard at
/// `choice.rs:2004` is false, the block that would re-offer is skipped, and control
/// reaches `finalize_choice` — with the remainder still sitting in `gs.looked_at_cards`
/// and no code path left to discard it. That is the shape of the reported behaviour:
/// one card in hand, six stranded.
///
/// # What is NOT established, and was wrong before
///
/// An earlier version of this comment claimed the declining path reaches
/// `handle_revealed_cards_selection`, which discards from `gs.revealed_cards`
/// (`choice.rs:1758-1772`), and called that a buffer mismatch. **It does not.** The
/// zone dispatch is mutually exclusive — `Zone::LookedAt` goes to
/// `handle_looked_at_selection` and `Zone::RevealedCards` to
/// `handle_revealed_cards_selection` (`choice.rs:1114-1124`) — so the latter is never
/// reached from a looked-at selection. Also recorded here because it is tempting:
/// passing `ctx.discard_remaining` through at `choice.rs:1997` is provably a NO-OP, since
/// `move/mod.rs:3030-3034` already reads `sa.discard_remaining_any()` first. Both were
/// tried or reasoned and neither is the cause.
///
/// The precise line that drops the remainder on the declined-re-offer path is not
/// identified yet; the account above is the narrowest reading consistent with the
/// observed state and the code read so far, not a confirmed trace.
///
/// # Observed, and what the file never asserted
///
/// One card reaches hand, the waitroom grows by 1 (the optional cost only), and
/// `gs.looked_at_cards` holds the six cards the `card_type` + `heart_colors` filter
/// excluded. They are in no zone, yet still readable as shared state by the next
/// look-based card and by the opponent.
///
/// 「残り」 means the rest of the SEVEN.
///
/// 米女 `PL!SP-bp5-007-AR` mills correctly with the same `count`, `max`, `optional`,
/// `reveal` and `discard_remaining` — so none of those is sufficient to cause it.
///
/// Current behaviour recorded, not asserted as correct, per the `open_gap_*` convention.
/// INVERT every assertion when the clause is fixed.
///
/// Nothing could see this before. The only waitroom assertion in this file was in
/// `you_ability_ends_and_discard_only_grows_at_end`, which takes NOTHING — and taking
/// none IS everything, so that case cannot distinguish 「残り」 from "discard the lot".
/// Every test that actually took cards asserted only the hand.
#[test]
fn open_gap_you_taking_one_of_seven_leaves_the_rest_stranded_in_looked_at() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Should have look_and_select choice");
    game.select_indices(&[0]); // take exactly ONE
    resolve_all_up_to(&mut game, 30);

    // The take itself works.
    assert_eq!(
        game.state.player1.hand.cards.iter().filter(|&&c| c == qualifying).count(),
        1,
        "the one qualifying card was taken to hand"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 7,
        "the look window is exactly 7, so a sentinel card below it is untouched"
    );

    // OPEN GAP: the 6 cards the filter excluded are stranded. Expected, once fixed: the
    // waitroom grows by 7 (1 optional cost + 「残り」 x6) and `looked_at_cards` is empty.
    assert_eq!(
        game.state.player1.waitroom.cards.len() - waitroom_before,
        1,
        "OPEN GAP: only the cost card was discarded. Expected 1 + 「残りを控え室に置く」 \
         x6 = 7 — the six fillers are excluded by card_type + heart_colors, so the \
         cleanup never sees them. INVERT THIS when the clause covers the whole look."
    );
    assert_eq!(
        game.state.looked_at_cards.len(),
        6,
        "OPEN GAP: the 6 「残り」 are stranded in the looked-at buffer — in no zone, \
         yet still readable as shared state by the next look-based card and by the \
         opponent. INVERT THIS when they are discarded."
    );
}

#[test]
fn you_ability_select_multiple_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q1 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q2 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q3 must be offered");
    game.select_indices(&[0]);
    // Observed: picking the 3rd of 3 eligible cards reaches the max cap and
    // the ability auto-finalizes — no skip prompt is offered.
    assert!(
        !game.has_pending_choice(),
        "reaching max (3) picks must auto-finalize without prompting"
    );
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
    assert!(game.state.player1.hand.cards.contains(&q3), "Q3 in hand");
}

#[test]
fn you_ability_user_scenario_select_one_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "look_and_select must be offered");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(
        !game.state.player1.hand.cards.contains(&q2),
        "Q2 NOT in hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&q3),
        "Q3 NOT in hand"
    );
}

#[test]
fn you_q124_two_plays_both_reject_blade_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![filler, filler, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(
        !game.state.player1.hand.cards.contains(&blade_only),
        "Blade-only NOT in hand"
    );
}

#[test]
fn you_select_2_then_ends() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q1 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q2 must be offered");
    game.select_indices(&[0]);
    // Observed: after the last eligible card is picked (2 < max 3) the
    // ability auto-finalizes when no eligible cards remain — no skip prompt.
    assert!(
        !game.has_pending_choice(),
        "exhausting eligible cards must auto-finalize without prompting"
    );
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
}

#[test]
fn you_select_3_then_ends() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "cost choice must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q1 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q2 must be offered");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "select q3 must be offered");
    game.select_indices(&[0]);
    // Observed: picking the 3rd of 3 eligible cards reaches the max cap and
    // the ability auto-finalizes — no skip prompt is offered.
    assert!(
        !game.has_pending_choice(),
        "reaching max (3) picks must auto-finalize without prompting"
    );
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
    assert!(game.state.player1.hand.cards.contains(&q3), "Q3 in hand");
}
